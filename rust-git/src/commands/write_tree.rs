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

        trace!("processing dir entry: '{name}'");

        let tree_entry = if entry.metadata()?.is_dir() {
            if name == util::GIT_DIR_NAME {
                continue;
            }

            let sha1 = write_tree(path.join(&name))?;
            make_tree_entry(name, entry, sha1, true)?
        } else {
            let mut file = std::fs::File::open(path.join(&name))?;
            let sha1 = hash_object::hash_object(&make_hash_object_args("blob"), &mut file)?;
            make_tree_entry(name, entry, sha1, false)?
        };

        tree.entries.push(tree_entry);
    }

    tree.entries.sort_by(|x, y| x.name.cmp(&y.name));

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

fn make_tree_entry(
    name: String,
    entry: DirEntry,
    sha1: String,
    is_tree: bool,
) -> GitResult<TreeEntry> {
    let raw_mode = entry.metadata()?.mode();
    let mode = mode_to_string(raw_mode);
    Ok(TreeEntry { name, mode, sha1 })
}

/// 32-bit mode, split into (high to low bits)
///
/// 4-bit object type
///   valid values in binary are 1000 (regular file), 1010 (symbolic link)
///   and 1110 (gitlink)
///
/// 3-bit unused
///
/// 9-bit unix permission. Only 0755 and 0644 are valid for regular files.
/// Symbolic links and gitlinks have value 0 in this field.
///
/// 0100000000000000 (040 000): Directory
/// 1000000110100100 (100 644): Regular non-executable file
/// 1000000110110100 (100 664): Regular non-executable group-writeable file
/// 1000000111101101 (100 755): Regular executable file
/// 1010000000000000 (120 000): Symbolic link
/// 1110000000000000 (160 000): Gitlink
///
/// https://stackoverflow.com/questions/737673/how-to-read-the-mode-field-of-git-ls-trees-output/8347325

fn mode_to_string(mode: u32) -> String {
    match mode & 0o170000 {
        // if the type is dir or symlink, just use that; don't care about further permissions
        0o040000 | 0o120000 => format!("{:0>6o}", mode & 0o170000),
        // regular files, use the whole thing
        0o100000 => format!("{:0>6o}", mode),
        // everything else is invalid (e.g. block/char devices, pipes, etc)
        _ => panic!("invalid mode: {:o}", mode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_to_string() {
        assert_eq!("040000", mode_to_string(0o040755));
        assert_eq!("120000", mode_to_string(0o120000));
        assert_eq!("100755", mode_to_string(0o100755));
        assert_eq!("100444", mode_to_string(0o100444));
    }
}
