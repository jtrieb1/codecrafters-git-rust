use crate::shared::hash::{hash_from_string, hash_to_string};
use crate::shared::object::Object;

pub fn resolve_ref_name_to_full_ref(ref_name: &str) -> Result<String, String> {
    if (ref_name == "HEAD") || ref_name.starts_with("refs/") {
        Ok(ref_name.to_string())
    } else if ref_name.starts_with("heads/") {
        Ok(format!("refs/{}", ref_name))
    } else if ref_name.starts_with("tags/") {
        Ok(format!("refs/{}", ref_name))
    } else if ref_name.starts_with("remotes/") {
        Ok(format!("refs/{}", ref_name))
    } else {
        // Try to find in heads, then tags, then remotes
        let heads_ref = format!("refs/heads/{}", ref_name);
        if std::fs::metadata(&format!(".git/{}", heads_ref)).is_ok() {
            return Ok(heads_ref);
        }
        let tags_ref = format!("refs/tags/{}", ref_name);
        if std::fs::metadata(&format!(".git/{}", tags_ref)).is_ok() {
            return Ok(tags_ref);
        }
        
        // Remotes usually are nested under refs/remotes/<remote_name>/<ref_name>, so we need to check if there's any ref under refs/remotes that ends with the provided ref_name
        let remotes_dir = std::path::Path::new(".git/refs/remotes");
        if remotes_dir.exists() && remotes_dir.is_dir() {
            for entry in std::fs::read_dir(remotes_dir).map_err(|e| format!("Failed to read remotes directory: {}", e))? {
                let entry = entry.map_err(|e| format!("Failed to read remote directory entry: {}", e))?;
                let path = entry.path();
                if path.is_dir() {
                    // Ref name might be in format refs/remotes/<remote_name>/<ref_name>, or remotes/<remote_name>/<ref_name> or just <remote_name>/<ref_name>, so we check for all of those
                    let trimmed_ref_name = if ref_name.starts_with("refs/remotes/") {
                        ref_name.strip_prefix("refs/remotes/").unwrap()
                    } else if ref_name.starts_with("remotes/") {
                        ref_name.strip_prefix("remotes/").unwrap()
                    } else {
                        ref_name
                    };

                    // Recall that our current path is .git/refs/remotes, so we can just check for <remote_name>/<ref_name> under this path
                    if ref_name.contains("/") {
                        // This is in the format <remote_name>/<ref_name>, so we can check for it directly
                        // Check if the <remote_name> part matches the current directory name, and if so check for the ref
                        let parts: Vec<&str> = trimmed_ref_name.splitn(2, '/').collect();
                        if parts.len() != 2 {
                            continue; // Invalid format, skip
                        }
                        let remote_name = parts[0];
                        let ref_name = parts[1];
                        if path.file_name().map_or(false, |name| name == remote_name) {
                            let remote_ref_path = path.join(ref_name);
                            if remote_ref_path.exists() {
                                return Ok(format!("refs/remotes/{}/{}", remote_name, ref_name));
                            }
                        }
                    }

                    let remote_ref_path = path.join(ref_name);
                    if remote_ref_path.exists() {
                        return Ok(format!("refs/remotes/{}/{}", path.file_name().unwrap().to_string_lossy(), ref_name));
                    }
                }
            }
        }

        return Err(format!(
            "Ref name {} not found as a branch, tag, or remote",
            ref_name
        ));
    }
}

#[derive(Debug, Clone)]
pub struct PlainGitRef {
    pub name: String,
    pub hash: [u8; 20],
}

impl PlainGitRef {
    pub fn new(name: String, hash: [u8; 20]) -> Self {
        Self { name, hash }
    }

    pub fn persist_head(&self, force: bool) -> Result<(), String> {
        let ref_path = format!(".git/refs/heads/{}", self.name);
        if !force && std::path::Path::new(&ref_path).exists() {
            return Err(format!(
                "Ref {} already exists. Use force option to overwrite.",
                self.name
            ));
        }
        std::fs::create_dir_all(std::path::Path::new(&ref_path).parent().unwrap())
            .map_err(|e| format!("Failed to create directories for ref: {}", e))?;
        std::fs::write(&ref_path, hash_to_string(&self.hash))
            .map_err(|e| format!("Failed to write ref file: {}", e))?;
        Ok(())
    }

