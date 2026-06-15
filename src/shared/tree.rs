use sha1::Digest;
use std::{convert::TryFrom, os::unix::fs::PermissionsExt};

use crate::shared::hash::hash_to_string;

use super::object::{Object, ObjectType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeMode {
    File(u32),
    Directory,
}

impl TreeMode {
    pub fn from_str(s: &str) -> Result<Self, String> {
        if s == "40000" {
            Ok(TreeMode::Directory)
        } else {
            let mode =
                u32::from_str_radix(s, 8).map_err(|e| format!("Invalid tree mode: {}", e))?;
            Ok(TreeMode::File(mode - 0o100000)) // For some reason git stores file modes as 100000 + actual mode, so we need to subtract 100000 to get the actual mode
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            TreeMode::File(mode) => format!("{:o}", 0o100000 + mode),
            TreeMode::Directory => "40000".to_string(),
        }
    }
}

impl std::fmt::Display for TreeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub mode: TreeMode,
    pub name: String,
    pub hash: Vec<u8>,
}

impl TreeEntry {
    pub fn new(mode: TreeMode, name: String, hash: Vec<u8>) -> Self {
        TreeEntry { mode, name, hash }
    }

    pub fn to_raw_content(&self) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(self.mode.as_str().as_bytes());
        content.push(b' ');
        content.extend_from_slice(self.name.as_bytes());
        content.push(b'\0');
        content.extend_from_slice(&self.hash);
        content
    }

    pub fn to_referenced_object(&self) -> Result<(Object, TreeMode), String> {
        let object = Object::try_from_hash(&self.hash).map_err(|e| {
            format!(
                "Failed to read object {}: {}",
                hash_to_string(&self.hash),
                e
            )
        })?;
        return Ok((object, self.mode));
    }
}

#[derive(Debug, Clone)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn entries(&self) -> &Vec<TreeEntry> {
        &self.entries
    }

    pub fn empty() -> Self {
        Tree {
            entries: Vec::new(),
        }
    }

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

            let hash_bytes = &content[i..i + 20];
            let hash = hash_bytes.to_vec();
            i += 20;

            entries.push(TreeEntry::new(
                TreeMode::from_str(&mode).unwrap(),
                name,
                hash,
            ));
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Tree { entries }
    }

    pub fn from_directory(path: &str) -> Result<Self, String> {
        let explicit_path = std::path::Path::new(path);
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(explicit_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            let name = entry.file_name().into_string().unwrap();

            let metadata =
                std::fs::metadata(&path).map_err(|e| format!("Failed to get metadata: {}", e))?;
            let mode = metadata.permissions().mode() & 0o777;

            if path.is_file() {
                let mode = 0o100000 + mode;
                let obj = Object::new(
                    ObjectType::Blob,
                    std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?,
                );
                let hash = obj
                    .persist()
                    .map_err(|e| format!("Failed to persist blob object: {}", e))?;
                entries.push(TreeEntry::new(
                    TreeMode::from_str(&format!("{:o}", mode)).unwrap(),
                    name,
                    hash,
                ));
            } else if path.is_dir() {
                // Check if this is a git directory, if so we skip it
                if name == ".git" {
                    continue;
                }
                // For directories, we need to recursively write trees and get their hashes
                let subtree = Tree::from_directory(path.to_str().unwrap())?;
                let tree_content = subtree.raw_content();
                let tree_object = Object::new(ObjectType::Tree, tree_content);
                let hash = tree_object
                    .persist()
                    .map_err(|e| format!("Failed to persist tree object: {}", e))?;
                entries.push(TreeEntry::new(TreeMode::Directory, name, hash));
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Tree { entries })
    }

    fn raw_content(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|entry| entry.to_raw_content())
            .collect()
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
        std::fs::create_dir_all(std::path::Path::new(&object_path).parent().unwrap())
            .map_err(|e| format!("Failed to create object directory: {}", e))?;
        let mut encoder = flate2::write::ZlibEncoder::new(
            std::fs::File::create(object_path)
                .map_err(|e| format!("Failed to create object file: {}", e))?,
            flate2::Compression::default(),
        );
        std::io::copy(&mut std::io::Cursor::new(content), &mut encoder)
            .map_err(|e| format!("Failed to write object content: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Failed to finish writing object: {}", e))?;

        Ok(hash.to_vec())
    }

    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "{} {}\0\n",
            ObjectType::Tree.as_str(),
            self.raw_content().len()
        ));
        for entry in &self.entries {
            output.push_str(&format!(
                "{} {}\0{}\n",
                entry.mode,
                entry.name,
                hash_to_string(&entry.hash)
            ));
        }
        output
    }

    pub fn print_entries(&self) -> String {
        let mut output = String::new();
        for entry in &self.entries {
            output.push_str(&format!(
                "{} {}\0{}\n",
                entry.mode,
                entry.name,
                hash_to_string(&entry.hash)
            ));
        }
        output.pop();
        output
    }

    pub fn print_names(&self) -> String {
        let mut output = String::new();
        for entry in &self.entries {
            output.push_str(&format!("{}\n", entry.name));
        }
        // Remove trailing newline
        output.pop();
        output
    }
}

