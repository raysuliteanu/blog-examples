use crate::commands::{GitError, GitResult};
use flate2::bufread::ZlibDecoder;
use lazy_static::lazy_static;
use log::debug;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::{env, path};

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

#[derive(Debug)]
pub(crate) enum GitObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl From<String> for GitObjectType {
    fn from(value: String) -> Self {
        GitObjectType::from(value.as_str())
    }
}

impl From<&str> for GitObjectType {
    fn from(value: &str) -> Self {
        match value {
            "blob" => GitObjectType::Blob,
            "tree" => GitObjectType::Tree,
            "commit" => GitObjectType::Commit,
            "tag" => GitObjectType::Tag,
            _ => panic!(),
        }
    }
}

pub(crate) fn get_git_dirs(
    directory: Option<OsString>,
    separate_git_dir: Option<OsString>,
) -> GitResult<(PathBuf, Option<PathBuf>)> {
    let git_parent_dir = if let Some(dir) = directory {
        path::absolute(dir.to_str().unwrap())?
    } else {
        env::current_dir()?
    };

    let separate_parent_dir =
        separate_git_dir.map(|dir| path::absolute(dir.to_str().unwrap()).unwrap());

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

pub(crate) struct Tag {
    pub name: String,
    pub path: PathBuf,
    pub obj_id: String,
}

pub(crate) fn get_tag(name: &str) -> Option<Tag> {
    let path = get_git_tags_dir().join(name);
    match File::open(&path) {
        Ok(mut file) => {
            let mut obj_id = String::new();
            match file.read_to_string(&mut obj_id) {
                Ok(_) => Some(Tag {
                    name: name.to_string(),
                    path,
                    obj_id,
                }),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

pub(crate) fn get_object_header(decoded_content: &mut [u8], index: usize) -> (String, String) {
    let header = &mut decoded_content[0..index].split(|x| *x == b' ');
    let obj_type = bytes_to_string(header.next().unwrap());
    let obj_len = bytes_to_string(header.next().unwrap());
    (obj_type, obj_len)
}

pub(crate) fn find_null_byte_index(content: &[u8]) -> usize {
    debug!("{:?}", content);
    for (i, v) in content.iter().enumerate() {
        if *v == 0 {
            return i;
        }
    }

    content.len()
}

pub(crate) fn bytes_to_string(content: &[u8]) -> String {
    content
        .iter()
        .map(|b| *b as char)
        .fold(String::new(), |mut acc, c| {
            acc.push(c);
            acc
        })
}

pub(crate) fn get_object(object: &str) -> GitResult<Vec<u8>> {
    let object_file = find_object_file(object);
    match object_file {
        Ok(path) => Ok(get_object_from_path(path)?),
        Err(e) => Err(e),
    }
}

pub(crate) fn get_object_from_path(path: PathBuf) -> GitResult<Vec<u8>> {
    match File::open(path) {
        Ok(file) => Ok(decode_obj_content(file)?),
        Err(e) => Err(GitError::Io { source: e }),
    }
}

fn decode_obj_content(file: File) -> GitResult<Vec<u8>> {
    let content: &mut Vec<u8> = &mut Vec::new();
    let mut reader = BufReader::new(file);
    let _ = reader.read_to_end(content)?;
    let mut decoder = ZlibDecoder::new(&content[..]);
    let mut decoded_content: Vec<u8> = Vec::new();
    decoder.read_to_end(&mut decoded_content)?;

    Ok(decoded_content)
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
