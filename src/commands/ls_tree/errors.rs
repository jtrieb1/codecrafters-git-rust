#[derive(Debug, PartialEq, Eq)]
pub enum LsTreeError {
    ObjectNotFound(String),
    NotATree(String),
}

impl std::fmt::Display for LsTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LsTreeError::ObjectNotFound(sha) => write!(f, "Object not found: {}", sha),
            LsTreeError::NotATree(sha) => write!(f, "Object is not a tree: {}", sha),
        }
    }
}