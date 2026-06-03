use crate::shared::hash::hash_from_string;
use crate::shared::hash::hash_to_string;

use super::object::Object;
use super::object::ObjectType;

pub struct CommitAuthor {
    name: String,
    email: String,
    timestamp: i64,
    timezone: String,
}

impl CommitAuthor {
    pub fn from_str(s: &str) -> Result<Self, String> {
        // Format: "[author|committer] Name <email> timestamp timezone"
        // Grab the first whitespace to separate the prefix from the rest of the data
        let prefix_end = s.find(' ').ok_or("Invalid author/committer line: missing prefix")?;

        // Only consider the data after the prefix for parsing the name, email, timestamp, and timezone
        let data_line = s[prefix_end + 1..].trim();

        // Email is enclosed in angle brackets, so we can find the name by looking for the first '<' character
        let name_end = data_line.find('<').ok_or("Invalid author/committer line: missing email")?;
        let name = data_line[..name_end].trim().to_string();

        // The email is between the '<' and '>' characters
        let email_start = name_end + 1;
        let email_end = data_line[email_start..].find('>').ok_or("Invalid author/committer line: malformed email")? + email_start;
        let email = data_line[email_start..email_end].trim().to_string();

        // The timestamp and timezone come after the email, so we can split on whitespace to get them
        let rest = data_line[email_end + 1..].trim();
        let mut rest_parts = rest.split_whitespace();
        let timestamp_str = rest_parts.next().ok_or("Invalid author/committer line: missing timestamp")?;
        let timezone = rest_parts.next().ok_or("Invalid author/committer line: missing timezone")?.to_string();

        // Parse the timestamp as an integer
        let timestamp = timestamp_str.parse::<i64>().map_err(|e| format!("Invalid timestamp: {}", e))?;

        Ok(CommitAuthor { name, email, timestamp, timezone })
    }
}

pub struct Commit {
    tree: Vec<u8>,
    parent: Option<Vec<u8>>,
    author: CommitAuthor,
    committer: CommitAuthor,
    message: String,
}

impl Commit {
    pub fn from_input(message: String, parent: Option<String>, tree: String) -> Result<Self, String> {
        let tree_hash = hash_from_string(&tree);
        let tree_obj = Object::try_from_hash(&tree_hash).map_err(|e| format!("Tree object not found: {}", e))?;
        if *tree_obj.object_type() != ObjectType::Tree {
            return Err(format!("Object {} is not a tree", tree));
        }

        let parent_hash = if let Some(parent) = parent {
            let parent_hash = hash_from_string(&parent);
            let parent_obj = Object::try_from_hash(&parent_hash).map_err(|e| format!("Parent commit object not found: {}", e))?;
            if *parent_obj.object_type() != ObjectType::Commit {
                return Err(format!("Object {} is not a commit", parent));
            }
            Some(parent_hash)
        } else {
            None
        };

        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| format!("System time error: {}", e))?.as_secs() as i64;

        Ok(Commit {
            tree: tree_hash.to_vec(),
            parent: parent_hash.map(|h| h.to_vec()),
            author: CommitAuthor { name: "Author Name".to_string(), email: "author@example.com".to_string(), timestamp, timezone: "+0000".to_string() },
            committer: CommitAuthor { name: "Committer Name".to_string(), email: "committer@example.com".to_string(), timestamp, timezone: "+0000".to_string() },
            message,
        })
    }

    fn parse_tree_line(line: &str) -> Result<Vec<u8>, String> {
        let tree_hash_str = line.strip_prefix("tree ").ok_or("Invalid tree line")?.trim();
        Ok(hash_from_string(tree_hash_str))
    }

    fn parse_parent_line(line: &str) -> Result<Vec<u8>, String> {
        let parent_hash_str = line.strip_prefix("parent ").ok_or("Invalid parent line")?.trim();
        Ok(hash_from_string(parent_hash_str))
    }

    fn parse_author_line(line: &str) -> Result<CommitAuthor, String> {
        CommitAuthor::from_str(line)
    }

    fn parse_committer_line(line: &str) -> Result<CommitAuthor, String> {
        CommitAuthor::from_str(line)
    }

    pub fn from_content(content: &[u8]) -> Result<Self, String> {
        // Everything after the headers is actually plaintext in a commit object, so we can convert to a string for easier parsing
        let content_str = String::from_utf8_lossy(content);
        let mut tree: Option<Vec<u8>> = None;
        let mut parent: Option<Vec<u8>> = None;
        let mut author: Option<CommitAuthor> = None;
        let mut committer: Option<CommitAuthor> = None;
        
        for line in content_str.lines() {
            if line.starts_with("tree ") {
                tree = Some(Self::parse_tree_line(line)?);
                continue;
            }
            if line.starts_with("parent ") {
                parent = Some(Self::parse_parent_line(line)?);
                continue;
            }
            if line.starts_with("author ") {
                author = Some(Self::parse_author_line(line)?);
                continue;
            }
            if line.starts_with("committer ") {
                committer = Some(Self::parse_committer_line(line)?);
                continue;
            }
            // The first blank line indicates the end of headers and the start of the commit message
            if line.trim().is_empty() {
                let message = content_str.splitn(2, "\n\n").nth(1).unwrap_or("").to_string();
                if let (Some(tree), Some(author), Some(committer)) = (tree, author, committer) {
                    return Ok(Commit { tree, parent, author, committer, message });
                } else {
                    return Err("Missing required commit fields".to_string());
                }
            }
        }
        Err("Invalid commit content".to_string())
    }

    pub fn pretty_print(&self) {
        println!("tree {}", hash_to_string(&self.tree));
        if let Some(parent) = &self.parent {
            println!("parent {}", hash_to_string(parent));
        }
        println!("author {} <{}> {} {}", self.author.name, self.author.email, self.author.timestamp, self.author.timezone);
        println!("committer {} <{}> {} {}\n", self.committer.name, self.committer.email, self.committer.timestamp, self.committer.timezone);
        println!("{}", self.message);
    }

    pub fn persist(&self) -> Result<Vec<u8>, String> {
        let object: Object = self.into();
        object.persist()
    }
}

impl TryFrom<&Object> for Commit {
    type Error = String;

    fn try_from(object: &Object) -> Result<Self, Self::Error> {
        if object.object_type() != &ObjectType::Commit {
            return Err("Object is not a commit".to_string());
        }
        let res = Commit::from_content(object.content());
        if let Err(ref e) = res {
            eprintln!("Failed to parse commit content: {}", e);
        }
        res
    }
}

impl From<&Commit> for Object {
    fn from(value: &Commit) -> Self {
        let mut content = format!("tree {}\n", hash_to_string(&value.tree)).into_bytes();
        if let Some(parent) = &value.parent {
            content.extend_from_slice(format!("parent {}\n", hash_to_string(parent)).as_bytes());
        }
        content.extend_from_slice(format!("author {} <{}> {} {}\n", value.author.name, value.author.email, value.author.timestamp, value.author.timezone).as_bytes());
        content.extend_from_slice(format!("committer {} <{}> {} {}\n\n", value.committer.name, value.committer.email, value.committer.timestamp, value.committer.timezone).as_bytes());
        content.extend_from_slice(value.message.as_bytes());

        Object::new(ObjectType::Commit, content)
    }
}