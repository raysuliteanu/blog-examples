use crate::commands::{GitError, GitResult};
use lazy_static::lazy_static;
use log::debug;
use std::ffi::OsString;
use std::path::PathBuf;
use std::env;
use tempfile::{Builder, NamedTempFile};

pub(crate) const GIT_DEFAULT_BRANCH_NAME: &str = "master";
pub(crate) const GIT_DIR_NAME: &str = ".git";
pub(crate) const GIT_OBJ_DIR_NAME: &str = "objects";
pub(crate) const GIT_OBJ_BRANCHES_DIR_NAME: &str = "objects/branches";
pub(crate) const GIT_OBJ_HOOKS_DIR_NAME: &str = "objects/hooks";
pub(crate) const GIT_OBJ_INFO_DIR_NAME: &str = "objects/info";
pub(crate) const GIT_OBJ_PACK_DIR_NAME: &str = "objects/pack";
pub(crate) const _GIT_REFS_DIR_NAME: &str = "refs";
pub(crate) const GIT_REFS_HEADS_DIR_NAME: &str = "refs/heads";
pub(crate) const GIT_REFS_TAGS_DIR_NAME: &str = "refs/tags";

lazy_static! {
    // NOTE: GIT_PARENT_DIR will panic if used during 'init' command processing
    // since there of course is no .git dir to find the parent of yet!
    pub(crate) static ref GIT_PARENT_DIR: PathBuf = find_git_parent_dir();

    pub(crate) static ref GIT_HEAD: PathBuf = PathBuf::from("HEAD");
    pub(crate) static ref GIT_REPO_CONFIG_FILE: PathBuf = PathBuf::from("config");
}

pub(crate) fn u8_slice_to_usize(slice: &[u8]) -> Option<usize> {
    std::str::from_utf8(slice)
        .ok()
        .map(|s| s.parse::<usize>().unwrap())
}

pub(crate) fn bytes_to_string(content: &[u8]) -> String {
    std::str::from_utf8(content)
        .expect("failed to convert bytes to string")
        .to_string()
}

pub(crate) fn get_git_dirs(
    directory: Option<OsString>,
    separate_git_dir: Option<OsString>,
) -> GitResult<(PathBuf, Option<PathBuf>)> {
    let git_parent_dir = if let Some(dir) = directory {
        std::fs::canonicalize(dir.to_str().unwrap())?
    } else {
        env::current_dir()?
    };

    let separate_parent_dir =
        separate_git_dir.map(|dir| std::fs::canonicalize(dir.to_str().unwrap()).unwrap());

    Ok((git_parent_dir, separate_parent_dir))
}

pub(crate) fn find_git_parent_dir() -> PathBuf {
    let current_dir = env::current_dir().expect("failed to get current directory");
    let mut current_dir = current_dir.to_path_buf();

    loop {
        let git_dir = current_dir.join(GIT_DIR_NAME);
        if git_dir.is_dir() {
            debug!("found .git dir: {:?}", git_dir.parent().unwrap());
            return git_dir.parent().unwrap().to_path_buf();
        }

        if !current_dir.pop() {
            break;
        }
    }

    panic!("not a git repository (or any of the parent directories): .git")
}

pub(crate) fn get_git_object_dir() -> PathBuf {
    GIT_PARENT_DIR.join(GIT_DIR_NAME).join(GIT_OBJ_DIR_NAME)
}

pub(crate) fn get_git_tags_dir() -> PathBuf {
    GIT_PARENT_DIR
        .join(GIT_DIR_NAME)
        .join(GIT_REFS_TAGS_DIR_NAME)
}

pub(crate) fn find_object_file(obj_id: &str) -> GitResult<PathBuf> {
    if obj_id.len() < 3 {
        return Err(GitError::InvalidObjectId {
            obj_id: String::from(obj_id),
        });
    }

    let (dir_name, id) = obj_id.split_at(2);
    let dir = get_git_object_dir().join(dir_name);
    if !dir.exists() || !dir.is_dir() {
        debug!("can't access {}", dir.display());
        return Err(GitError::InvalidObjectId {
            obj_id: String::from(obj_id),
        });
    }

    let mut file = dir.join(id);
    if !file.exists() || !file.is_file() {
        // maybe not a full hash so do a partial match
        let mut found = false;
        for entry in dir
            .read_dir()
            .unwrap_or_else(|_| panic!("Not a valid object name {obj_id}"))
            .flatten()
        {
            let os_string = entry.file_name();
            let filename = os_string.to_str().unwrap();
            if filename.starts_with(id) {
                file = dir.join(filename);
                found = true;
                break;
            }
        }

        if !found {
            debug!("Not a valid object name {obj_id}");
            return Err(GitError::InvalidObjectId {
                obj_id: String::from(obj_id),
            });
        }
    }

    debug!("found {:?}", file);

    Ok(file)
}

pub(crate) fn make_temp_file() -> GitResult<NamedTempFile> {
    let temp_file = Builder::new().prefix("rg").suffix(".tmp").tempfile()?;
    debug!("temp file: {:?}", temp_file.path());
    Ok(temp_file)
}