    pub fn persist_tag(&self, force: bool) -> Result<(), String> {
        let ref_path = format!(".git/refs/tags/{}", self.name);
        if !force && std::path::Path::new(&ref_path).exists() {
            return Err(format!(
                "Ref {} already exists. Use force option to overwrite.",
                self.name
            ));
        }
        std::fs::create_dir_all(std::path::Path::new(&ref_path).parent().unwrap())
            .map_err(|e| format!("Failed to create directories for ref: {}", e))?;
        std::fs::write(&ref_path, hash_to_string(&self.hash))
            .map_err(|e| format!("Failed to write ref file: {}", e))?;
        Ok(())
    }

    pub fn persist_remote(&self, remote_name: &str) -> Result<(), String> {
        let ref_path = format!(".git/refs/remotes/{}/{}", remote_name, self.name);
        std::fs::create_dir_all(std::path::Path::new(&ref_path).parent().unwrap())
            .map_err(|e| format!("Failed to create directories for ref: {}", e))?;
        std::fs::write(&ref_path, hash_to_string(&self.hash))
            .map_err(|e| format!("Failed to write ref file: {}", e))?;
        Ok(())
    }

    pub fn resolve_object(&self) -> Result<Object, String> {
        Object::try_from_hash(&self.hash)
            .map_err(|e| format!("Failed to read object for ref {}: {}", self.name, e))
    }
}

pub enum GitRef {
    Head(PlainGitRef),
    Tag(PlainGitRef),
    Remote {
        remote_name: String,
        ref_info: PlainGitRef,
    },
}

impl GitRef {
    pub fn current_head() -> Result<Self, String> {
        let head_path = std::path::Path::new(".git").join("HEAD");
        if let Ok(head_content) = std::fs::read_to_string(&head_path) {
            if head_content.trim().starts_with("ref: ") {
                let head_ref = head_content.trim().strip_prefix("ref: ").unwrap();
                let ref_path = std::path::Path::new(".git").join(head_ref);
                if let Ok(commit_hash) = std::fs::read_to_string(&ref_path) {
                    return Ok(GitRef::Head(PlainGitRef::new(
                        head_ref.to_string(),
                        hash_from_string(&commit_hash.trim())
                            .try_into()
                            .map_err(|_| "Invalid commit hash length in HEAD ref")?,
                    )));
                } else {
                    // If we fail to read the ref file, it could be because the ref is missing
                    // In this case, we can treat it as if the ref exists but points to a non-existent commit, since that's effectively the same state from the perspective of resolving HEAD
                    let trimmed_ref_name = head_ref.trim_start_matches("refs/heads/").to_string();
                    return Ok(GitRef::Head(PlainGitRef::new(trimmed_ref_name, [0u8; 20])));
                }
            } else if head_content.trim().len() == 40
                && head_content.trim().chars().all(|c| c.is_digit(16))
            {
                // Detached HEAD state, where HEAD contains a raw commit hash instead of a ref
                let commit_hash = hash_from_string(head_content.trim());
                return Ok(GitRef::Head(PlainGitRef::new(
                    "HEAD".to_string(),
                    commit_hash
                        .try_into()
                        .map_err(|_| "Invalid commit hash length in detached HEAD")?,
                )));
            } else {
                return Err("HEAD does not contain a valid ref or commit hash".to_string());
            }
        } else {
            return Err(format!("Failed to read HEAD: {}", "unknown error"));
        }
    }

