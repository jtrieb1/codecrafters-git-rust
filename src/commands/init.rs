use std::fs;

pub fn init() -> Result<(), String> {
    fs::create_dir_all(".git").map_err(|e| format!("Failed to create .git directory: {}", e))?;
    fs::create_dir_all(".git/hooks")
        .map_err(|e| format!("Failed to create .git/hooks directory: {}", e))?;
    fs::create_dir_all(".git/info")
        .map_err(|e| format!("Failed to create .git/info directory: {}", e))?;
    fs::create_dir_all(".git/objects")
        .map_err(|e| format!("Failed to create .git/objects directory: {}", e))?;
    fs::create_dir_all(".git/objects/info")
        .map_err(|e| format!("Failed to create .git/objects/info directory: {}", e))?;
    fs::create_dir_all(".git/objects/pack")
        .map_err(|e| format!("Failed to create .git/objects/pack directory: {}", e))?;
    fs::create_dir_all(".git/refs")
        .map_err(|e| format!("Failed to create .git/refs directory: {}", e))?;
    fs::create_dir_all(".git/refs/heads")
        .map_err(|e| format!("Failed to create .git/refs/heads directory: {}", e))?;
    fs::create_dir_all(".git/refs/tags")
        .map_err(|e| format!("Failed to create .git/refs/tags directory: {}", e))?;
    fs::create_dir_all(".git/refs/remotes")
        .map_err(|e| format!("Failed to create .git/refs/remotes directory: {}", e))?;
    fs::write(".git/HEAD", "ref: refs/heads/main\n")
        .map_err(|e| format!("Failed to write .git/HEAD: {}", e))?;
    fs::write(
        ".git/config",
        "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n",
    )
    .map_err(|e| format!("Failed to write .git/config: {}", e))?;
    fs::write(
        ".git/description",
        "Unnamed repository; edit this file to name it.\n",
    )
    .map_err(|e| format!("Failed to write .git/description: {}", e))?;

    Ok(())
}
