#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
mod shared;

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
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    eprintln!("Logs from your program will appear here!");

    let args = Cli::parse();
    match args.command {
        Commands::Init => {
            commands::init::init();
        }
        Commands::CatFile { pretty_print, ty, size, exists, hash } => {
            if pretty_print {
                commands::cat_file::cat_file("p", &hash);
            } else if ty {
                commands::cat_file::cat_file("t", &hash);
            } else if size {
                commands::cat_file::cat_file("s", &hash);
            } else if exists {
                commands::cat_file::cat_file("e", &hash);
            } else {
                println!("No flag provided for cat-file command");
            }
        },
        Commands::HashObject { write, ty, stdin, stdin_paths, file } => {
            commands::hash_object::hash_object(write, ty, stdin, stdin_paths, &file);
        },
        Commands::LsTree { name_only, sha } => {
            commands::ls_tree::ls_tree(name_only, &sha);
        }
    }
}
