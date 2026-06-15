use super::object::{Object, ObjectType};
use std::convert::TryFrom;

pub struct Blob {
    content: Vec<u8>,
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Self {
        Blob { content }
    }

    pub fn print_content(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }
}

impl TryFrom<&Object> for Blob {
    type Error = String;

    fn try_from(object: &Object) -> Result<Self, Self::Error> {
        if object.object_type() != &ObjectType::Blob {
            return Err(format!(
                "Expected blob object, got {}",
                object.object_type().as_str()
            ));
        }
        Ok(Blob::new(object.content().to_vec()))
    }
}
