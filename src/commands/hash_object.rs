use crate::shared::object::{Object, ObjectType};

use std::fs;
use std::io::{ BufRead, Read };
use std::path::{Path, PathBuf};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Sha1, Digest};

pub fn hash_object_from_file(write: bool, ty: &str, file_path: &str) {
    let content = fs::read(file_path).unwrap();

    let header = format!("{} {}\0", ty, content.len());

    let store_content = [header.as_bytes(), &content].concat();

    let mut sha = Sha1::new();
    sha.update(&store_content);
    let hash = &sha.finalize()[..];
    let hash = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    if write {
        let object_path = Object::hash_to_object_path(&hash);
        fs::create_dir_all(Path::new(&object_path).parent().unwrap()).unwrap();
        let mut encoder = ZlibEncoder::new(fs::File::create(object_path).unwrap(), Compression::default());
        std::io::copy(&mut std::io::Cursor::new(store_content), &mut encoder).unwrap();
        encoder.finish().unwrap();
    }
    println!("{}", hash);
}

pub fn hash_object_from_stdin(write: bool, ty: &str) {
    let mut buffer = Vec::new();
    std::io::stdin().read_to_end(&mut buffer).unwrap();

    let header = format!("{} {}\0", ty, buffer.len());

    let store_content = [header.as_bytes(), &buffer].concat();

    let mut sha = Sha1::new();
    sha.update(&store_content);
    let hash = &sha.finalize()[..];
    let hash = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    if write {
        let object_path = Object::hash_to_object_path(&hash);
        fs::create_dir_all(Path::new(&object_path).parent().unwrap()).unwrap();
        let mut encoder = ZlibEncoder::new(fs::File::create(object_path).unwrap(), Compression::default());
        std::io::copy(&mut std::io::Cursor::new(store_content), &mut encoder).unwrap();
        encoder.finish().unwrap();
    }
    println!("{}", hash);
    
}

pub fn hash_object(write: bool, ty: String, stdin: bool, stdin_paths: bool, file_paths: &Vec<PathBuf>) {
    if ObjectType::from_str(&ty).is_none() {
        println!("Invalid type: {}", ty);
        return;
    }

    if stdin {
        hash_object_from_stdin(write, &ty);
    } else if stdin_paths {
        for line in std::io::BufReader::new(std::io::stdin()).lines() {
            let path = line.unwrap();
            hash_object_from_file(write, &ty, &path);
        }
    } else {
        for path in file_paths {
            hash_object_from_file(write, &ty, path.to_str().unwrap());
        }
    };
}