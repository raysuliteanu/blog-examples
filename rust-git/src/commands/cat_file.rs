use crate::commands::{GitCommandResult, GitError, GitResult};
use crate::util;
use crate::util::GitObjectType;
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
    let result = util::find_object_file(&args.object);

    let path = match result {
        // if -e option (test for object existence) return Ok now, don't continue
        Ok(_) if args.exists => return Ok(()),
        Ok(p) => p,
        // if error already, return now, no point continuing regardless of -e option or not
        Err(e) => return Err(GitError::from(e)),
    };

    let decoded_content = &mut util::get_object_from_path(path)?;

    let index = util::find_null_byte_index(decoded_content);

    let (obj_type, obj_len) = util::get_object_header(decoded_content, index);

    let content = &decoded_content[index + 1..];

    if args.pretty {
        match GitObjectType::from(obj_type) {
            GitObjectType::Blob | GitObjectType::Commit => {
                print!("{}", util::bytes_to_string(content));
            }
            GitObjectType::Tree => {
                handle_cat_file_tree_object(content)?;
            }
        }
    } else if args.obj_type {
        println!("{obj_type}");
    } else if args.show_size {
        println!("{obj_len}");
    }

    Ok(())
}

/// each line of content is of the form
/// `[filemode][SP][filename]\0[hash-bytes]`
/// where SP is ASCII space (0x20) and where hash-bytes is the SHA-1 hash, a
/// fixed 20 bytes in length; so the next "line" starts immediately after that
/// e.g.
/// ```
/// [filemode][SP][filename]\0[hash-bytes][filemode][SP][filename]\0[hash-bytes]
/// ```
fn handle_cat_file_tree_object(content: &[u8]) -> GitResult<()> {
    let mut consumed = 0usize;
    let len = content.len();
    while consumed < len {
        let index = util::find_null_byte_index(&content[consumed..]);
        let end = consumed + index;
        assert!(end < content.len());

        let mode_and_file = &mut content[consumed..end].split(|x| *x == b' ');
        let mode = util::bytes_to_string(mode_and_file.next().unwrap());
        let file = util::bytes_to_string(mode_and_file.next().unwrap());
        consumed += index + 1; // +1 for null byte

        let hash = hex::encode(&content[consumed..consumed + 20]);
        consumed += 20; // sizeof SHA-1 hash

        let obj_contents = &mut util::get_object(hash.as_str())?;
        let index = util::find_null_byte_index(obj_contents);
        let (obj_type, _) = util::get_object_header(obj_contents, index);

        println!("{:0>6} {} {}    {}", mode, obj_type, hash, file);
    }

    Ok(())
}
