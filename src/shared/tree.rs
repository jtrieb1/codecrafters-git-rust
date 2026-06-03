use std::convert::TryFrom;
use super::object::{Object, ObjectType};

pub struct TreeEntry {
    pub mode: String,
    pub name: String,
    pub hash: String,
}

pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn from_content(content: &[u8]) -> Self {
        let mut entries = Vec::new();
        let mut i = 0;
        while i < content.len() {
            let mode_end = content[i..].iter().position(|&b| b == b' ').unwrap() + i;
            let mode = String::from_utf8_lossy(&content[i..mode_end]).to_string();
            i = mode_end + 1;

            let name_end = content[i..].iter().position(|&b| b == 0).unwrap() + i;
            let name = String::from_utf8_lossy(&content[i..name_end]).to_string();
            i = name_end + 1;

            let hash_bytes = &content[i..i+20];
            let hash = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            i += 20;

            entries.push(TreeEntry { mode, name, hash });
        }
        Tree { entries }
    }
}

impl TryFrom<&Object> for Tree {
    type Error = String;

    fn try_from(object: &Object) -> Result<Self, Self::Error> {
        if object.object_type() != &ObjectType::Tree {
            return Err(format!("Expected tree object, got {}", object.object_type().as_str()));
        }
        Ok(Tree::from_content(object.content()))
    }
}