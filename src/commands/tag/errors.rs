#[derive(Debug, PartialEq, Eq)]
pub enum TagError {
    InvalidInput(String),
    TagNotFound(String),
    ObjectNotFound(String),
    IoError(String),
}

impl std::fmt::Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            TagError::TagNotFound(name) => write!(f, "Tag not found: {}", name),
            TagError::ObjectNotFound(oid) => write!(f, "Object not found: {}", oid),
            TagError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for TagError {}
