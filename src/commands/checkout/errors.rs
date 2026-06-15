#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutError {
    InvalidCommitHash(String),
    CommitNotFound(String),
    InvalidCommitContent(String),
    IOError(String),
}

impl std::fmt::Display for CheckoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckoutError::InvalidCommitHash(msg) => write!(f, "Invalid commit hash: {}", msg),
            CheckoutError::CommitNotFound(msg) => write!(f, "Commit not found: {}", msg),
            CheckoutError::InvalidCommitContent(msg) => {
                write!(f, "Invalid commit content: {}", msg)
            }
            CheckoutError::IOError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for CheckoutError {}
