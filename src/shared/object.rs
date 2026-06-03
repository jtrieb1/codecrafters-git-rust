use std::io::Read;

use sha1::Digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl ObjectType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "blob" => Some(ObjectType::Blob),
            "tree" => Some(ObjectType::Tree),
            "commit" => Some(ObjectType::Commit),
            "tag" => Some(ObjectType::Tag),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
        }
    }
}

pub struct Object {
    object_type: ObjectType,
    size: usize,
    content: Vec<u8>,
}

impl Object {

    pub fn new(object_type: ObjectType, content: Vec<u8>) -> Self {
        let size = content.len();
        Object { object_type, size, content }
    }

    pub fn hash_to_object_path(hash: &[u8]) -> String {
        let hash_str = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let dir = &hash_str[0..2];
        let file = &hash_str[2..];
        format!(".git/objects/{}/{}", dir, file)
    }

    pub fn object_type(&self) -> &ObjectType {
        &self.object_type
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn header(&self) -> String {
        format!("{} {}\0", self.object_type.as_str(), self.size)
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn try_from_hash(hash: &[u8]) -> Result<Self, String> {
        let object_path = Object::hash_to_object_path(hash);
        let file = std::fs::File::open(object_path).map_err(|e| format!("Failed to open object file: {}", e))?;
        let mut decoder = flate2::read::ZlibDecoder::new(file);
        let mut content = Vec::new();
        decoder.read_to_end(&mut content).map_err(|e| format!("Failed to read object content: {}", e))?;

        let mut parts = content.splitn(2, |&b| b == 0);
        let header = parts.next().ok_or("Invalid object format: missing header")?;
        let content = parts.next().ok_or("Invalid object format: missing content")?;
        let header = std::str::from_utf8(header).map_err(|e| format!("Invalid UTF-8 in object header: {}", e))?;
        let mut header_parts = header.split_whitespace();
        let object_type = header_parts.next().ok_or("Invalid object format: missing object type")?.to_string();
        let size = header_parts.next().ok_or("Invalid object format: missing size")?.parse::<usize>().map_err(|e| format!("Invalid size in object header: {}", e))?;

        Ok(Object {
            object_type: ObjectType::from_str(&object_type).ok_or("Invalid object type")?,
            size,
            content: content.to_vec(),
        })
    }

    pub fn as_raw(&self) -> Vec<u8> {
        let header = self.header();
        [header.as_bytes(), &self.content].concat()
    }

    pub fn as_compressed(&self) -> Vec<u8> {
        let raw = self.as_raw();
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::copy(&mut std::io::Cursor::new(raw), &mut encoder).unwrap();
        encoder.finish().unwrap()
    }

    pub fn persist(&self) -> Result<Vec<u8>, String> {
        let compressed = self.as_compressed();
        let mut sha = sha1::Sha1::new();
        sha.update(&compressed);
        let hash = &sha.finalize()[..];

        let object_path = Object::hash_to_object_path(&hash);
        std::fs::create_dir_all(std::path::Path::new(&object_path).parent().unwrap()).map_err(|e| format!("Failed to create object directory: {}", e))?;
        std::fs::write(object_path, compressed).map_err(|e| format!("Failed to write object file: {}", e))?;

        Ok(hash.to_vec())
    }
}