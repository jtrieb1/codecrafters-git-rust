use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod shared;

use commands::{ 
    init::init, 
    cat_file::{command::cat_file, input::CatFileInput}, 
    hash_object::{command::hash_object, input::HashObjectInput}, 
    ls_tree::{command::ls_tree, input::LsTreeInput}, 
    write_tree::{command::write_tree, input::WriteTreeInput}
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
        hash: String,
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
    }
}

fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();
    match args.command {
        Commands::Init => {
            init().map_err(|e| anyhow::anyhow!(e))
        }
        Commands::CatFile { pretty_print, ty, size, exists, hash } => {
            let input = CatFileInput {
                pretty_print,
                ty,
                size,
                exists,
                hash,
            };

            if let Err(e) = input.validate() {
                return Err(anyhow::anyhow!(e));
            }

            cat_file(input).map_err(|e| anyhow::anyhow!(e))
        },
        Commands::HashObject { write, ty, stdin, stdin_paths, file } => {
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
            hash_object(input).map_err(|e| anyhow::anyhow!(e))
        },
        Commands::LsTree { name_only, sha } => {
            ls_tree(LsTreeInput {
                name_only,
                sha,
            }).map_err(|e| anyhow::anyhow!(e))
        },
        Commands::WriteTree { missing_ok, prefix } => {
            let input = WriteTreeInput {
                missing_ok,
                prefix,
            };
            write_tree(input).map_err(|e| anyhow::anyhow!(e))
        }
    }
}
