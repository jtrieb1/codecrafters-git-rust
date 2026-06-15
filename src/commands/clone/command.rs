use std::collections::HashSet;
use futures_util::StreamExt;

use crate::commands::init::init;
use crate::shared::pack::PackFile;
use crate::utils::CwdGuard;
use crate::commands::checkout::{ input::CheckoutInput, command::checkout };
use crate::commands::ls_remote::{ command::ls_remote, input::LsRemoteInput };

use super::{errors::CloneError, input::GitCloneInput};

use reqwest::ClientBuilder;

pub async fn clone(input: GitCloneInput) -> Result<String, CloneError> {
    // Get repo and destination path from input
    let repository_location = input.repository_location.trim_end_matches("/");
    let destination_path = input.destination_path.unwrap_or_else(|| ".".to_string());

    // Ensure destination path exists
    if !std::path::Path::new(&destination_path).exists() {
        if let Err(e) = std::fs::create_dir_all(&destination_path) {
            return Err(CloneError::NetworkError(format!(
                "Failed to create destination directory: {}",
                e
            )));
        }
    }

    // Are we cloning from a local path or a remote URL?
    if input.local {
        clone_local(repository_location, &destination_path)?;
    } else {
        // For remote repositories, we first need to fetch the refs, then use that to fetch the packfile, then unpack the objects, and finally checkout the default branch.

        let refs = fetch_refs(repository_location).await?;

        let packfile_bytes = fetch_packfile(&refs, repository_location).await?;

        // Now we need to unpack the PACK file and write the objects to the .git/objects directory
        let packfile = PackFile::from_bytes(&packfile_bytes).map_err(|e| CloneError::NetworkError(format!("Failed to parse packfile: {}", e)))?;
        let objects = packfile.unpack_objects().map_err(|e| CloneError::NetworkError(format!("Failed to unpack packfile: {}", e)))?;

        let _cwd_guard = CwdGuard::set_to(std::path::Path::new(&destination_path));
        init().map_err(|e| CloneError::NetworkError(format!("Failed to initialize local repository: {}", e)))?;

        // Set up our refs based on the refs we got from ls-remote
        for rref in refs {
            let ref_path = std::path::Path::new(".git").join("refs").join("remotes").join("origin").join(rref.1.trim_start_matches("refs/heads/"));
            if let Some(parent) = ref_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| CloneError::NetworkError(format!("Failed to create directory for ref: {}", e)))?;
            }
            std::fs::write(ref_path, rref.0).map_err(|e| CloneError::NetworkError(format!("Failed to write ref: {}", e)))?;
        }

        for obj in &objects {
            obj.persist().map_err(|e| CloneError::NetworkError(format!("Failed to write object to local repository: {}", e)))?;
        }

        // Now that all of the objects are in the .git folder, we just have to checkout the default branch to set up the working directory
        let checkout_input = CheckoutInput {
            committish: "origin/HEAD".to_string(), // This will checkout the default branch
        };
        checkout(checkout_input).map_err(|e| CloneError::NetworkError(format!("Failed to checkout default branch: {}", e)))?;
    }
    
    Ok("Cloned local repository successfully".to_string())
}

pub fn clone_local(repo_location: &str, destination_path: &str) -> Result<(), CloneError> {
    if std::path::Path::new(repo_location).exists() {
        // We could just copy the entire directory, but let's be more efficient and only copy the .git directory
        let source_git_dir = std::path::Path::new(repo_location).join(".git");
        let destination_git_dir = std::path::Path::new(destination_path).join(".git");

        if let Err(e) = std::fs::create_dir_all(&destination_git_dir) {
            return Err(CloneError::NetworkError(format!(
                "Failed to create destination directory: {}",
                e
            )));
        }

        if let Err(e) = std::fs::copy(&source_git_dir, &destination_git_dir) {
            return Err(CloneError::NetworkError(format!(
                "Failed to copy .git directory: {}",
                e
            )));
        }

        // Now we need to set up the working directory by checking out the default branch
        let _cwd_guard = CwdGuard::set_to(std::path::Path::new(destination_path));

        let checkout_input = CheckoutInput {
            committish: "HEAD".to_string(), // This will checkout the default branch
        };
        if let Err(e) = checkout(checkout_input) {
            return Err(CloneError::NetworkError(format!(
                "Failed to checkout default branch: {}",
                e
            )));
        }
    } else {
        return Err(CloneError::RepositoryNotFound(format!(
            "Local repository not found at path: {}",
            repo_location
        )));
    }
    Ok(())
}

