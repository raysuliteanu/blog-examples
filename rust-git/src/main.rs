use std::io;

use clap::Parser;

use crate::commands::cat_file;
use crate::commands::config;
use crate::commands::hash_object;
use crate::commands::init;
use crate::commands::{Commands, Git};

mod commands;
mod util;

fn main() -> io::Result<()> {
    env_logger::init();

    let git = Git::parse();

    match git.command {
        Commands::Init(args) => init::init_command(args),
        Commands::CatFile(args) => cat_file::cat_file_command(args),
        Commands::HashObject(args) => hash_object::hash_object_command(args),
        Commands::Config(args) => config::config_command(args),
    }
}
