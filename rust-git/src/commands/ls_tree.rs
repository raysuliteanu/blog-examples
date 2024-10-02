use clap::Args;
use std::io::Read;

use crate::commands::GitCommandResult;
use crate::commands::{GitError, GitResult};
use crate::object::{GitObject, GitObjectType};
use crate::util;

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
    // From the Git documentation:
    //
    // A tree object or an object that can be recursively dereferenced to a tree object. Dereferencing a commit object
    // yields the tree object corresponding to the revision's top directory. The following are all tree-ishes: a commit-ish,
    // a tree object, a tag object that points to a tree object, a tag object that points to a tag object that points to a
    // tree object, etc.

    let arg_id = args.tree_ish.as_str();
    let obj = GitObject::read(arg_id)?;
    match obj.kind {
        GitObjectType::Tree => {
            // format and print tree obj body
            print_tree_object(&args, obj)
        }
        GitObjectType::Commit => {
            // get tree object of commit and print that
            todo!("handle commit obj")
        }
        GitObjectType::Tag => {
            // iterate until tag points to a tree object (or not)
            todo!("handle tag obj")
        }
        GitObjectType::Blob => {
            eprintln!("cannot ls-tree a blob");
            Err(GitError::InvalidObjectId {
                obj_id: args.tree_ish,
            })
        }
    }
}

pub fn print_tree_object(_args: &LsTreeArgs, obj: GitObject) -> GitResult<()> {
    // each entry is 'mode name\0[hash:20]
    let mut body = obj.body.unwrap();

    loop {
        if body.is_empty() {
            break;
        }

        let mut split = body.splitn(2, |b| *b == 0);
        let mode_and_file = split.next().unwrap();
        let mut rest = split.next().unwrap();
        let mut split = mode_and_file.split(|b| *b == b' ');
        let mode = util::bytes_to_string(split.next().unwrap());
        let file = util::bytes_to_string(split.next().unwrap());

        let mut hash_buf = [0u8; 20];
        rest.read_exact(&mut hash_buf)?;
        body = rest.to_vec();

        let hash = hex::encode(hash_buf);
        let entry_obj = GitObject::read(hash.as_str())?;

        println!("{:0>6} {} {}    {}", mode, entry_obj.kind, hash, file);
    }

    Ok(())
}
