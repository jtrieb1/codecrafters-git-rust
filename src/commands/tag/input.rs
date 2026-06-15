pub enum TagInput {
    Create(TagCreateInput),
    Delete(TagDeleteInput),
}

pub struct TagCreateInput {
    pub annotated: bool,
    pub force: bool,
    pub message: Option<String>,
    pub file: Option<String>,
    pub tag_name: String,
    pub object: String,
}

pub struct TagDeleteInput {
    pub tag_name: String,
}

impl From<crate::Commands> for TagInput {
    fn from(cmd: crate::Commands) -> Self {
        match cmd {
            crate::Commands::Tag {
                annotated,
                delete,
                force,
                message,
                file,
                tag_name,
                object,
            } => {
                if delete {
                    TagInput::Delete(TagDeleteInput { tag_name })
                } else {
                    TagInput::Create(TagCreateInput {
                        annotated,
                        force,
                        message,
                        file,
                        tag_name,
                        object,
                    })
                }
            }
            _ => panic!("Invalid command for TagInput"),
        }
    }
}

impl TagCreateInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.message.is_some() && self.file.is_some() {
            return Err("Cannot specify both message and file for tag creation".to_string());
        }
        Ok(())
    }
}

impl TagDeleteInput {
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}
