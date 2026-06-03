pub fn ls_tree(name_only: bool, sha: &str) {
    let object = crate::shared::object::Object::from_hash(sha).unwrap();
    if let Ok(tree) = crate::shared::tree::Tree::try_from(&object) {
        for entry in tree.entries {
            if name_only {
                println!("{}", entry.name);
            } else {
                println!("{} {} {}", entry.mode, entry.name, entry.hash);
            }
        }
    } else {
        println!("Object {} is not a tree", sha);
    }
}