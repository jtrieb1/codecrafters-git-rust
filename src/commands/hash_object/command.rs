use sha1::{Digest, Sha1};
use std::fs;
use std::io::{BufRead, Read};

use super::{errors::HashObjectError, input::HashObjectInput};
use crate::shared::hash::hash_to_string;
use crate::shared::object::{Object, ObjectType};

pub fn hash_object(input: HashObjectInput) -> Result<String, HashObjectError> {
    input.validate()?;

    let Some(ty) = ObjectType::from_str(&input.ty) else {
        return Err(HashObjectError::InvalidType(input.ty.clone()));
    };

    if input.stdin {
        hash_object_from_stdin(input.write, &ty)
    } else if input.stdin_paths {
        let mut output = String::new();
        for line in std::io::BufReader::new(std::io::stdin()).lines() {
            let path = line.map_err(|e| {
                HashObjectError::IoError(format!("Failed to read line from stdin: {}", e))
            })?;
            let output_line = format!("{}\n", hash_object_from_file(input.write, &ty, &path)?);
            output.push_str(&output_line);
        }
        Ok(output)
    } else {
        let mut output = String::new();
        for path in &input.file {
            let output_line = format!(
                "{}\n",
                hash_object_from_file(input.write, &ty, path.to_str().unwrap())?
            );
            output.push_str(&output_line);
        }
        Ok(output.trim_end_matches('\n').to_string())
    }
}

fn hash_object_from_file(
    write: bool,
    ty: &ObjectType,
    file_path: &str,
) -> Result<String, HashObjectError> {
    let content = fs::read(file_path).map_err(|e| {
        HashObjectError::IoError(format!("Failed to read file {}: {}", file_path, e))
    })?;
    let obj = Object::new(*ty, content.clone());

    let hash = if write {
        obj.persist()
            .map_err(|e| HashObjectError::IoError(format!("Failed to persist object: {}", e)))?
    } else {
        let header = format!("{} {}\0", ty.as_str(), content.len());
        let store_content = [header.as_bytes(), &content].concat();
        let mut sha = Sha1::new();
        sha.update(&store_content);
        sha.finalize().to_vec()
    };

    Ok(hash_to_string(&hash))
}

fn hash_object_from_stdin(write: bool, ty: &ObjectType) -> Result<String, HashObjectError> {
    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|e| HashObjectError::IoError(format!("Failed to read from stdin: {}", e)))?;

    let object = Object::new(*ty, buffer.clone());

    let hash = if write {
        object
            .persist()
            .map_err(|e| HashObjectError::IoError(format!("Failed to persist object: {}", e)))?
    } else {
        let header = format!("{} {}\0", ty.as_str(), buffer.len());
        let store_content = [header.as_bytes(), &buffer].concat();
        let mut sha = Sha1::new();
        sha.update(&store_content);
        sha.finalize().to_vec()
    };

    Ok(hash_to_string(&hash))
}
