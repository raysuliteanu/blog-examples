use chrono::Local;
use clap::Args;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    commands::{hash_object, GitCommandResult},
    config::GIT_CONFIG,
    hash_object::HashObjectArgs,
    object, util,
};

#[derive(Debug, Args, Default)]
pub(crate) struct CommitTreeArgs {
    /// Each -p indicates the id of a parent commit object.
    /// Impl note: only handling one parent
    #[arg(short, name = "parent")]
    parent: Option<String>,

    /// A paragraph in the commit log message. This can be given more than once and
    /// each <message> becomes its own paragraph.
    /// Impl note: only handling one single-line message
    #[arg(short, name = "message")]
    message: Option<String>,

    /// An existing tree object.
    #[arg(name = "tree")]
    tree: String,
}

pub(crate) fn commit_tree_command(args: CommitTreeArgs) -> GitCommandResult {
    // make sure tree exists
    let tree = object::GitObject::read(args.tree.as_str())?;
    assert!(tree.sha1.starts_with(args.tree.as_str()));
    let tree_hash = tree.sha1;

    let email_default = || GIT_CONFIG.get("user.email").expect("valid user.email");
    let user_default = || GIT_CONFIG.get("user.name").expect("valid user.name");

    let author_email = GIT_CONFIG.get("author.email").unwrap_or_else(email_default);
    let author_name = GIT_CONFIG.get("author.name").unwrap_or_else(user_default);

    let committer_email = GIT_CONFIG
        .get("committer.email")
        .unwrap_or_else(email_default);
    let committer_name = GIT_CONFIG
        .get("committer.name")
        .unwrap_or_else(user_default);

    let mut commit: Vec<u8> = Vec::new();
    let mut size = commit.write(format!("tree {}\n", tree_hash).as_bytes())?;

    if let Some(parent_arg) = args.parent {
        let parent = object::GitObject::read(&parent_arg)?;
        size += commit.write(format!("parent {}\n", parent.sha1).as_bytes())?;
    }

    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("valid epoch time")
        .as_secs();

    let tz = get_tz();

    size += commit.write(
        format!(
            "author {} <{}> {} {}\n",
            author_name, author_email, epoch, tz
        )
        .as_bytes(),
    )?;
    size += commit.write(
        format!(
            "committer {} <{}> {} {}\n",
            committer_name, committer_email, epoch, tz
        )
        .as_bytes(),
    )?;

    size += commit.write("\n".as_bytes())?;

    if let Some(message) = args.message {
        size += commit.write(format!("{}\n", message).as_bytes())?;
    }

    let mut temp = util::make_temp_file()?;
    let n = temp.write(&commit)?;
    assert_eq!(n, size);
    temp.flush()?;
    let mut temp = temp.reopen()?;
    let hash = hash_object::hash_object(&make_hash_object_args("commit"), &mut temp)?;

    println!("{hash}");

    Ok(())
}

fn make_hash_object_args(obj_type: &str) -> HashObjectArgs {
    HashObjectArgs {
        obj_type: obj_type.to_string(),
        write_to_db: true,
        ..Default::default()
    }
}

fn get_tz() -> String {
    let local_time = Local::now();
    let offset = local_time.offset();
    let local_offset = offset.local_minus_utc();
    format!("{:+}{:02}", local_offset / 3600, (local_offset % 3600) / 60)
}

#[cfg(test)]
mod test {
    use crate::commands::commit_tree::get_tz;
    use chrono::Local;

    #[test]
    fn tz() {
        let local_time = Local::now();
        let offset = local_time.offset();
        let local = offset.local_minus_utc();
        eprintln!("local: {}", local);
        let utc = offset.utc_minus_local();
        eprintln!("UTC: {}", utc);

        let tz = dbg!(get_tz());
        assert!(tz.starts_with("-"));
    }
}
