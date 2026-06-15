use super::errors::CatFileError;
use super::input::CatFileInput;
use crate::shared::hash::hash_to_string;
use crate::shared::refs::GitRef;
use crate::shared::{hash::hash_from_string, object::Object};

fn get_input_object(input_obj: &str) -> Result<Object, CatFileError> {
    // Need to check for 3 possible kinds:
    // SHA-1 hash, ref name, or tree-ish (commit/tag/tree)
    // Check if it's a valid SHA-1 hash first
    // Just check if it's 40 hex characters, since that's the only valid format for a hash input
    if input_obj.len() == 40 && input_obj.chars().all(|c| c.is_digit(16)) {
        let hash = hash_from_string(input_obj);
        let object = Object::try_from_hash(&hash).map_err(|_| {
            CatFileError::FileNotFound(
                hash_to_string(&hash)
            )
        })?;
        return Ok(object);
    }

    // If not a hash, check if it's a ref name
    let ref_path = format!(".git/refs/heads/{}", input_obj);
    if std::fs::metadata(&ref_path).is_ok() {
        let Ok(ref_head) = GitRef::from_file_path(&ref_path) else {
            return Err(CatFileError::FileNotFound(
                input_obj.to_string()
            ));
        };
        let object = ref_head.resolve().map_err(|e| {
            CatFileError::FileNotFound(format!("Failed to resolve ref {}: {}", input_obj, e))
        })?;
        return Ok(object);
    }

    // Check if it's a tag name
    let tag_ref_path = format!(".git/refs/tags/{}", input_obj);
    if std::fs::metadata(&tag_ref_path).is_ok() {
        let Ok(ref_tag) = GitRef::from_file_path(&tag_ref_path) else {
            return Err(CatFileError::FileNotFound(format!(
                "Failed to read tag ref {}: unknown error",
                input_obj
            )));
        };
        let object = ref_tag.resolve().map_err(|e| {
            CatFileError::FileNotFound(format!("Failed to resolve tag ref {}: {}", input_obj, e))
        })?;
        return Ok(object);
    }
    Err(CatFileError::FileNotFound(format!(
        "Invalid input: {}",
        input_obj
    )))
}

pub fn cat_file(input: CatFileInput) -> Result<String, CatFileError> {
    let flag = input.as_flag().ok_or(CatFileError::NoFlagProvided)?;
    let object = get_input_object(&input.object);

    if flag == "e" {
        if let Err(_) = object {
            return Err(CatFileError::FileNotFound(input.object))
        }
        return Ok("".to_string());
    }

    let object = object?;
    if flag == "p" {
        return Ok(object.pretty_print());
    } else if flag == "t" {
        return Ok(object.object_type().as_str().to_string());
    } else if flag == "s" {
        return Ok(object.size().to_string());
    } else {
        return Err(CatFileError::UnknownFlag(flag.to_string()));
    }
}
