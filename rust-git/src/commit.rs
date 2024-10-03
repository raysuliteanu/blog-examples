use crate::object::GitObject;
use bytes::Buf;
use std::io::{BufRead, Read};

// The format for a commit object is simple: it specifies the top-level tree for the snapshot of
// the project at that point; the parent commits if any (the commit object described above does not
// have any parents); the author/committer information (which uses your user.name and user.email
// configuration settings and a timestamp); a blank line, and then the commit message.
pub(crate) struct Commit {
    sha1: String,
    pub(crate) tree: String,
    parent: String,
    author: String,
    committer: String,
    comment: String,
}

impl From<GitObject<'_>> for Commit {
    fn from(object: GitObject) -> Self {
        let body = object.body.unwrap();
        let mut reader = body.reader();

        let tree = get_entry(&mut reader, "tree");
        let parent = get_entry(&mut reader, "parent");
        let author = get_entry(&mut reader, "author");
        let committer = get_entry(&mut reader, "committer");

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

fn get_entry(reader: &mut impl BufRead, name: &str) -> String {
    let mut entry = String::new();
    let _ = reader.read_line(&mut entry);
    let mut n = entry.splitn(2, ' ');
    assert_eq!(n.next(), Some(name));
    n.next().unwrap().trim().to_string()
}
