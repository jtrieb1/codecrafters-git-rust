#[allow(dead_code)] // Temporary until we implement the missing_ok functionality
pub struct WriteTreeInput {
    pub missing_ok: bool,
    pub prefix: Option<String>,
}
