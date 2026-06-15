use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod shared;
mod utils;

use commands::{
    cat_file::{command::cat_file, input::CatFileInput},
    commit_tree::{command::commit_tree, input::CommitTreeInput},
    hash_object::{command::hash_object, input::HashObjectInput},
    init::init,
    ls_tree::{command::ls_tree, input::LsTreeInput},
    write_tree::{command::write_tree, input::WriteTreeInput},
};

use crate::commands::{
    checkout::command::checkout, ls_remote::{command::ls_remote, input::LsRemoteInput}, tag::command::tag, unpack_objects::command::unpack_objects
};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    #[command(name = "cat-file")]
    CatFile {
        #[arg(short, long, conflicts_with_all = &["ty", "size", "exists"])]
        pretty_print: bool,
        #[arg(short, long, conflicts_with_all = &["pretty_print", "size", "exists"])]
        ty: bool,
        #[arg(short, long, conflicts_with_all = &["pretty_print", "ty", "exists"])]
        size: bool,
        #[arg(short, long, conflicts_with_all = &["pretty_print", "ty", "size"])]
        exists: bool,
        object: String,
    },
    HashObject {
        #[arg(short, long)]
        write: bool,
        #[arg(short = 't', default_value = "blob")]
        ty: String,
        #[arg(long, conflicts_with = "stdin_paths")]
        stdin: bool,
        #[arg(long, conflicts_with_all = &["stdin", "file", "path"])]
        stdin_paths: bool,
        #[arg(
            value_name = "file",
            required_unless_present_any = &["stdin", "stdin_paths"],
            conflicts_with = "stdin_paths"
        )]
        file: Vec<PathBuf>,
    },
    LsTree {
        #[arg(long)]
        name_only: bool,
        sha: String,
    },
    WriteTree {
        #[arg(long)]
        missing_ok: bool,
        #[arg(long)]
        prefix: Option<String>,
    },
    CommitTree {
        #[arg(short, long)]
        message: String,
        #[arg(short, long, action=clap::ArgAction::Append)]
        parent: Vec<String>,
        tree: String,
    },
    Checkout {
        commit_hash: String,
    },
    UnpackObjects {
        #[arg(long, short = 'n')]
        dry_run: bool,
        #[arg(long, short = 'r')]
        best_effort: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long, default_value_t = 256 * 1024 * 1024)]
        max_input_size: usize,
        packfile_path: String,
    },
    Tag {
        #[arg(short)]
        annotated: bool,
        #[arg(short)]
        delete: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(short, long, conflicts_with = "file")]
        message: Option<String>,
        #[arg(short, long, conflicts_with = "message")]
        file: Option<String>,
        tag_name: String,
        object: String,
    },
    LsRemote {
        repository: String,
    },
    Clone {
        #[arg(short, long)]
        local: bool,
        repository_location: String,
        destination_path: Option<String>,
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();
    match args.command {
        Commands::Init => init().map_err(|e| anyhow::anyhow!(e)),
        Commands::CatFile {
            pretty_print,
            ty,
            size,
            exists,
            object,
        } => {
            let input = CatFileInput {
                pretty_print,
                ty,
                size,
                exists,
                object,
            };

            if let Err(e) = input.validate() {
                return Err(anyhow::anyhow!(e));
            }

            println!("{}", cat_file(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::HashObject {
            write,
            ty,
            stdin,
            stdin_paths,
            file,
        } => {
            let input = HashObjectInput {
                write,
                ty,
                stdin,
                stdin_paths,
                file,
            };
            if let Err(e) = input.validate() {
                return Err(anyhow::anyhow!(e));
            }
            println!("{}", hash_object(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::LsTree { name_only, sha } => {
            println!(
                "{}",
                ls_tree(LsTreeInput { name_only, sha }).map_err(|e| anyhow::anyhow!(e))?
            );
            Ok(())
        }
        Commands::WriteTree { missing_ok, prefix } => {
            let input = WriteTreeInput { missing_ok, prefix };
            println!("{}", write_tree(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::CommitTree {
            message,
            parent,
            tree,
        } => {
            let input = CommitTreeInput {
                message,
                parents: parent,
                tree,
            };
            if let Err(e) = input.validate() {
                return Err(anyhow::anyhow!(e));
            };

            println!("{}", commit_tree(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::Checkout { commit_hash } => {
            let input = commands::checkout::input::CheckoutInput {
                committish: commit_hash,
            };
            if let Err(e) = input.validate() {
                return Err(anyhow::anyhow!(e));
            };

            println!("{}", checkout(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::UnpackObjects {
            dry_run,
            best_effort,
            strict,
            max_input_size,
            packfile_path,
        } => {
            let input = commands::unpack_objects::input::UnpackObjectsInput {
                dry_run,
                best_effort,
                strict,
                max_input_size,
                packfile_path,
            };
            println!("{}", unpack_objects(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::Tag {
            annotated,
            delete,
            force,
            message,
            file,
            tag_name,
            object,
        } => {
            let input = commands::tag::input::TagInput::from(Commands::Tag {
                annotated,
                delete,
                force,
                message,
                file,
                tag_name,
                object,
            });
            if let Err(e) = match &input {
                commands::tag::input::TagInput::Create(create_input) => create_input.validate(),
                commands::tag::input::TagInput::Delete(delete_input) => delete_input.validate(),
            } {
                return Err(anyhow::anyhow!(e));
            };

            println!("{}", tag(input).map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
        Commands::LsRemote { repository } => {
            let input = LsRemoteInput {
                repository: Some(repository),
            };
            println!("{}", ls_remote(input).await.map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        },
        Commands::Clone { local, repository_location, destination_path } => {
            let input = commands::clone::input::GitCloneInput {
                local,
                repository_location,
                destination_path,
            };
            println!("{}", commands::clone::command::clone(input).await.map_err(|e| anyhow::anyhow!(e))?);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::CwdGuard;
    use serial_test::serial;

    // Integration tests

    #[test]
    #[serial]
    fn test_init() {
        // Just test that it creates the .git directory and the necessary subdirectories
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        init().unwrap();
        assert!(std::path::Path::new(".git").exists());
        assert!(std::path::Path::new(".git/objects").exists());
        assert!(std::path::Path::new(".git/objects/info").exists());
        assert!(std::path::Path::new(".git/objects/pack").exists());
        assert!(std::path::Path::new(".git/refs").exists());
        assert!(std::path::Path::new(".git/refs/heads").exists());
        assert!(std::path::Path::new(".git/refs/tags").exists());
        assert!(std::path::Path::new(".git/refs/remotes").exists());
        assert!(std::path::Path::new(".git/hooks").exists());
        assert!(std::path::Path::new(".git/info").exists());
        assert!(std::path::Path::new(".git/HEAD").exists());
        assert!(std::path::Path::new(".git/config").exists());
        assert!(std::path::Path::new(".git/description").exists());
    }

    #[test]
    #[serial]
    fn test_write_tree() {
        // Test that we can write a tree object and that it has the correct content
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        std::fs::write("file1.txt", "Hello, world!").unwrap();
        std::fs::write("file2.txt", "Goodbye, world!").unwrap();

        let input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };

        let tree_hash_str = write_tree(input).unwrap();
        let tree_hash = shared::hash::hash_from_string(&tree_hash_str);
        let tree_object = shared::object::Object::try_from_hash(&tree_hash).unwrap();
        assert_eq!(tree_object.object_type(), &shared::object::ObjectType::Tree);
        let tree = shared::tree::Tree::try_from(&tree_object).unwrap();
        let entries = tree.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "file1.txt");
        let obj_hash = entries[0].hash.clone();
        let obj = shared::object::Object::try_from_hash(&obj_hash).unwrap();
        assert_eq!(obj.object_type(), &shared::object::ObjectType::Blob);
        assert_eq!(obj.content(), b"Hello, world!");

        assert_eq!(entries[1].name, "file2.txt");
        let obj_hash = entries[1].hash.clone();
        let obj = shared::object::Object::try_from_hash(&obj_hash).unwrap();
        assert_eq!(obj.object_type(), &shared::object::ObjectType::Blob);
        assert_eq!(obj.content(), b"Goodbye, world!");
    }

    #[test]
    #[serial]
    fn test_ls_tree() {
        // Test that we can list the contents of a tree object
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        std::fs::write("file1.txt", "Hello, world!").unwrap();
        std::fs::write("file2.txt", "Goodbye, world!").unwrap();

        let input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };

        let tree_hash_str = write_tree(input).unwrap();

        let ls_tree_input = commands::ls_tree::input::LsTreeInput {
            name_only: false,
            sha: tree_hash_str.clone(),
        };

        let output = ls_tree(ls_tree_input).unwrap();
        let lines: Vec<&str> = output.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("file1.txt"));
        assert!(lines[1].contains("file2.txt"));
    }

    #[test]
    #[serial]
    fn test_commit_tree() {
        // Test that we can create a commit object from a tree and that it has the correct content
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        std::fs::write("file1.txt", "Hello, world!").unwrap();
        std::fs::write("file2.txt", "Goodbye, world!").unwrap();

        let write_tree_input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };

        let tree_hash_str = write_tree(write_tree_input).unwrap();

        let commit_tree_input = commands::commit_tree::input::CommitTreeInput {
            message: "Initial commit".to_string(),
            parents: vec![],
            tree: tree_hash_str.clone(),
        };

        let commit_hash_str = commit_tree(commit_tree_input).unwrap();
        let commit_hash = shared::hash::hash_from_string(&commit_hash_str);
        let commit_object = shared::object::Object::try_from_hash(&commit_hash).unwrap();
        assert_eq!(
            commit_object.object_type(),
            &shared::object::ObjectType::Commit
        );
        let commit = shared::commit::Commit::try_from(&commit_object).unwrap();
        assert_eq!(commit.message(), "Initial commit");
        assert_eq!(
            commit.tree(),
            &shared::hash::hash_from_string(&tree_hash_str)
        );
    }

    #[test]
    #[serial]
    fn test_hash_object() {
        // Test that we can hash a file and that the resulting object has the correct content
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        std::fs::write("file1.txt", "Hello, world!").unwrap();

        let hash_object_input = commands::hash_object::input::HashObjectInput {
            write: true,
            ty: "blob".to_string(),
            stdin: false,
            stdin_paths: false,
            file: vec![PathBuf::from("file1.txt")],
        };

        let hash_str = hash_object(hash_object_input).unwrap();
        let hash = shared::hash::hash_from_string(&hash_str);
        let obj = shared::object::Object::try_from_hash(&hash).unwrap();
        assert_eq!(obj.object_type(), &shared::object::ObjectType::Blob);
        assert_eq!(obj.content(), b"Hello, world!");
    }

    #[test]
    #[serial]
    fn test_checkout() {
        // Test that we can checkout a commit and that it updates the working directory
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        std::fs::write("file1.txt", "Hello, world!").unwrap();

        let write_tree_input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };

        let tree_hash_str = write_tree(write_tree_input).unwrap();

        let commit_tree_input = commands::commit_tree::input::CommitTreeInput {
            message: "Initial commit".to_string(),
            parents: vec![],
            tree: tree_hash_str.clone(),
        };

        let commit_hash_str = commit_tree(commit_tree_input).unwrap();

        // Modify the file and create a new commit
        std::fs::write("file1.txt", "Goodbye, world!").unwrap();

        let write_tree_input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };

        let tree_hash_str2 = write_tree(write_tree_input).unwrap();

        let commit_tree_input = commands::commit_tree::input::CommitTreeInput {
            message: "Second commit".to_string(),
            parents: vec![commit_hash_str.clone()],
            tree: tree_hash_str2.clone(),
        };

        let commit_hash_str2 = commit_tree(commit_tree_input).unwrap();

        // Checkout the first commit
        let checkout_input = commands::checkout::input::CheckoutInput {
            committish: commit_hash_str.clone(),
        };

        checkout(checkout_input).unwrap();
        let content = std::fs::read_to_string("file1.txt").unwrap();
        assert_eq!(content, "Hello, world!");

        // Checkout the second commit
        let checkout_input = commands::checkout::input::CheckoutInput {
            committish: commit_hash_str2.clone(),
        };

        checkout(checkout_input).unwrap();
        let content = std::fs::read_to_string("file1.txt").unwrap();
        assert_eq!(content, "Goodbye, world!");
    }

    #[test]
    #[serial]
    fn test_tag() {
        // Test that we can create a tag and that it points to the correct object
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        std::fs::write("file1.txt", "Hello, world!").unwrap();

        let write_tree_input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };

        let tree_hash_str = write_tree(write_tree_input).unwrap();

        let commit_tree_input = commands::commit_tree::input::CommitTreeInput {
            message: "Initial commit".to_string(),
            parents: vec![],
            tree: tree_hash_str.clone(),
        };

        let commit_hash_str = commit_tree(commit_tree_input).unwrap();

        let tag_input =
            commands::tag::input::TagInput::Create(commands::tag::input::TagCreateInput {
                annotated: true,
                force: false,
                message: Some("This is a tag".to_string()),
                file: None,
                tag_name: "v1.0".to_string(),
                object: commit_hash_str.clone(),
            });

        let tag_hash_str = tag(tag_input).unwrap();
        let tag_hash = shared::hash::hash_from_string(&tag_hash_str);
        let obj = shared::object::Object::try_from_hash(&tag_hash).unwrap();
        assert_eq!(obj.object_type(), &shared::object::ObjectType::Tag);

        let tag = shared::tag::AnnotatedTag::try_from(&obj).unwrap();
        assert_eq!(tag.name, "v1.0");
        assert_eq!(tag.message, "This is a tag");
        assert_eq!(
            tag.object_hash,
            shared::hash::hash_from_string(&commit_hash_str)
        );

        let resolved = tag.resolve_target().unwrap();
        assert_eq!(resolved.object_type(), &shared::object::ObjectType::Commit);
        assert_eq!(
            resolved.content(),
            shared::object::Object::try_from_hash(&shared::hash::hash_from_string(
                &commit_hash_str
            ))
            .unwrap()
            .content()
        );
    }

    #[test]
    #[serial]
    fn test_unpack_objects() {
        // We don't currently have the ability to pack objects ourselves, so
        // we'll have to let the actual Git handle it for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let _cwd_guard = CwdGuard::set_to(temp_dir.path());
        assert!(init().is_ok());

        // Write a few files
        std::fs::write("file1.txt", "Hello, world!").unwrap();
        std::fs::write("file2.txt", "Goodbye, world!").unwrap();

        // Write tree
        let write_tree_input = commands::write_tree::input::WriteTreeInput {
            missing_ok: false,
            prefix: None,
        };
        let tree_hash_str = write_tree(write_tree_input).unwrap();

        // Commit tree
        let commit_tree_input = commands::commit_tree::input::CommitTreeInput {
            message: "Initial commit".to_string(),
            parents: vec![],
            tree: tree_hash_str.clone(),
        };
        commit_tree(commit_tree_input).unwrap();

        // Compile all object hashes into a text file for git to read when creating the packfile
        let mut object_hashes = vec![];
        // Easiest to just read the objects directory and get all the hashes from there since we know that's where they are
        for entry in std::fs::read_dir(".git/objects").unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                let dir_name = entry.file_name().into_string().unwrap();
                if dir_name.len() == 2 {
                    for file_entry in std::fs::read_dir(entry.path()).unwrap() {
                        let file_entry = file_entry.unwrap();
                        if file_entry.file_type().unwrap().is_file() {
                            let file_name = file_entry.file_name().into_string().unwrap();
                            if file_name.len() == 38 {
                                object_hashes.push(format!("{}{}", dir_name, file_name));
                            }
                        }
                    }
                }
            }
        }

        // Create a packfile using Git
        std::fs::create_dir_all(".git/objects/pack").unwrap();
        let mut child = std::process::Command::new("git")
            .args(["pack-objects", ".git/objects/pack/pack-test"])
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to start git pack-objects");

        {
            let stdin = child
                .stdin
                .as_mut()
                .expect("Failed to open stdin for git pack-objects");
            let hash_input = format!("{}\n", object_hashes.join("\n"));
            std::io::Write::write_all(stdin, hash_input.as_bytes())
                .expect("Failed to write object hashes to git pack-objects");
        }

        let output = child
            .wait_with_output()
            .expect("Failed to wait for git pack-objects");
        assert!(
            output.status.success(),
            "git pack-objects failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let mut generated_pack_path: Option<String> = None;
        for entry in std::fs::read_dir(".git/objects/pack").unwrap() {
            let entry = entry.unwrap();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.ends_with(".pack") {
                generated_pack_path = Some(format!(".git/objects/pack/{}", file_name));
            }
        }

        let packfile_path =
            generated_pack_path.expect("No .pack file generated by git pack-objects");

        let unpack_objects_input = commands::unpack_objects::input::UnpackObjectsInput {
            dry_run: false,
            best_effort: false,
            strict: false,
            max_input_size: 256 * 1024 * 1024,
            packfile_path,
        };
        let unpack_result = commands::unpack_objects::command::unpack_objects(unpack_objects_input);
        assert!(unpack_result.is_ok());
        let unpack_output = unpack_result.unwrap();
        assert!(unpack_output.contains("Unpacked 4 objects"));
    }
}
