use crate::commands::cat_file::CatFileArgs;
use crate::commands::commit_tree::CommitTreeArgs;
use crate::commands::config::ConfigArgs;
use crate::commands::hash_object::HashObjectArgs;
use crate::commands::init::InitArgs;
use clap::{Parser, Subcommand};
use ls_tree::LsTreeArgs;
use std::io;
use thiserror::Error;

pub(crate) mod cat_file;
pub(crate) mod commit_tree;
pub(crate) mod config;
pub(crate) mod hash_object;
pub(crate) mod init;
pub(crate) mod ls_tree;
pub(crate) mod write_tree;

#[derive(Debug, Parser)]
pub(crate) struct Git {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    /// Create an empty Git repository or reinitialize an existing one
    Init(InitArgs),
    /// Provide contents or details of repository objects
    CatFile(CatFileArgs),
    /// Compute object ID and optionally create an object from a file
    HashObject(HashObjectArgs),
    /// Get and set repository or global options
    Config(ConfigArgs),
    /// List the contents of a tree object
    LsTree(LsTreeArgs),
    /// Create a tree object from the current index
    WriteTree,
    CommitTree(CommitTreeArgs),
}

pub type GitResult<T> = Result<T, GitError>;
pub type GitCommandResult = GitResult<()>;

#[derive(Error, Debug)]
pub(crate) enum GitError {
    #[error("read object failed")]
    ReadObjectError,
    #[error("Not a valid object name {obj_id}")]
    InvalidObjectId { obj_id: String },
    #[error("I/O error")]
    Io {
        #[from]
        source: io::Error,
    },
    #[error("hex conversion error")]
    HexConversionError {
        #[from]
        source: hex::FromHexError,
    },
}
