use super::errors::CatFileError;

pub struct CatFileInput {
    pub pretty_print: bool,
    pub ty: bool,
    pub size: bool,
    pub exists: bool,
    pub object: String,
}

impl CatFileInput {
    pub fn validate(&self) -> Result<(), CatFileError> {
        let flags_set =
            self.pretty_print as u8 + self.ty as u8 + self.size as u8 + self.exists as u8;
        if flags_set == 0 {
            Err(CatFileError::NoFlagProvided)
        } else if flags_set > 1 {
            Err(CatFileError::UnknownFlag(
                "Multiple flags provided. Please provide only one of -p, -t, -s, or -e."
                    .to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn as_flag(&self) -> Option<&str> {
        if self.pretty_print {
            Some("p")
        } else if self.ty {
            Some("t")
        } else if self.size {
            Some("s")
        } else if self.exists {
            Some("e")
        } else {
            None
        }
    }
}