async fn fetch_refs(repository_location: &str) -> Result<Vec<(String, String)>, CloneError> {
    let ls_remote_input = LsRemoteInput {
        repository: Some(repository_location.to_string()),
    };

    let refs = match ls_remote(ls_remote_input).await {
        Ok(refs) => refs,
        Err(e) => {
            return Err(CloneError::NetworkError(format!(
                "Failed to fetch refs from remote repository: {}",
                e
            )));
        }
    };

    // Split refs into lines and collect them as pairs of (sha1, refname)
    let refs: Vec<(String, String)> = refs.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                ("".to_string(), "".to_string())
            }
        })
        .collect();

    Ok(refs)
}

async fn fetch_packfile(refs: &Vec<(String, String)>, repository_location: &str) -> Result<Vec<u8>, CloneError> {
    let mut advertised = HashSet::new();
    for (sha1, refname) in refs {
        if !sha1.is_empty() && !refname.is_empty() {
            advertised.insert(sha1.clone());
        }
    }

    let want = advertised.iter().cloned().collect::<Vec<String>>();

    let request_packet = build_request_packet(&want);

    let client = ClientBuilder::new()
            .build()
            .expect("Failed to build HTTP client");

    // Make request
    let resp = client.post(format!("{}/git-upload-pack", &repository_location))
        .header("Content-Type", "application/x-git-upload-pack-request")
        .header("Accept", "application/x-git-upload-pack-result")
        .body(request_packet)
        .send()
        .await
        .map_err(|e| CloneError::NetworkError(format!("Failed to send request: {}", e)))?;

    let mut stream = resp.bytes_stream();
    let mut relevant_bytes = vec![];
    let mut byte_buffer = vec![];

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                process_packfile_chunk(&mut relevant_bytes, &mut byte_buffer, &bytes);
            }
            Err(e) => {
                eprintln!("Error while receiving packfile chunk: {}", e);
                return Err(CloneError::NetworkError(format!("Failed to receive packfile chunk: {}", e)));
            }
        }
    }

    Ok(relevant_bytes)
}

fn build_request_packet(want: &Vec<String>) -> String {
    let mut request_packet = String::new();
    let mut first = true;
    for sha1 in want {
        if first {
            let mut line = format!("want {}", sha1);
            line.push_str(" multi_ack_detailed side-band-64k thin-pack ofs-delta\x0A");
            // Calculate hex length of line and prepend it as a 4-digit hex number
            let line_len = line.len() + 4; // +4 for the length prefix itself
            let line_len_hex = format!("{:04x}", line_len);
            request_packet.push_str(&line_len_hex);
            request_packet.push_str(&line);
            first = false;
        } else {
            request_packet.push_str(&format!("0032want {}\x0A", sha1));
        }
    }
    request_packet.push_str("0000");
    request_packet.push_str("0009done\x0A");
    request_packet
}

fn process_packfile_chunk(relevant_bytes: &mut Vec<u8>, byte_buffer: &mut Vec<u8>, chunk: &[u8]) {
    // This function processes a chunk of bytes from the response stream and extracts the relevant bytes that belong to the PACK file, ignoring any progress messages or error messages.
    // The logic is the same as in read_packfile_response, but we need to handle the case where a packet might be split across multiple chunks, so we need to keep track of any leftover bytes that we haven't processed yet.

    let mut chunk_bytes = byte_buffer.clone();
    chunk_bytes.extend_from_slice(chunk);

    loop {
        if chunk_bytes.len() < 4 {
            // Not enough bytes to read the length prefix, we need to wait for the next chunk
            break;
        }
        let len_prefix = &chunk_bytes[..4];
        let len = usize::from_str_radix(std::str::from_utf8(len_prefix).unwrap_or("0000"), 16).unwrap_or(0);
        if len == 0 {
            // This is a flush packet, we can ignore it
            chunk_bytes.drain(..4);
            continue;
        }
        if chunk_bytes.len() < len {
            // Not enough bytes to read the full packet, we need to wait for the next chunk
            break;
        }
        let packet = &chunk_bytes[4..len];
        let channel = packet[0];
        let data = &packet[1..];
        if channel == 1 {
            // This is pack data, we want to keep it
            relevant_bytes.extend_from_slice(data);
        } else if channel == 2 {
            // This is a progress message, we can ignore it for now but we might want to display it to the user in the future
            let progress_message = std::str::from_utf8(data).unwrap_or("");
            println!("Progress: {}", progress_message);
        } else if channel == 3 {
            // This is an error message, we should probably handle it more gracefully but for now we'll just print it to stderr
            let error_message = std::str::from_utf8(data).unwrap_or("");
            eprintln!("Error from server: {}", error_message);
        }
        chunk_bytes.drain(..len);
    }
    // Whatever bytes are left in chunk_bytes after processing all complete packets should be saved in the byte_buffer to be processed with the next chunk
    byte_buffer.clear();
    byte_buffer.extend_from_slice(&chunk_bytes);
}