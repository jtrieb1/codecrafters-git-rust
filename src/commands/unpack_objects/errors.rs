#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpackObjectsError {
    InvalidPackfile(String),
    IoError(String),
    ObjectParsingError(String),
    PackfileReadError(String),
    UnpackError(String),
}

impl std::fmt::Display for UnpackObjectsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnpackObjectsError::InvalidPackfile(msg) => write!(f, "Invalid packfile: {}", msg),
            UnpackObjectsError::IoError(msg) => write!(f, "I/O error: {}", msg),
            UnpackObjectsError::ObjectParsingError(msg) => {
                write!(f, "Object parsing error: {}", msg)
            }
            UnpackObjectsError::PackfileReadError(msg) => write!(f, "Packfile read error: {}", msg),
            UnpackObjectsError::UnpackError(msg) => write!(f, "Unpack error: {}", msg),
        }
    }
}

impl std::error::Error for UnpackObjectsError {}
