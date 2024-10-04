use crate::commands::hash_object::HashObjectArgs;
use crate::commands::{hash_object, GitCommandResult, GitError, GitResult};
use crate::util;
use log::trace;
use sha1::Digest;
use std::fs::DirEntry;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

pub(crate) fn write_tree_command() -> GitCommandResult {
    let sha1 = write_tree(std::env::current_dir()?)?;
    println!("{sha1}");

    Ok(())
}

#[derive(Debug)]
struct TreeEntry {
    name: String,
    mode: String,
    sha1: String,
}

#[derive(Debug)]
struct Tree {
    entries: Vec<TreeEntry>,
}

fn write_tree(path: PathBuf) -> GitResult<String> {
    trace!("write_tree({:?})", path);
    let dir = std::fs::read_dir(&path)?;

    let mut tree = Tree {
        entries: Vec::new(),
    };

    for entry in dir {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        trace!("processing {:?}", name);

        let tree_entry = if name != util::GIT_DIR_NAME && entry.metadata()?.is_dir() {
            let sha1 = write_tree(path.join(&name))?;
            make_tree_entry(name, entry, sha1)?
        } else {
            let mut file = std::fs::File::open(path.join(&name))?;
            let sha1 = hash_object::hash_object(&make_hash_object_args("blob"), &mut file)?;
            make_tree_entry(name, entry, sha1)?
        };

        tree.entries.push(tree_entry);
    }

    let mut entries: Vec<u8> = Vec::new();
    let mut size = 0;
    for entry in tree.entries.iter_mut() {
        let mode_and_name = format!("{} {}\0", entry.mode, entry.name);
        size += entries.write(mode_and_name.as_bytes())?;
        size += entries.write(hex_to_bytes(entry.sha1.as_str())?.as_slice())?;
    }

    let mut temp = util::make_temp_file()?;
    let n = temp.write(entries.as_slice())?;
    assert_eq!(n, size);
    temp.flush()?;
    let mut temp = temp.reopen()?;
    let hash = hash_object::hash_object(&make_hash_object_args("tree"), &mut temp)?;

    Ok(hash)
}

fn hex_to_bytes(hex: &str) -> GitResult<Vec<u8>> {
    hex::decode(hex).map_err(|e| GitError::HexConversionError { source: e })
}

fn make_hash_object_args(obj_type: &str) -> HashObjectArgs {
    HashObjectArgs {
        obj_type: obj_type.to_string(),
        write_to_db: true,
        ..Default::default()
    }
}

fn make_tree_entry(name: String, entry: DirEntry, sha1: String) -> GitResult<TreeEntry> {
    Ok(TreeEntry {
        name,
        mode: entry.metadata()?.mode().to_string(),
        sha1,
    })
}
