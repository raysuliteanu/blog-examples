use clap::Parser;
use std::io;

use crate::commands::cat_file::cat_file_command;
use crate::commands::config::config_command;
use crate::commands::hash_object::hash_object_command;
use crate::commands::init::init_command;
use crate::commands::{Commands, Git};

mod commands;
mod util;

fn main() -> io::Result<()> {
    env_logger::init();

    let git = Git::parse();

    match git.command {
        Commands::Init(args) => init_command(args),
        Commands::CatFile(args) => cat_file_command(args),
        Commands::HashObject(args) => hash_object_command(args),
        Commands::Config(args) => config_command(args),
    }
}
