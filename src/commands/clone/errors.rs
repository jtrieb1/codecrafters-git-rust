#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloneError {
    RepositoryNotFound(String),
    NetworkError(String),
}

impl std::fmt::Display for CloneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneError::RepositoryNotFound(msg) => write!(f, "Repository not found: {}", msg),
            CloneError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for CloneError {}