use crate::commands::{GitError, GitResult};
use crate::util::{bytes_to_string, find_object_file, u8_slice_to_usize};
use flate2::bufread::ZlibDecoder;
use log::trace;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{BufRead, BufReader, Read};

pub(crate) struct GitObject {
    pub(crate) kind: GitObjectType,
    pub(crate) sha1: String,
    pub(crate) size: usize,
    pub(crate) body: Option<Vec<u8>>,
}

impl GitObject {
    pub(crate) fn read(obj_id: &str) -> GitResult<GitObject> {
        trace!("read({obj_id})");
        let path = &find_object_file(obj_id)?;
        let reader = BufReader::new(fs::File::open(path)?);
        let contents = GitObject::decode_obj_content(reader)?;
        let mut header_and_body = contents.splitn(2, |b| *b == 0);
        let header = header_and_body.next().unwrap();
        let body = header_and_body.next().unwrap();
        let (obj_type, size) = GitObject::get_object_header(header)?;

        let file_part = path.file_name().unwrap();
        let file_part = file_part.to_string_lossy();
        let mut parent_part = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        parent_part.push_str(&file_part);

        Ok(GitObject {
            kind: obj_type.into(),
            sha1: parent_part,
            size,
            body: Some(body.to_vec()),
        })
    }

    fn get_object_header(content: &[u8]) -> GitResult<(String, usize)> {
        let header = &mut content.splitn(2, |x| *x == b' ');
        let obj_type = bytes_to_string(header.next().unwrap());
        let obj_len_bytes = header.next().unwrap();
        match u8_slice_to_usize(obj_len_bytes) {
            None => Err(GitError::ReadObjectError),
            Some(obj_len) => Ok((obj_type, obj_len)),
        }
    }

    fn decode_obj_content(mut reader: impl BufRead) -> GitResult<Vec<u8>> {
        let content: &mut Vec<u8> = &mut Vec::new();
        let _ = reader.read_to_end(content)?;
        let mut decoder = ZlibDecoder::new(&content[..]);
        let mut decoded_content: Vec<u8> = Vec::new();
        decoder.read_to_end(&mut decoded_content)?;

        Ok(decoded_content)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum GitObjectType {
    Blob,
    Tree,
    Commit,
}

impl Display for GitObjectType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GitObjectType::Blob => write!(f, "blob"),
            GitObjectType::Tree => write!(f, "tree"),
            GitObjectType::Commit => write!(f, "commit"),
        }
    }
}

impl From<String> for GitObjectType {
    fn from(value: String) -> Self {
        GitObjectType::from(value.as_str())
    }
}

impl From<&str> for GitObjectType {
    fn from(value: &str) -> Self {
        match value {
            "blob" => GitObjectType::Blob,
            "tree" => GitObjectType::Tree,
            "commit" => GitObjectType::Commit,
            _ => panic!("trying to convert '{}' to a GitObjectType", value),
        }
    }
}
