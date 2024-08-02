use std::ffi::OsString;
use std::{fs, io};

use clap::Args;
use log::{debug, trace};

use crate::commands::config::GIT_CONFIG;
use crate::util;
use crate::util::{
    GIT_DEFAULT_BRANCH_NAME, GIT_DIR_NAME, GIT_HEAD, GIT_OBJ_BRANCHES_DIR_NAME,
    GIT_OBJ_HOOKS_DIR_NAME, GIT_OBJ_INFO_DIR_NAME, GIT_OBJ_PACK_DIR_NAME, GIT_REFS_HEADS_DIR_NAME,
    GIT_REFS_TAGS_DIR_NAME, GIT_REPO_CONFIG_FILE,
};

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(short, long, default_value_t)]
    pub(crate) quiet: bool,
    #[arg(long, default_value_t)]
    pub(crate) bare: bool,
    #[arg(long)]
    pub(crate) template: Option<OsString>,
    #[arg(long)]
    pub(crate) separate_git_dir: Option<OsString>,
    #[arg(long, default_value = "sha1")]
    pub(crate) object_format: String,
    #[arg(short = 'b', long)]
    pub(crate) initial_branch: Option<String>,
    #[arg(long)]
    pub(crate) shared: Option<String>,
    pub(crate) directory: Option<OsString>,
}

pub(crate) fn init_command(args: InitArgs) -> io::Result<()> {
    let (git_parent_dir, separate_git_dir) =
        util::get_git_dirs(args.directory, args.separate_git_dir)?;

    debug!(
        "git dir: {:?}\tseparate dir: {:?}",
        git_parent_dir, separate_git_dir
    );

    let actual_git_parent_dir = match separate_git_dir {
        Some(dir) => {
            // make link to dir
            if !git_parent_dir.exists() {
                debug!("creating {:?}", git_parent_dir);
                fs::create_dir_all(&git_parent_dir)?;
            }

            let dot_git_file = git_parent_dir.join(GIT_DIR_NAME);
            debug!("creating {:?}", dot_git_file);
            fs::write(&dot_git_file, format!("gitdir: {}\n", dir.display()))?;

            dir
        }
        None => {
            if !git_parent_dir.exists() {
                debug!("creating {:?}", git_parent_dir);
                fs::create_dir_all(&git_parent_dir)?;
            }

            git_parent_dir.join(GIT_DIR_NAME)
        }
    };

    for dir in [
        GIT_OBJ_BRANCHES_DIR_NAME,
        GIT_OBJ_HOOKS_DIR_NAME,
        GIT_OBJ_PACK_DIR_NAME,
        GIT_OBJ_INFO_DIR_NAME,
        GIT_REFS_TAGS_DIR_NAME,
        GIT_REFS_HEADS_DIR_NAME,
    ] {
        let path = actual_git_parent_dir.join(dir);
        fs::create_dir_all(&path)?;
        trace!("created {}", &path.display());
    }

    let path_buf = actual_git_parent_dir.join(GIT_HEAD.as_path());

    let branch_name =
        args.initial_branch
            .unwrap_or_else(|| match GIT_CONFIG.get("init.defaultBranch") {
                Some(branch) => branch.to_string(),
                None => GIT_DEFAULT_BRANCH_NAME.to_string(),
            });

    fs::write(
        path_buf.as_path(),
        format!("ref: refs/heads/{branch_name}\n"),
    )?;

    let mut dot_git_config = String::from(
        r"[core]
    repositoryformatversion = 0
    filemode = true
    logallrefupdates = true
    ",
    );

    dot_git_config.push_str(format!("bare = {}\n\n", args.bare).as_str());

    let config_file_path = actual_git_parent_dir.join(GIT_REPO_CONFIG_FILE.as_path());
    fs::write(config_file_path.as_path(), dot_git_config)?;

    println!(
        "Initialized empty Git repository in {}",
        actual_git_parent_dir.display()
    );

    Ok(())
}
