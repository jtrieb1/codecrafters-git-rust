use super::errors::CheckoutError;
use super::input::CheckoutInput;
use crate::shared::commit::Commit;
use crate::shared::hash::{hash_from_string, hash_to_string};
use crate::shared::object::{Object, ObjectType};
use crate::shared::refs::{GitRef, resolve_ref_name_to_full_ref};
use crate::shared::tag::AnnotatedTag;
use crate::shared::tree::{Tree, TreeMode};

pub fn checkout(input: CheckoutInput) -> Result<String, CheckoutError> {
    // Check if the input is a valid commit hash (40 hex characters) or a ref name (like a branch or tag)
    let commit_hash = if let Ok(hash) = resolve_ref_name_to_hash(&input.committish) {
        hash
    } else {
        hash_from_string(&input.committish)
    };

    let hash_str = hash_to_string(&commit_hash);

    // Validate the commit hash format (should be a 40-character hexadecimal string)
    if commit_hash.len() != 20 {
        return Err(CheckoutError::InvalidCommitHash(format!(
            "Commit hash must be 20 bytes (40 hex characters), got {} bytes",
            commit_hash.len()
        )));
    }

    // Try to read the commit object from the object store
    let commit_object = match Object::try_from_hash(&commit_hash) {
        Ok(obj) => obj,
        Err(_) => {
            return Err(CheckoutError::CommitNotFound(format!(
                "Commit with hash {} not found",
                hash_to_string(&commit_hash)
            )));
        }
    };

    // Parse the commit object content to extract the tree hash and other information
    let commit = match Commit::try_from(&commit_object) {
        Ok(c) => c,
        Err(e) => {
            return Err(CheckoutError::InvalidCommitContent(format!(
                "Failed to parse commit content: {}",
                e
            )));
        }
    };

    // Update HEAD to point to the new commit hash
    let Ok(head) = GitRef::current_head() else {
        return Err(CheckoutError::InvalidCommitContent(
            "Failed to read current HEAD".to_string(),
        ));
    };

    let Ok(_new_head) =
        head.update_ref(commit_hash.try_into().map_err(|_| {
            CheckoutError::InvalidCommitHash("Invalid commit hash length".to_string())
        })?)
    else {
        return Err(CheckoutError::InvalidCommitContent(
            "Failed to update HEAD with new commit hash".to_string(),
        ));
    };

    // Update the working directory to match the tree of the new commit
    let tree = match commit.try_get_tree() {
        Ok(t) => t,
        Err(e) => {
            return Err(CheckoutError::InvalidCommitContent(format!(
                "Failed to get tree from commit: {}",
                e
            )));
        }
    };

    if let Err(e) = update_working_directory(
        &tree,
        &std::env::current_dir().map_err(|e| {
            CheckoutError::IOError(format!("Failed to get current directory: {}", e))
        })?,
    ) {
        return Err(CheckoutError::InvalidCommitContent(format!(
            "Failed to update working directory: {}",
            e
        )));
    }

    Ok(format!("Checked out commit {}", hash_str))
}


fn update_working_directory(tree: &Tree, cwd: &std::path::Path) -> Result<(), String> {
    // Start by clearing the current working directory (except for the .git directory)
    for entry in
        std::fs::read_dir(&cwd).map_err(|e| format!("Failed to read current directory: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.file_name().map_or(false, |name| name == ".git") {
            continue; // Skip the .git directory
        }
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| format!("Failed to remove directory {}: {}", path.display(), e))?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove file {}: {}", path.display(), e))?;
        }
    }

    // Now we can write the files from the tree to the working directory
    for entry in tree.entries() {
        let Ok((object, mode)) = entry.to_referenced_object() else {
            return Err(format!(
                "Failed to read object for tree entry {}: {}",
                entry.name, "unknown error"
            ));
        };

        object.pretty_print();

        let object_path = cwd.join(&entry.name);
        if mode == TreeMode::Directory {
            std::fs::create_dir_all(&object_path).map_err(|e| {
                format!(
                    "Failed to create directory {}: {}",
                    object_path.display(),
                    e
                )
            })?;
            let subtree = Tree::try_from(&object).map_err(|e| {
                format!(
                    "Failed to parse tree object {}: {}",
                    hash_to_string(&entry.hash),
                    e
                )
            })?;
            update_working_directory(&subtree, &object_path)?;
        } else {
            if let Some(parent) = object_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create directory {}: {}", parent.display(), e)
                })?;
            }

            std::fs::write(&object_path, object.content())
                .map_err(|e| format!("Failed to write file {}: {}", object_path.display(), e))?;
        }
    }

    Ok(())
}

fn resolve_tag_chain_to_commit_hash(mut object: Object) -> Result<Vec<u8>, String> {
    loop {
        if object.object_type() == &ObjectType::Commit {
            return Ok(object.get_hash());
        } else if object.object_type() == &ObjectType::Tag {
            let annotated_tag = AnnotatedTag::try_from(&object)
                .map_err(|e| format!("Failed to parse tag object: {}", e))?;
            let target_object = annotated_tag
                .resolve_target()
                .map_err(|e| format!("Failed to resolve tag target: {}", e))?;
            object = target_object;
        } else {
            return Err(format!(
                "Object {} is not a commit or tag",
                hash_to_string(&object.get_hash())
            ));
        }
    }
}

fn resolve_ref_name_to_hash(ref_name: &str) -> Result<Vec<u8>, String> {
    let full_ref = resolve_ref_name_to_full_ref(ref_name)?;

    let ref_path = format!(".git/{}", full_ref);
    if std::fs::metadata(&ref_path).is_ok() {
        let obj_ref = GitRef::from_file_path(&ref_path)?;
        let object = obj_ref.resolve()?;

        // If the ref points to a tag, we need to resolve the tag to get the underlying object hash
        // Note that the tag could itself point to another tag, so we need to keep resolving until we get to a non-tag object
        return resolve_tag_chain_to_commit_hash(object);
    }

    Err(format!(
        "Ref name {} not found as a branch or tag",
        ref_name
    ))
}
