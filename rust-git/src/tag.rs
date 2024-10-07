use crate::util::get_git_tags_dir;
use log::debug;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub(crate) struct Tag {
    _name: String,
    _path: PathBuf,
    pub obj_id: String,
}

impl Tag {
    pub(crate) fn get_tag(name: &str) -> Option<Tag> {
        let path = get_git_tags_dir().join(name);
        debug!("looking for tag {}", path.display());
        match File::open(path) {
            Ok(mut file) => {
                let mut obj_id = String::new();
                match file.read_to_string(&mut obj_id) {
                    Ok(_) => Some(Tag {
                        _name: name.to_string(),
                        _path: get_git_tags_dir().join(name),
                        obj_id: obj_id.trim().to_string(),
                    }),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }
}
