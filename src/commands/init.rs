use std::fs;

pub fn init() -> Result<(), String> {
    fs::create_dir(".git").map_err(|e| e.to_string())?;
    fs::create_dir(".git/objects").map_err(|e| e.to_string())?;
    fs::create_dir(".git/refs").map_err(|e| e.to_string())?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n").map_err(|e| e.to_string())?;
    println!("Initialized git directory");
    Ok(())
}