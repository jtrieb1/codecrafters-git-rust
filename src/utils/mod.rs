pub struct CwdGuard {
    previous: std::path::PathBuf,
}

impl CwdGuard {
    pub fn set_to(path: &std::path::Path) -> Self {
        let previous = std::env::current_dir().expect("Failed to read current directory");
        std::env::set_current_dir(path).expect("Failed to set current directory");
        Self { previous }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous);
    }
}
