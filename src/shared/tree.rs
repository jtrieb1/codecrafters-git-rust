use std::{convert::TryFrom, os::unix::fs::PermissionsExt};
use sha1::Digest;

use crate::shared::hash::hash_to_string;

use super::object::{Object, ObjectType};

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub mode: String,
    pub name: String,
    pub hash: Vec<u8>,
}

impl TreeEntry {
    pub fn new(mode: String, name: String, hash: Vec<u8>) -> Self {
        TreeEntry { mode, name, hash }
    }

    pub fn to_raw_content(&self) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(self.mode.as_bytes());
        content.push(b' ');
        content.extend_from_slice(self.name.as_bytes());
        content.push(b'\0');
        content.extend_from_slice(&self.hash);
        content
    }
}

#[derive(Debug, Clone)]
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
            let hash = hash_bytes.to_vec();
            i += 20;

            entries.push(TreeEntry::new(mode, name, hash));
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Tree { entries }
    }

    pub fn from_directory(path: &str) -> Result<Self, String> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let name = entry.file_name().into_string().unwrap();

            let metadata = std::fs::metadata(&path).map_err(|e| format!("Failed to get metadata: {}", e))?;
            let mode = metadata.permissions().mode() & 0o777;

            if path.is_file() {
                let mode = 0o100000 + mode;
                let obj = Object::new(ObjectType::Blob, std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?);
                let hash = obj.persist().map_err(|e| format!("Failed to persist blob object: {}", e))?;
                entries.push(TreeEntry::new(format!("{:o}", mode), name, hash));
            } else if path.is_dir() {
                // Check if this is a git directory, if so we skip it
                if name == ".git" {
                    continue;
                }
                // For directories, we need to recursively write trees and get their hashes
                let subtree = Tree::from_directory(path.to_str().unwrap())?;
                let tree_content = subtree.raw_content();
                let tree_object = Object::new(ObjectType::Tree, tree_content);
                let hash = tree_object.persist().map_err(|e| format!("Failed to persist tree object: {}", e))?;
                entries.push(TreeEntry::new("40000".to_string(), name, hash));
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Tree { entries })
    }

    fn raw_content(&self) -> Vec<u8> {
        self.entries
        .iter()
        .flat_map(|entry| {
            entry.to_raw_content()
        }).collect()
    }

    pub fn as_raw_content(&self) -> Vec<u8> {
        let content = self.raw_content();
        let header = format!("{} {}\0", ObjectType::Tree.as_str(), content.len());
        [header.as_bytes(), &content].concat()
    }

    pub fn persist(&self) -> Result<Vec<u8>, String> {
        let content = self.as_raw_content();
        let mut sha = sha1::Sha1::new();
        sha.update(&content);
        let hash = &sha.finalize().to_vec();

        let object_path = Object::hash_to_object_path(&hash);
        std::fs::create_dir_all(std::path::Path::new(&object_path).parent().unwrap()).map_err(|e| format!("Failed to create object directory: {}", e))?;
        let mut encoder = flate2::write::ZlibEncoder::new(std::fs::File::create(object_path).map_err(|e| format!("Failed to create object file: {}", e))?, flate2::Compression::default());
        std::io::copy(&mut std::io::Cursor::new(content), &mut encoder).map_err(|e| format!("Failed to write object content: {}", e))?;
        encoder.finish().map_err(|e| format!("Failed to finish writing object: {}", e))?;

        Ok(hash.to_vec())
    }

    pub fn pretty_print(&self) {
        println!("{} {}\0", ObjectType::Tree.as_str(), self.raw_content().len());
        for entry in &self.entries {
            println!("{} {}\0{}", entry.mode, entry.name, hash_to_string(&entry.hash));
        }
    }

    pub fn print_entries(&self) {
        for entry in &self.entries {
            println!("{} {}\0{}", entry.mode, entry.name, hash_to_string(&entry.hash));
        }
    }

    pub fn print_names(&self) {
        for entry in &self.entries {
            println!("{}", entry.name);
        }
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

impl Into<Object> for Tree {
    fn into(self) -> Object {
        let content = self.raw_content();
        Object::new(ObjectType::Tree, content)
    }
}