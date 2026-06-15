#[derive(Debug, PartialEq, Eq)]
pub enum CatFileError {
    NoFlagProvided,
    UnknownFlag(String),
    FileNotFound(String)
}

impl std::fmt::Display for CatFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatFileError::NoFlagProvided => write!(
                f,
                "No flag provided. Please provide one of -p, -t, -s, or -e."
            ),
            CatFileError::UnknownFlag(flag) => write!(
                f,
                "Unknown flag: {}. Please provide one of -p, -t, -s, or -e.",
                flag
            ),
            CatFileError::FileNotFound(reference) => write!(
                f,
                "Failed to find object corresponding to input {}",
                reference
            )
        }
    }
}
