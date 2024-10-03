use crate::commands::GitCommandResult;
use crate::commands::{GitError, GitResult};
use crate::object::{GitObject, GitObjectType};
use crate::{tag, util};
use clap::{arg, Args};
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
    // From the Git documentation:
    //
    // A tree object or an object that can be recursively dereferenced to a tree object. Dereferencing a commit object
    // yields the tree object corresponding to the revision's top directory. The following are all tree-ishes: a commit-ish,
    // a tree object, a tag object that points to a tree object, a tag object that points to a tag object that points to a
    // tree object, etc.

    match GitObject::read(obj_id) {
        Ok(obj) => match obj.kind {
            GitObjectType::Tree => {
                // format and print tree obj body
                print_tree_object(&args, obj)
            }
            GitObjectType::Commit => {
                // get tree object of commit and print that
                todo!("handle commit obj")
            }
            GitObjectType::Blob => {
                eprintln!("cannot ls-tree a blob");
                Err(GitError::InvalidObjectId {
                    obj_id: args.tree_ish.to_string(),
                })
            }
            _ => todo!("can we get here e.g. for a tag? I think that goes to the Err arm"),
        },
        Err(_) => {
            // could be that the arg_id is not an object (blob/commit/tree)
            // check for tag
            match tag::Tag::get_tag(obj_id) {
                Some(tag) => ls_tree(&tag.obj_id, args),
                None => {
                    // not a tree or a commit or a tag, no good
                    Err(GitError::InvalidObjectId {
                        obj_id: obj_id.to_string(),
                    })
                }
            }
        }
    }
}

pub fn print_tree_object(args: &LsTreeArgs, obj: GitObject) -> GitResult<()> {
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
        let file = util::bytes_to_string(split.next().unwrap());

        // 4. read the next 20 bytes from `rest` which is the object hash
        let mut hash_buf = [0u8; 20];
        rest.read_exact(&mut hash_buf)?;

        // 4. point body at the remaining bytes for the loop
        body = rest.to_vec();

        // 5. if name_only then only print the name :)
        if args.name_only {
            println!("{}", file);
            continue;
        }

        // 5. using the hash, look up the referenced object to get its type
        let hash = hex::encode(hash_buf);
        let entry_obj = GitObject::read(hash.as_str())?;
        let kind = &entry_obj.kind;

        print!("{:0>6} {} {}", mode, kind, hash);

        if args.show_size {
            let len = entry_obj.size;
            if entry_obj.kind == GitObjectType::Tree {
                print!("{: >8}", "-");
            } else {
                print!("{: >8}", len);
            }
        }

        println!("    {}", file);
    }

    Ok(())
}
