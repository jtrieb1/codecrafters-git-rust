pub struct GitCloneInput {
    pub local: bool,
    pub repository_location: String,
    pub destination_path: Option<String>,
}