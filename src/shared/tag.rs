use crate::shared::{
    commit::Commit,
    hash::{hash_from_string, hash_to_string},
    object::{Object, ObjectType},
    tree::Tree,
};

pub struct AnnotatedTag {
    pub name: String,
    pub object_hash: Vec<u8>,
    pub object_type: ObjectType,
    pub tagger_name: String,
    pub tagger_email: String,
    pub timestamp: i64,
    pub timezone: String,
    pub message: String,
}

impl AnnotatedTag {
    pub fn new(
        name: String,
        object_hash: Vec<u8>,
        object_type: ObjectType,
        tagger_name: String,
        tagger_email: String,
        message: String,
    ) -> Self {
        let timestamp = chrono::Utc::now().timestamp();
        let timezone = chrono::Local::now().format("%z").to_string();
        AnnotatedTag {
            name,
            object_hash,
            object_type,
            tagger_name,
            tagger_email,
            timestamp,
            timezone,
            message,
        }
    }

    pub fn tag_object(
        obj: &Object,
        name: String,
        tagger_name: String,
        tagger_email: String,
        message: String,
    ) -> Self {
        let object_hash = obj.get_hash();
        let object_type = obj.object_type().clone();
        let timestamp = chrono::Utc::now().timestamp();
        let timezone = chrono::Local::now().format("%z").to_string();
        AnnotatedTag {
            name,
            object_hash,
            object_type,
            tagger_name,
            tagger_email,
            timestamp,
            timezone,
            message,
        }
    }

    pub fn from_file(rel_path: &str) -> Result<Self, String> {
        let path = format!(".git/refs/tags/{}", rel_path);
        let content = std::fs::read(&path)
            .map_err(|e| format!("Failed to read tag ref file {}: {}", path, e))?;
        AnnotatedTag::from_content(&content)
    }

