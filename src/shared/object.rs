use std::io::Read;

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

    pub fn hash_to_object_path(hash: &str) -> String {
        let dir = &hash[0..2];
        let file = &hash[2..];
        format!(".git/objects/{}/{}", dir, file)
    }

    pub fn object_type(&self) -> &ObjectType {
        &self.object_type
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn content(&self) -> &[u8] {
        &self.content
    }

    pub fn from_hash(hash: &str) -> Result<Self, String> {
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
}