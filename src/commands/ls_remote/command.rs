use super::{ errors::LsRemoteError, input::LsRemoteInput };
use reqwest::get;

pub async fn ls_remote(input: LsRemoteInput) -> Result<String, LsRemoteError> {
    if input.repository.is_none() {
        return Err(LsRemoteError::RepositoryNotFound(
            "No repository specified".to_string(),
        ));
    }

    let repository = input.repository.unwrap(); // An HTTP URL to the repository
    let url = format!("{}/info/refs?service=git-upload-pack", repository);

    let resp = match get(&url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.text().await {
                    Ok(text) => Ok(text),
                    Err(_) => Err(LsRemoteError::NetworkError(
                        "Failed to read response body".to_string(),
                    )),
                }
            } else {
                Err(LsRemoteError::RepositoryNotFound(format!(
                    "Repository not found at URL: {}",
                    repository
                )))
            }
        }
        Err(e) => Err(LsRemoteError::NetworkError(
            format!("Failed to connect to repository: {}", e),
        )),
    };

    // Parse response to extract refs
    resp.and_then(|body| {
        // Body starts with a 4-byte length prefix, followed by '# service=git-upload-pack\n', then a 4-byte '0000' separator, and then the refs
        let mut lines: Vec<&str> = body.split('\n').collect();
        // First line is the length prefix and the service announcement, we can ignore it
        // There's a '0000' string at the beginning of the remaining data that we can also ignore
        if lines.len() < 2 || !lines[1].starts_with("0000") {
            return Err(LsRemoteError::NetworkError(
                "Unexpected response format".to_string(),
            ));
        }

        lines[1] = &lines[1][4..]; // Remove the '0000' prefix from the second line

        // The rest are the refs
        let refs = lines[1..]
            .iter()
            .filter(|line| !line.is_empty())
            .map(|line| {
                // Each line starts with a 4 digit length prefix, followed by the ref info
                let ref_info = &line[4..]; // Skip the length prefix
                // The ref info is in the format: <sha1> <refname>\0<capabilities>
                // We only care about the <sha1> and <refname>
                let parts: Vec<&str> = ref_info.split_whitespace().collect();
                if parts.len() >= 2 {
                    let sha1 = parts[0];
                    let refname = parts[1];
                    // Trim the trailing capabilities if present (after a null byte)
                    let refname = refname.split('\0').next().unwrap_or(refname);
                    Some(format!("{}\t{}", sha1, refname))
                } else {                    
                    None
                }
            })
            .filter_map(|x| x)
            .collect::<Vec<String>>()
            .join("\n");
        Ok(refs)
    })
}