impl TryFrom<&Object> for Tree {
    type Error = String;

    fn try_from(object: &Object) -> Result<Self, Self::Error> {
        if object.object_type() != &ObjectType::Tree {
            return Err(format!(
                "Expected tree object, got {}",
                object.object_type().as_str()
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::CwdGuard;
    use serial_test::serial;

    #[test]
    fn test_tree_entry_to_raw_content() {
        let entry = TreeEntry::new(
            TreeMode::File(0o644),
            "test.txt".to_string(),
            vec![
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
                0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            ],
        );
        let raw_content = entry.to_raw_content();
        let expected_content = b"100644 test.txt\0\x12\x34\x56\x78\x9a\xbc\xde\xf0\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\xbb\xcc".to_vec();
        assert_eq!(raw_content, expected_content);
    }

    #[test]
    fn test_tree_from_content() {
        let content = b"100644 test.txt\0\x12\x34\x56\x78\x9a\xbc\xde\xf0\x11\x22\x33\x44\x55\x66\x77\x88\x99\xaa\xbb\xcc".to_vec();
        let tree = Tree::from_content(&content);
        assert_eq!(tree.entries.len(), 1);
        let entry = &tree.entries[0];
        assert_eq!(entry.mode, TreeMode::File(0o644));
        assert_eq!(entry.name, "test.txt");
        assert_eq!(
            entry.hash,
            vec![
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
                0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc
            ]
        );
    }

    #[test]
    #[serial]
    fn test_directory_related() {
        let tempdir = tempfile::tempdir().unwrap();

        let _cwd_guard = CwdGuard::set_to(tempdir.path());
        std::fs::create_dir(tempdir.path().join("test_dir")).unwrap();
        std::fs::write(
            tempdir.path().join("test_dir").join("test.txt"),
            "Hello, world!",
        )
        .unwrap();
        std::fs::create_dir(tempdir.path().join("test_dir").join("subdir")).unwrap();

        test_tree_from_directory();
        test_tree_persist_and_try_from();
    }

    fn test_tree_from_directory() {
        let tree = Tree::from_directory("test_dir").unwrap();
        assert_eq!(tree.entries.len(), 2);
        // Entries are sorted alphabetically by name, so the first entry should be subdir and the second entry should be test.txt
        let entry1 = &tree.entries[0];
        assert_eq!(entry1.mode, TreeMode::Directory);
        assert_eq!(entry1.name, "subdir");
        let entry2 = &tree.entries[1];
        assert!(matches!(entry2.mode, TreeMode::File(_)));
        assert_eq!(entry2.name, "test.txt");
    }

    fn test_tree_persist_and_try_from() {
        let tree = Tree::from_directory("test_dir").unwrap();
        let hash = tree.persist().unwrap();
        let object = Object::try_from_hash(&hash).unwrap();
        let tree_from_object = Tree::try_from(&object).unwrap();
        assert_eq!(tree.entries.len(), tree_from_object.entries.len());
        for (entry1, entry2) in tree.entries.iter().zip(tree_from_object.entries.iter()) {
            assert_eq!(entry1.mode, entry2.mode);
            assert_eq!(entry1.name, entry2.name);
            assert_eq!(entry1.hash, entry2.hash);
        }
    }
}
