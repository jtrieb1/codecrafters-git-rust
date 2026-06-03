use crate::shared::tree::Tree;
use super::{errors::WriteTreeError, input::WriteTreeInput};

pub fn write_tree(input: WriteTreeInput) -> Result<(), WriteTreeError> {
    let dir = input.prefix.unwrap_or_else(|| ".".to_string());
    let tree = Tree::from_directory(&dir).map_err(|e| WriteTreeError::IoError(format!("Failed to read directory: {}", e)))?;
    println!("{}", tree.persist().map_err(|e| WriteTreeError::IoError(format!("Failed to persist tree: {}", e)))?.iter().map(|b| format!("{:02x}", b)).collect::<String>());
    Ok(())
}