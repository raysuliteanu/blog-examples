use clap::Args;
use log::trace;

use super::GitCommandResult;

#[derive(Debug, Args, Default)]
pub(crate) struct CloneArgs {
    #[arg(name = "repository")]
    repository: String,

    #[arg(name = "directory")]
    directory: Option<String>,
}

pub(crate) fn clone_command(args: &CloneArgs) -> GitCommandResult {
    trace!("clone_command()");

    Ok(())
}
