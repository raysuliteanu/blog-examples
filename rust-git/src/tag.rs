use crate::util::get_git_tags_dir;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub(crate) struct Tag {
    pub name: String,
    pub path: PathBuf,
    pub obj_id: String,
}

impl Tag {
    pub(crate) fn get_tag(name: &str) -> Option<Tag> {
        let path = get_git_tags_dir().join(name);
        match File::open(path) {
            Ok(mut file) => {
                let mut obj_id = String::new();
                match file.read_to_string(&mut obj_id) {
                    Ok(_) => Some(Tag {
                        name: name.to_string(),
                        path: get_git_tags_dir().join(name),
                        obj_id,
                    }),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
}