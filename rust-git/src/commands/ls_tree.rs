use clap::Args;

use crate::{commands::GitCommandResult, util};

#[derive(Debug, Args)]
pub(crate) struct LsTreeArgs {
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

    // 1. see if args.tree_ish is a tag by looking in refs/tags
    let tag = util::get_tag(args.tree_ish.as_str());
    let _tish = match tag {
        None => args.tree_ish,
        Some(t) => t.obj_id,
    };

    Ok(())
}
