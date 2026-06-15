use std::path::PathBuf;

use super::errors::HashObjectError;
use crate::shared::object::ObjectType;

pub struct HashObjectInput {
    pub write: bool,
    pub ty: String,
    pub stdin: bool,
    pub stdin_paths: bool,
    pub file: Vec<PathBuf>,
}

impl HashObjectInput {
    pub fn validate(&self) -> Result<(), HashObjectError> {
        if ObjectType::from_str(&self.ty).is_none() {
            return Err(HashObjectError::InvalidType(self.ty.clone()));
        }
        if self.stdin && self.stdin_paths {
            return Err(HashObjectError::StdinAndStdinPathsConflict);
        }
        if !self.stdin && !self.stdin_paths && self.file.is_empty() {
            return Err(HashObjectError::NoInputProvided);
        }
        Ok(())
    }
}
