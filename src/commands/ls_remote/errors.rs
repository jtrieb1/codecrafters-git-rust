#[derive(Debug, PartialEq, Eq)]
pub enum LsRemoteError {
    RepositoryNotFound(String),
    NetworkError(String),
}

impl std::fmt::Display for LsRemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LsRemoteError::RepositoryNotFound(msg) => write!(f, "Repository not found: {}", msg),
            LsRemoteError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for LsRemoteError {}