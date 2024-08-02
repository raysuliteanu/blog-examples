use crate::commands::cat_file::CatFileArgs;
use crate::commands::config::ConfigArgs;
use crate::commands::hash_object::HashObjectArgs;
use crate::commands::init::InitArgs;
use clap::{Parser, Subcommand};

pub(crate) mod cat_file;
pub(crate) mod config;
pub(crate) mod hash_object;
pub(crate) mod init;

#[derive(Debug, Parser)]
pub(crate) struct Git {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Init(InitArgs),
    CatFile(CatFileArgs),
    HashObject(HashObjectArgs),
    Config(ConfigArgs),
}
