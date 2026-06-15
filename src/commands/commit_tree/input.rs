use crate::shared::hash::hash_from_string;
use crate::shared::object::{Object, ObjectType};

use super::errors::CommitTreeError;

pub struct CommitTreeInput {
    pub message: String,
    pub parents: Vec<String>,
    pub tree: String,
}

impl CommitTreeInput {
    pub fn validate(&self) -> Result<(), CommitTreeError> {
        if self.message.trim().is_empty() {
            return Err(CommitTreeError::InvalidMessage(
                "Commit message cannot be empty".to_string(),
            ));
        }
        if self.tree.trim().is_empty() {
            return Err(CommitTreeError::InvalidTreeHash(
                "Tree hash cannot be empty".to_string(),
            ));
        }

        let obj = Object::try_from_hash(&hash_from_string(&self.tree)).map_err(|e| {
            CommitTreeError::ObjectNotFound(format!("Tree object not found: {}", e))
        })?;
        if *obj.object_type() != ObjectType::Tree {
            return Err(CommitTreeError::InvalidTreeHash(format!(
                "Object {} is not a tree",
                self.tree
            )));
        }

        for parent_hash in &self.parents {
            if parent_hash.trim().is_empty() {
                return Err(CommitTreeError::InvalidParentHash(
                    "Parent hash cannot be empty".to_string(),
                ));
            }
            let parent_obj =
                Object::try_from_hash(&hash_from_string(parent_hash)).map_err(|e| {
                    CommitTreeError::ObjectNotFound(format!(
                        "Parent commit object not found: {}",
                        e
                    ))
                })?;
            if *parent_obj.object_type() != ObjectType::Commit {
                return Err(CommitTreeError::InvalidParentHash(format!(
                    "Object {} is not a commit",
                    parent_hash
                )));
            }
        }

        Ok(())
    }
}
