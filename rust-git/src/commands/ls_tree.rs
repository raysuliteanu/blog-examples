use crate::commands::GitCommandResult;
use crate::commands::{GitError, GitResult};
use crate::object::{GitObject, GitObjectType};
use crate::{commit, tag, util};
use clap::{arg, Args};
use log::{debug, trace};
use std::io::Read;

#[derive(Debug, Args, Default)]
pub(crate) struct LsTreeArgs {
    /// Show only the named tree entry itself, not its children.
    #[arg(long, default_value = "false")]
    name_only: bool,

    /// Show only the named tree entry itself, not its children.
    #[arg(short, default_value = "false")]
    dir_only: bool,

    /// Recurse into sub-trees.
    #[arg(short, default_value = "false")]
    recurse: bool,

    /// Show tree entries even when going to recurse them. Has no effect if -r was not passed.  -d implies -t.
    #[arg(short = 't', default_value = "false")]
    show_trees: bool,

    /// Show object size of blob (file) entries.
    #[arg(short = 'l', long = "long", default_value = "false")]
    show_size: bool,

    #[arg(name = "tree-ish")]
    tree_ish: String,

    /// When paths are given, show them (note that this isn’t really raw pathnames, but rather a list of
    /// patterns to match). Otherwise implicitly uses the root level of the tree as the sole path argument.
    #[arg(name = "path")]
    path: Option<Vec<String>>,
}

pub(crate) fn ls_tree_command(args: LsTreeArgs) -> GitCommandResult {
    ls_tree(&args.tree_ish, &args)
}

pub(crate) fn ls_tree(obj_id: &String, args: &LsTreeArgs) -> GitCommandResult {
    trace!("ls_tree({obj_id})");
    match GitObject::read(obj_id) {
        Ok(obj) => match obj.kind {
            GitObjectType::Tree => {
                // format and print tree obj body
                print_tree_object(args, obj, None)
            }
            GitObjectType::Commit => {
                // get tree object of commit and print that
                let commit = commit::Commit::from(obj);
                ls_tree(&commit.tree, args)
            }
            GitObjectType::Blob => {
                debug!("cannot ls-tree a blob");
                Err(GitError::InvalidObjectId {
                    obj_id: args.tree_ish.to_string(),
                })
            }
        },
        Err(_) => {
            debug!("cannot read object file for id '{obj_id}'; trying as a tag ...");
            // could be that the arg_id is not an object (blob/commit/tree)
            // check for tag
            match tag::Tag::get_tag(obj_id) {
                Some(tag) => ls_tree(&tag.obj_id, args),
                None => {
                    debug!("not a tag {obj_id}");
                    // not a tree or a commit or a tag, no good
                    Err(GitError::InvalidObjectId {
                        obj_id: obj_id.to_string(),
                    })
                }
            }
        }
    }
}

// TODO: when printing (recursively only?) implicitly filter entries at "higher"
// directories i.e. if the tree structure is src/commands/this/that and ls-file
// is executed from src/commands/this then only entries in this and this/that
// should be printed

/// each line of content is of the form
/// `[filemode][SP][filename]\0[hash-bytes]`
/// where SP is ASCII space (0x20) and where hash-bytes is the SHA-1 hash, a
/// fixed 20 bytes in length; so the next "line" starts immediately after that
/// e.g.
/// ```
/// [filemode][SP][filename]\0[hash-bytes][filemode][SP][filename]\0[hash-bytes]
/// ```
pub fn print_tree_object(
    args: &LsTreeArgs,
    obj: GitObject,
    path_part: Option<String>,
) -> GitResult<()> {
    // each entry is 'mode name\0[hash:20]
    let mut body = obj.body.unwrap();

    loop {
        if body.is_empty() {
            break;
        }

        // 1. split into two buffers, `[mode_and_name]0[rest]` with the 0 discarded
        let mut split = body.splitn(2, |b| *b == 0);
        let mode_and_file = split.next().unwrap();
        let mut rest = split.next().unwrap();

        // 2. spit the mode_and_name buffer into the mode and the name, which are separated by ' '
        let mut split = mode_and_file.split(|b| *b == b' ');
        let mode = util::bytes_to_string(split.next().unwrap());
        let filename = util::bytes_to_string(split.next().unwrap());

        // 3. read the next 20 bytes from `rest` which is the object hash
        let mut hash_buf = [0u8; 20];
        rest.read_exact(&mut hash_buf)?;

        // 4. point body at the remaining bytes for the loop
        body = rest.to_vec();

        // 5. using the hash, look up the referenced object to get its type
        let hash = hex::encode(hash_buf);
        let entry_obj = GitObject::read(hash.as_str())?;
        let kind = &entry_obj.kind;

        let path = create_file_name(&path_part, filename);

        // 6. if name_only then only print the name :)
        if args.name_only {
            if *kind == GitObjectType::Tree && args.recurse {
                print_tree_object(args, entry_obj, Some(path))?;
            } else {
                println!("{}", path);
            }

            continue;
        }

        if *kind == GitObjectType::Tree && args.recurse {
            print_tree_object(args, entry_obj, Some(path))?;
        } else {
            print!("{:0>6} {} {}", mode, kind, hash);

            if args.show_size {
                let len = entry_obj.size;
                if entry_obj.kind == GitObjectType::Tree {
                    print!("{: >8}", "-");
                } else {
                    print!("{: >8}", len);
                }
            }

            println!("\t{}", path);
        }
    }

    Ok(())
}

fn create_file_name(path: &Option<String>, filename: String) -> String {
    match path {
        Some(p) => p.to_owned() + "/" + filename.as_str(),
        None => filename,
    }
}
