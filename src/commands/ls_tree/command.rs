use super::{ errors::LsTreeError, input::LsTreeInput };
use crate::shared::{ hash::hash_from_string, object::Object, tree::Tree };

pub fn ls_tree(input: LsTreeInput) -> Result<(), LsTreeError> {
    let sha_bytes = hash_from_string(&input.sha);
    let object = Object::try_from_hash(&sha_bytes).map_err(|_| LsTreeError::ObjectNotFound(input.sha.clone()))?;
    if let Ok(tree) = Tree::try_from(&object) {
        if input.name_only {
            tree.print_names();
        } else {
            tree.print_entries();
        }
    } else {
        return Err(LsTreeError::NotATree(input.sha.clone()));
    }
    Ok(())
}