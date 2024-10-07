use crate::commands::config;
use crate::commands::hash_object;
use crate::commands::init;
use crate::commands::ls_tree;
use crate::commands::{cat_file, write_tree};
use crate::commands::{Commands, Git};
use clap::Parser;
use commands::commit_tree;
use std::process::ExitCode;

mod commands;
mod commit;
mod object;
mod tag;
mod util;

fn main() -> ExitCode {
    env_logger::init();

    let git = Git::parse();

    let result = match git.command {
        Commands::Init(args) => init::init_command(args),
        Commands::CatFile(args) => cat_file::cat_file_command(args),
        Commands::HashObject(args) => hash_object::hash_object_command(args),
        Commands::Config(args) => config::config_command(args),
        Commands::LsTree(args) => ls_tree::ls_tree_command(args),
        Commands::WriteTree => write_tree::write_tree_command(),
        Commands::CommitTree(args) => commit_tree::commit_tree_command(args),
        Commands::Clone(_) => todo!(),
    };

    let code = match result {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{e}");
            128
        }
    };

    ExitCode::from(code)
}
