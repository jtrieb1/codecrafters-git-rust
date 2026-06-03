use crate::shared::object::Object;

pub fn cat_file(flag: &str, hash: &str) {
    if flag == "p" {
        let object = Object::from_hash(hash).unwrap();
        print!("{}", String::from_utf8_lossy(&object.content()));
    } else if flag == "t" {
        let object = Object::from_hash(hash).unwrap();
        print!("{}", object.object_type());
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