    pub fn from_content(content: &[u8]) -> Result<Self, String> {
        let content_str = String::from_utf8_lossy(content);
        let lines = content_str.lines();

        let mut name = None;
        let mut object_hash = None;
        let mut object_type = None;
        let mut tagger_name = None;
        let mut tagger_email = None;
        let mut timestamp = None;
        let mut timezone = None;
        let mut message_lines = Vec::new();
        let mut in_message = false;

        for line in lines {
            if line.is_empty() {
                in_message = true;
                continue;
            }

            if in_message {
                message_lines.push(line);
            } else if line.starts_with("object ") {
                object_hash = Some(hash_from_string(&line[7..]));
            } else if line.starts_with("type ") {
                // Type is one of the following: commit, tree, blob, tag
                let obj_type_str = &line[5..];
                let obj_type = match obj_type_str {
                    "commit" => ObjectType::Commit,
                    "tree" => ObjectType::Tree,
                    "blob" => ObjectType::Blob,
                    "tag" => ObjectType::Tag,
                    _ => {
                        return Err(format!(
                            "Unknown object type in tag content: {}",
                            obj_type_str
                        ));
                    }
                };
                object_type = Some(obj_type);
            } else if line.starts_with("tag ") {
                name = Some(line[4..].to_string());
            } else if line.starts_with("tagger ") {
                let tagger_info = &line[7..];
                // Tagger info looks like:
                // Alice <alice@email.com> Sat May 23 16:48:58 2009 -0700
                // So we can split on the first '<' to get the name, split on '>' to get the email, then rsplit on the last space to get the timestamp and timezone
                if let Some(start_email) = tagger_info.find('<') {
                    let name_part = &tagger_info[..start_email].trim();
                    let rest = &tagger_info[start_email + 1..];
                    if let Some(end_email) = rest.find('>') {
                        let email_part = &rest[..end_email].trim();
                        let timestamp_part = &rest[end_email + 1..].trim();
                        tagger_name = Some(name_part.to_string());
                        tagger_email = Some(email_part.to_string());
                        let parsed = chrono::DateTime::parse_from_str(
                            timestamp_part,
                            "%a %b %d %H:%M:%S %Y %z",
                        )
                        .map(|dt| dt.timestamp())
                        .map_err(|e| format!("Failed to parse timestamp with timezone: {}", e))
                        .ok();
                        if let Some(parsed_timestamp) = parsed {
                            timestamp = Some(parsed_timestamp);
                            timezone = Some(
                                timestamp_part
                                    .split_whitespace()
                                    .last()
                                    .unwrap_or("")
                                    .to_string(),
                            );
                        } else {
                            return Err(format!(
                                "Failed to parse tagger timestamp: '{}'",
                                timestamp_part
                            ));
                        }
                    }
                }
            }
        }

        let name = name.ok_or_else(|| "Missing tag name".to_string())?;
        let object_hash = object_hash.ok_or_else(|| "Missing object hash".to_string())?;
        let object_type = object_type.ok_or_else(|| "Missing object type".to_string())?;
        let tagger_name = tagger_name.ok_or_else(|| "Missing tagger name".to_string())?;
        let tagger_email = tagger_email.ok_or_else(|| "Missing tagger email".to_string())?;
        let timestamp = timestamp.ok_or_else(|| "Missing timestamp".to_string())?;
        let timezone = timezone.ok_or_else(|| "Missing timezone".to_string())?;
        let message = message_lines.join("\n");

        Ok(AnnotatedTag {
            name,
            object_hash,
            object_type,
            tagger_name,
            tagger_email,
            timestamp,
            timezone,
            message,
        })
    }

    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("tag {}\n", self.name));
        output.push_str(&format!("object {}\n", hash_to_string(&self.object_hash)));
        output.push_str(&format!(
            "type {}\n",
            match self.object_type {
                ObjectType::Commit => "commit",
                ObjectType::Tree => "tree",
                ObjectType::Blob => "blob",
                ObjectType::Tag => "tag",
            }
        ));
        output.push_str(&format!(
            "tagger {} <{}>\n",
            self.tagger_name, self.tagger_email
        ));
        output.push_str("\n");
        output.push_str(&format!("{}\n", self.message));
        output
    }

    pub fn to_raw_content(&self) -> Vec<u8> {
        let mut content = Vec::new();
        content.extend_from_slice(
            format!("object {}\n", hash_to_string(&self.object_hash)).as_bytes(),
        );
        content.extend_from_slice(
            format!(
                "type {}\n",
                match self.object_type {
                    ObjectType::Commit => "commit",
                    ObjectType::Tree => "tree",
                    ObjectType::Blob => "blob",
                    ObjectType::Tag => "tag",
                }
            )
            .as_bytes(),
        );
        content.extend_from_slice(format!("tag {}\n", self.name).as_bytes());

        let hour_offset = self.timezone[1..3].parse::<i32>().unwrap_or(0);
        let minute_offset = self.timezone[3..5].parse::<i32>().unwrap_or(0);
        let total_offset_seconds = hour_offset * 3600 + minute_offset * 60;
        let total_offset_seconds = if self.timezone.starts_with('-') {
            -total_offset_seconds
        } else {
            total_offset_seconds
        };
        let dt = chrono::DateTime::from_timestamp(self.timestamp, 0);
        let tz = chrono::FixedOffset::east_opt(total_offset_seconds)
            .unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());
        let dt = dt
            .map(|dt| dt.with_timezone(&tz))
            .map(|dt| dt.format("%a %b %d %H:%M:%S %Y").to_string())
            .unwrap_or_else(|| self.timestamp.to_string());

        content.extend_from_slice(
            format!(
                "tagger {} <{}> {} {}\n",
                self.tagger_name, self.tagger_email, dt, self.timezone
            )
            .as_bytes(),
        );
        content.push(b'\n');
        content.extend_from_slice(self.message.as_bytes());
        content
    }

    pub fn resolve_target(&self) -> Result<Object, String> {
        let obj = Object::try_from_hash(&self.object_hash)
            .map_err(|e| format!("Failed to read object for tag {}: {}", self.name, e))?;
        if obj.object_type() == &ObjectType::Tag {
            let tag = AnnotatedTag::try_from(&obj)
                .map_err(|e| format!("Failed to parse tag object for tag {}: {}", self.name, e))?;
            tag.resolve_target()
        } else {
            Ok(obj)
        }
    }

    pub fn try_get_tree(&self) -> Result<Tree, String> {
        let object = self.resolve_target()?;
        if object.object_type() == &ObjectType::Commit {
            let commit = Commit::try_from(&object)?;
            commit.try_get_tree()
        } else if object.object_type() == &ObjectType::Tag {
            let tag = AnnotatedTag::try_from(&object)?;
            tag.try_get_tree()
        } else if object.object_type() == &ObjectType::Tree {
            Tree::try_from(&object)
        } else {
            Err(format!(
                "Tag {} points to an object that does not terminate in a tree",
                self.name
            ))
        }
    }

    pub fn persist(&self, force: bool) -> Result<Vec<u8>, String> {
        let ref_path = format!(".git/refs/tags/{}", self.name);
        if !force && std::path::Path::new(&ref_path).exists() {
            return Err(format!(
                "Tag {} already exists. Use force option to overwrite.",
                self.name
            ));
        }

        // First, persist self as an object
        let object: Object = self.into();
        let obj_hash = object
            .persist()
            .map_err(|e| format!("Failed to persist tag object: {}", e))?;

        // Then create the ref file pointing to the tag object
        std::fs::create_dir_all(std::path::Path::new(&ref_path).parent().unwrap())
            .map_err(|e| format!("Failed to create directories for tag ref: {}", e))?;
        std::fs::write(&ref_path, hash_to_string(&obj_hash))
            .map_err(|e| format!("Failed to write tag ref: {}", e))?;
        Ok(obj_hash)
    }
}

