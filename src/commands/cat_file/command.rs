use crate::shared::{ object::Object, blob::Blob, tree::Tree, hash::hash_from_string };
use super::input::CatFileInput;
use super::errors::CatFileError;

fn check_exists(hash: &[u8]) -> bool {
    let object_path = Object::hash_to_object_path(hash);
    std::fs::metadata(object_path).is_ok()
}

fn pretty_print(object: &Object) {
    if let Ok(blob) = Blob::try_from(object) {
        blob.print_content();
    } else if let Ok(tree) = Tree::try_from(object) {
        tree.pretty_print();
    } else {
        print!("Unsupported object type: {}", object.object_type().as_str());
    }
}

pub fn cat_file(input: CatFileInput) -> Result<(), CatFileError> {
    let flag = input.as_flag().ok_or(CatFileError::NoFlagProvided)?;
    let hash = hash_from_string(&input.hash);

    if flag == "e" {
        // Handle this up-front since we don't need to read the object data
        if check_exists(&hash) {
            print!("exists");
        } else {
            print!("does not exist");
        }
        return Ok(());
    }

    let object = Object::try_from_hash(&hash).map_err(|_| CatFileError::UnknownFlag(format!("Object not found: {}", input.hash)))?;

    if flag == "p" {
        pretty_print(&object);
    } else if flag == "t" {
        print!("{}", object.object_type().as_str());
    } else if flag == "s" {
        print!("{}", object.size());
    } else {
        print!("unknown flag: {}", flag);
        return Err(CatFileError::UnknownFlag(flag.to_string()));
    }

    Ok(())
}