use crate::shared::hash::{hash_from_string, hash_to_string};
use crate::shared::object::Object;
use crate::shared::tag::AnnotatedTag;
use crate::{commands::tag::errors::TagError, shared::refs::GitRef};

use super::input::{TagCreateInput, TagDeleteInput, TagInput};

fn get_tagger_name_and_email() -> Result<(String, String), TagError> {
    let name = std::env::var("GIT_AUTHOR_NAME").map_err(|_| {
        TagError::InvalidInput("GIT_AUTHOR_NAME environment variable not set".to_string())
    })?;
    let email = std::env::var("GIT_AUTHOR_EMAIL").map_err(|_| {
        TagError::InvalidInput("GIT_AUTHOR_EMAIL environment variable not set".to_string())
    })?;
    Ok((name, email))
}

pub fn tag(input: TagInput) -> Result<String, TagError> {
    match input {
        TagInput::Create(create_input) => {
            let TagCreateInput {
                annotated,
                force,
                message,
                file,
                tag_name,
                object,
            } = create_input;

            // Check if the object hash is valid before doing any work
            let hash = hash_from_string(&object);
            if !Object::exists(&hash) {
                return Err(TagError::ObjectNotFound(object));
            }

            if !annotated {
                let tag = GitRef::tag(
                    tag_name.clone(),
                    hash_from_string(&object).try_into().map_err(|e| {
                        TagError::InvalidInput(format!("Invalid object hash: {:?}", e))
                    })?,
                );
                tag.persist(force).map_err(|err| TagError::IoError(err))?;
                return Ok("".to_string());
            }

            let (name, email) = get_tagger_name_and_email()?;
            let obj = Object::try_from_hash(&hash)
                .map_err(|_| TagError::ObjectNotFound(object.clone()))?;

            if let Some(message) = &message {
                let tag = AnnotatedTag::new(
                    tag_name,
                    hash_from_string(&object),
                    obj.object_type().clone(),
                    name,
                    email,
                    message.clone(),
                );
                let tag_hash = tag.persist(force).map_err(|err| TagError::IoError(err))?;
                return Ok(hash_to_string(&tag_hash));
            }

            if let Some(file) = &file {
                let message = std::fs::read_to_string(file)
                    .map_err(|err| TagError::IoError(err.to_string()))?;
                let tag = AnnotatedTag::new(
                    tag_name,
                    hash_from_string(&object),
                    obj.object_type().clone(),
                    name,
                    email,
                    message,
                );
                let tag_hash = tag.persist(force).map_err(|err| TagError::IoError(err))?;
                return Ok(hash_to_string(&tag_hash));
            }

            let tag = AnnotatedTag::new(
                tag_name,
                hash_from_string(&object),
                obj.object_type().clone(),
                name,
                email,
                "".to_string(),
            );
            let tag_hash = tag.persist(force).map_err(|err| TagError::IoError(err))?;
            Ok(hash_to_string(&tag_hash))
        }
        TagInput::Delete(delete_input) => {
            // Basically just need to delete the tag file, but we should also check if it exists first and return an error if it doesn't
            let TagDeleteInput { tag_name } = delete_input;

            let tag_path = format!(".git/refs/tags/{}", tag_name);
            if !std::path::Path::new(&tag_path).exists() {
                return Err(TagError::TagNotFound(tag_name));
            }

            std::fs::remove_file(&tag_path).map_err(|err| TagError::IoError(err.to_string()))?;
            Ok("".to_string())
        }
    }
}
