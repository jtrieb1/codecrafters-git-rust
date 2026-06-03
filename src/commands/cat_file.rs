use crate::shared::{ blob::Blob, object::Object, tree::Tree };

pub fn cat_file(flag: &str, hash: &str) {
    if flag == "p" {
        let object = Object::from_hash(hash).unwrap();
        if let Ok(blob) = Blob::try_from(&object) {
            print!("{}", String::from_utf8_lossy(blob.content()));
        } else if let Ok(tree) = Tree::try_from(&object) {
            println!("{} {}\0", object.object_type().as_str(), object.size());
            for entry in tree.entries {
                println!("{} {} {}", entry.mode, entry.name, entry.hash);
            }
        } else {
            print!("Unsupported object type: {}", object.object_type().as_str());
        }
    } else if flag == "t" {
        let object = Object::from_hash(hash).unwrap();
        print!("{}", object.object_type().as_str());
    } else if flag == "s" {
        let object = Object::from_hash(hash).unwrap();
        print!("{}", object.size());
    } else if flag == "e" {
        let object_path = Object::hash_to_object_path(hash);
        if std::fs::metadata(object_path).is_ok() {
            print!("exists");
        } else {
            print!("does not exist");
        }
    } else {
        print!("unknown flag: {}", flag);
    }
}