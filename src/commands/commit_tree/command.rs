use crate::shared::commit::Commit;
use crate::shared::hash::hash_to_string;
use crate::shared::refs::GitRef;

use super::errors::CommitTreeError;
use super::input::CommitTreeInput;

pub fn commit_tree(input: CommitTreeInput) -> Result<String, CommitTreeError> {
    input.validate()?;

    let comm = Commit::from_input(input.message, input.parents, input.tree).unwrap(); // Safe to unwrap since we've already validated the input

    let hash = comm
        .persist()
        .map_err(|e| CommitTreeError::ObjectNotFound(format!("Failed to persist commit: {}", e)))?;
    let hash_str = hash_to_string(&hash);

    // Update HEAD to point to the new commit
    let Ok(head) = GitRef::current_head() else {
        return Err(CommitTreeError::HeadUpdateError(
            "Failed to read current HEAD".to_string(),
        ));
    };

    let Ok(_) = head.update_ref(hash.try_into().map_err(|e| {
        CommitTreeError::HeadUpdateError(format!("Failed to convert hash: {:?}", e))
    })?) else {
        return Err(CommitTreeError::HeadUpdateError(
            "Failed to update HEAD to new commit".to_string(),
        ));
    };

    Ok(hash_str)
}
