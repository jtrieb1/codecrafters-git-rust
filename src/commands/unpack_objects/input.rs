#[allow(dead_code)]

pub struct UnpackObjectsInput {
    pub dry_run: bool,
    pub best_effort: bool,
    pub strict: bool,
    pub max_input_size: usize,
    pub packfile_path: String,
}
