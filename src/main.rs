#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;

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
        #[arg(short, long)]
        pretty_print: bool,
        #[arg(short, long)]
        ty: bool,
        #[arg(short, long)]
        size: bool,
        #[arg(short, long)]
        exists: bool,
        hash: String,
    },
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
        }
    }
}
