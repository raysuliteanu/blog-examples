use crate::commands::ls_tree::LsTreeArgs;
use crate::commands::{ls_tree, GitCommandResult, GitResult};
use crate::object::{GitObject, GitObjectType};
use crate::util;
use clap::Args;

#[derive(Debug, Args)]
pub(crate) struct CatFileArgs {
    /// pretty-print object's content
    #[arg(short, default_value = "false", group = "operation")]
    pretty: bool,
    /// show object type
    #[arg(short = 't', default_value = "false", group = "operation")]
    obj_type: bool,
    /// allow -s and -t to work with broken/corrupt objects
    #[arg(long, default_value = "false")]
    allow_unknown_type: bool,
    /// show object size
    #[arg(short, default_value = "false", group = "operation")]
    show_size: bool,
    /// exit with zero when there's no error
    #[arg(short, default_value = "false", group = "operation")]
    exists: bool,
    #[arg(name = "object")]
    object: String,
}

pub(crate) fn cat_file_command(args: CatFileArgs) -> GitCommandResult {
    let obj = GitObject::read(&args.object)?;

    if args.exists {
        return Ok(());
    }

    if args.pretty {
        match obj.kind {
            GitObjectType::Blob | GitObjectType::Commit => {
                print!("{}", util::bytes_to_string(obj.body.unwrap().as_slice()));
            }
            GitObjectType::Tree => {
                handle_cat_file_tree_object(obj)?;
            }
            _ => {}
        }
    } else if args.obj_type {
        println!("{}", obj.kind);
    } else if args.show_size {
        println!("{}", obj.size);
    }

    Ok(())
}

fn handle_cat_file_tree_object(obj: GitObject) -> GitResult<()> {
    let args = LsTreeArgs::default();
    ls_tree::print_tree_object(&args, obj, None)
}
