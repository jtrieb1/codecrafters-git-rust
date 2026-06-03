use crate::shared::commit::Commit;
use crate::shared::hash::hash_to_string;

use super::input::CommitTreeInput;
use super::errors::CommitTreeError;

pub fn commit_tree(input: CommitTreeInput) -> Result<(), CommitTreeError> {
    input.validate()?;

    let comm = Commit::from_input(input.message, input.parent, input.tree).unwrap(); // Safe to unwrap since we've already validated the input

    let hash = comm.persist().map_err(|e| CommitTreeError::ObjectNotFound(format!("Failed to persist commit: {}", e)))?;
    println!("{}", hash_to_string(&hash));

    Ok(())
}