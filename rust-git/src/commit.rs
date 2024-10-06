use crate::object::GitObject;
use bytes::Buf;
use std::io::{BufRead, Read};

// The format for a commit object is simple: it specifies the top-level tree for the snapshot of
// the project at that point; the parent commits if any (the commit object described above does not
// have any parents); the author/committer information (which uses your user.name and user.email
// configuration settings and a timestamp); a blank line, and then the commit message.
pub(crate) struct Commit {
    pub(crate) sha1: String,
    pub(crate) tree: String,
    pub(crate) parent: Option<String>,
    pub(crate) author: String,
    pub(crate) committer: String,
    pub(crate) comment: String,
}

impl From<GitObject<'_>> for Commit {
    fn from(object: GitObject) -> Self {
        let body = object.body.unwrap();
        let mut reader = body.reader();

        let tree = get_entry(&mut reader, "tree").unwrap_or_else(|| panic!("invalid commit object"));
        let parent = get_entry(&mut reader, "parent"); // parent is optional, but rarely so
        let author = get_entry(&mut reader, "author").unwrap_or_else(|| panic!("invalid commit object"));
        let committer = get_entry(&mut reader, "committer").unwrap_or_else(|| panic!("invalid commit object"));

        let mut comment = String::new();
        let _ = reader.read_to_string(&mut comment);

        Self {
            sha1: object.sha1.to_string(),
            tree,
            parent,
            author,
            committer,
            comment,
        }
    }
}

fn get_entry(reader: &mut impl BufRead, name: &str) -> Option<String> {
    let mut entry = String::new();
    let _ = reader.read_line(&mut entry);
    let mut n = entry.splitn(2, ' ');
    match n.next() {
        Some(e) if e == name => Some(n.next().unwrap().trim().to_string()),
        _ => None
    }
}
