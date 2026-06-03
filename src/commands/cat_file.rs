use crate::shared::object::Object;

pub fn cat_file(flag: &str, hash: &str) {
    if flag == "p" {
        let object = Object::from_hash(hash).unwrap();
        println!("{}", String::from_utf8_lossy(&object.content()));
    } else if flag == "t" {
        let object = Object::from_hash(hash).unwrap();
        println!("{}", object.object_type());
    } else if flag == "s" {
        let object = Object::from_hash(hash).unwrap();
        println!("{}", object.size());
    } else if flag == "e" {
        let object_path = Object::hash_to_object_path(hash);
        if std::fs::metadata(object_path).is_ok() {
            println!("exists");
        } else {
            println!("does not exist");
        }
    } else {
        println!("unknown flag: {}", flag);
    }
}