impl TryFrom<&Object> for AnnotatedTag {
    type Error = String;

    fn try_from(object: &Object) -> Result<Self, Self::Error> {
        if object.object_type() != &ObjectType::Tag {
            return Err("Object is not a tag".to_string());
        }
        AnnotatedTag::from_content(object.content())
    }
}

impl Into<Object> for &AnnotatedTag {
    fn into(self) -> Object {
        let content = self.to_raw_content();
        Object::new(ObjectType::Tag, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_parsing() {
        let tag_content = b"object 0123456789abcdef0123456789abcdef01234567\n\
type commit\n\
tag v1.0\n\
tagger Alice <alice@example.com> Sat May 23 16:48:58 2009 -0400\n\n\
This is a test tag.\n";

        let tag = AnnotatedTag::from_content(tag_content).expect("Failed to parse tag content");
        assert_eq!(tag.name, "v1.0");
        assert_eq!(
            hash_to_string(&tag.object_hash),
            "0123456789abcdef0123456789abcdef01234567"
        );
        assert_eq!(tag.tagger_name, "Alice");
        assert_eq!(tag.tagger_email, "alice@example.com");
        assert_eq!(tag.timestamp, 1243111738);
        assert_eq!(tag.timezone, "-0400");
        assert_eq!(tag.message, "This is a test tag.");
    }

    #[test]
    fn test_tag_to_object() {
        let tag = AnnotatedTag {
            name: "v1.0".to_string(),
            object_hash: hash_from_string("0123456789abcdef0123456789abcdef01234567"),
            object_type: ObjectType::Commit,
            tagger_name: "Alice".to_string(),
            tagger_email: "alice@example.com".to_string(),
            timestamp: 1243111738,
            timezone: "-0400".to_string(),
            message: "This is a test tag.".to_string(),
        };
        let object: Object = (&tag).into();
        assert_eq!(object.object_type(), &ObjectType::Tag);
    }

    #[test]
    fn test_tag_pretty_print() {
        let tag = AnnotatedTag {
            name: "v1.0".to_string(),
            object_hash: hash_from_string("0123456789abcdef0123456789abcdef01234567"),
            object_type: ObjectType::Commit,
            tagger_name: "Alice".to_string(),
            tagger_email: "alice@example.com".to_string(),
            timestamp: 1243111738,
            timezone: "-0400".to_string(),
            message: "This is a test tag.".to_string(),
        };
        tag.pretty_print();
    }

    #[test]
    fn test_tag_round_trip() {
        let original_tag = AnnotatedTag {
            name: "v1.0".to_string(),
            object_hash: hash_from_string("0123456789abcdef0123456789abcdef01234567"),
            object_type: ObjectType::Commit,
            tagger_name: "Alice".to_string(),
            tagger_email: "alice@example.com".to_string(),
            timestamp: 1243111738,
            timezone: "-0400".to_string(),
            message: "This is a test tag.".to_string(),
        };
        let round_trip_tag = AnnotatedTag::from_content(&(&original_tag.to_raw_content()))
            .expect("Failed to parse tag content");
        assert_eq!(original_tag.name, round_trip_tag.name);
        assert_eq!(original_tag.object_hash, round_trip_tag.object_hash);
        assert_eq!(original_tag.tagger_name, round_trip_tag.tagger_name);
        assert_eq!(original_tag.tagger_email, round_trip_tag.tagger_email);
        assert_eq!(original_tag.timestamp, round_trip_tag.timestamp);
        assert_eq!(original_tag.timezone, round_trip_tag.timezone);
        assert_eq!(original_tag.message, round_trip_tag.message);
    }
}
