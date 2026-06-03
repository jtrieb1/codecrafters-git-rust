#[derive(Debug, PartialEq, Eq)]
pub enum WriteTreeError {
    IoError(String),
}

impl std::fmt::Display for WriteTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteTreeError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}