    pub fn from_file_path(ref_path: &str) -> Result<Self, String> {
        let ref_content = std::fs::read_to_string(ref_path)
            .map_err(|e| format!("Failed to read ref file {}: {}", ref_path, e))?;
        let commit_hash = hash_from_string(ref_content.trim());
        let name = std::path::Path::new(ref_path)
            .file_name()
            .ok_or("Invalid ref path: missing file name")?
            .to_str()
            .ok_or("Invalid UTF-8 in ref file name")?
            .to_string();
        // Check to see if this file is under refs/heads, refs/tags, or refs/remotes to determine the type of ref
        if ref_path.contains("refs/heads/") {
            Ok(GitRef::Head(PlainGitRef::new(
                name,
                commit_hash
                    .try_into()
                    .map_err(|_| "Invalid commit hash length in ref file")?,
            )))
        } else if ref_path.contains("refs/tags/") {
            Ok(GitRef::Tag(PlainGitRef::new(
                name,
                commit_hash
                    .try_into()
                    .map_err(|_| "Invalid commit hash length in ref file")?,
            )))
        } else if ref_path.contains("refs/remotes/") {
            // Extract the remote name from the path, which should be in the format refs/remotes/<remote_name>/<ref_name>
            let parts: Vec<&str> = ref_path.split(std::path::MAIN_SEPARATOR).collect();
            let remote_index = parts
                .iter()
                .position(|&p| p == "remotes")
                .ok_or("Invalid remote ref path: missing remotes directory")?;
            if remote_index + 2 >= parts.len() {
                return Err("Invalid remote ref path: missing remote name or ref name".to_string());
            }
            let remote_name = parts[remote_index + 1].to_string();
            Ok(GitRef::Remote {
                remote_name,
                ref_info: PlainGitRef::new(
                    name,
                    commit_hash
                        .try_into()
                        .map_err(|_| "Invalid commit hash length in ref file")?,
                ),
            })
        } else {
            Err(format!(
                "Invalid ref path: {} is not under refs/heads, refs/tags, or refs/remotes",
                ref_path
            ))
        }
    }

    // Invalidates the current ref, so consumes it
    pub fn update_ref(mut self, new_hash: [u8; 20]) -> Result<GitRef, String> {
        match self {
            GitRef::Head(ref mut ref_info) => {
                ref_info.hash = new_hash;
                ref_info.persist_head(true)?; // Force update since we're purposefully overwriting the ref
                Ok(GitRef::Head(ref_info.clone()))
            }
            GitRef::Tag(ref mut ref_info) => {
                ref_info.hash = new_hash;
                ref_info.persist_tag(true)?;
                Ok(GitRef::Tag(ref_info.clone()))
            }
            GitRef::Remote {
                remote_name,
                ref mut ref_info,
            } => {
                ref_info.hash = new_hash;
                ref_info.persist_remote(&remote_name)?;
                Ok(GitRef::Remote {
                    remote_name,
                    ref_info: ref_info.clone(),
                })
            }
        }
    }

    #[allow(dead_code)]
    pub fn head(name: String, hash: [u8; 20]) -> Self {
        GitRef::Head(PlainGitRef::new(name, hash))
    }

    pub fn tag(name: String, hash: [u8; 20]) -> Self {
        GitRef::Tag(PlainGitRef::new(name, hash))
    }

    #[allow(dead_code)]
    pub fn remote(remote_name: String, name: String, hash: [u8; 20]) -> Self {
        GitRef::Remote {
            remote_name,
            ref_info: PlainGitRef::new(name, hash),
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        match self {
            GitRef::Head(ref_info) => &ref_info.name,
            GitRef::Tag(ref_info) => &ref_info.name,
            GitRef::Remote { ref_info, .. } => &ref_info.name,
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> &[u8; 20] {
        match self {
            GitRef::Head(ref_info) => &ref_info.hash,
            GitRef::Tag(ref_info) => &ref_info.hash,
            GitRef::Remote { ref_info, .. } => &ref_info.hash,
        }
    }

    pub fn persist(&self, force: bool) -> Result<(), String> {
        match self {
            GitRef::Head(ref_info) => ref_info.persist_head(force),
            GitRef::Tag(ref_info) => ref_info.persist_tag(force),
            GitRef::Remote {
                remote_name,
                ref_info,
            } => ref_info.persist_remote(remote_name),
        }
    }

    pub fn resolve(&self) -> Result<Object, String> {
        match self {
            GitRef::Head(ref_info) => ref_info.resolve_object(),
            GitRef::Tag(ref_info) => ref_info.resolve_object(),
            GitRef::Remote { ref_info, .. } => ref_info.resolve_object(),
        }
    }
}
