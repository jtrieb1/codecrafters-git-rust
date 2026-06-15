use crate::shared::refs::resolve_ref_name_to_full_ref;

pub struct CheckoutInput {
    pub committish: String,
}

impl CheckoutInput {
    pub fn validate(&self) -> Result<(), String> {
        // Committish could be one of the following:
        // - A full 40-character commit hash
        // - A branch name (which we would need to resolve to a commit hash)
        // - A tag name (which we would also need to resolve to a commit hash)

        // First check if we've got an active ref that matches the committish (for branch names)
        if let Ok(ref_path) = resolve_ref_name_to_full_ref(&self.committish) {
            if std::fs::metadata(&format!(".git/{}", ref_path)).is_ok() {
                return Ok(());
            }
        }

        // If not a ref, check if it's a valid 40-character commit hash
        if self.committish.len() == 40 && self.committish.chars().all(|c| c.is_digit(16)) {
            return Ok(());
        }

        Err(format!("Invalid committish: {}", self.committish))
    }
}
