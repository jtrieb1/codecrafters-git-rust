
#[derive(Debug, PartialEq, Eq)]
pub enum CommitTreeError {
    InvalidTreeHash(String),
    InvalidParentHash(String),
    InvalidMessage(String),
    ObjectNotFound(String),
}

impl std::fmt::Display for CommitTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitTreeError::InvalidTreeHash(e) => write!(f, "Invalid tree hash: {}", e),
            CommitTreeError::InvalidParentHash(e) => write!(f, "Invalid parent hash: {}", e),
            CommitTreeError::InvalidMessage(e) => write!(f, "Invalid commit message: {}", e),
            CommitTreeError::ObjectNotFound(e) => write!(f, "Object not found: {}", e),
        }
    }
}