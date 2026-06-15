#[derive(Debug, PartialEq, Eq)]
pub enum HashObjectError {
    InvalidType(String),
    StdinAndStdinPathsConflict,
    NoInputProvided,
    IoError(String),
}

impl std::fmt::Display for HashObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashObjectError::InvalidType(ty) => write!(
                f,
                "Invalid type: {}. Valid types are blob, tree, commit, tag.",
                ty
            ),
            HashObjectError::StdinAndStdinPathsConflict => {
                write!(f, "Cannot use --stdin and --stdin-paths together.")
            }
            HashObjectError::NoInputProvided => write!(
                f,
                "Must provide either --stdin, --stdin-paths, or at least one file."
            ),
            HashObjectError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}
