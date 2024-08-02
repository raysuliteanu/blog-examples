use std::io;

use clap::Args;

use crate::util;
use crate::util::GitObjectType;

#[derive(Debug, Args)]
pub(crate) struct CatFileArgs {
    #[arg(short, default_value = "false")]
    pub(crate) pretty: bool,
    #[arg(short = 't', default_value = "false")]
    pub(crate) obj_type: bool,
    #[arg(short, default_value = "false")]
    pub(crate) show_size: bool,
    #[arg(long, default_value = "false")]
    pub(crate) allow_unknown_type: bool,
    #[arg(short, default_value = "false")]
    pub(crate) exists: bool,
    pub(crate) object: String,
}

pub(crate) fn cat_file_command(args: CatFileArgs) -> io::Result<()> {
    let decoded_content = &mut util::get_object(&args.object)?;

    let index = util::find_null_byte_index(decoded_content);

    let (obj_type, obj_len) = util::get_object_header(decoded_content, index);

    let content = &decoded_content[index + 1..];

    if args.pretty {
        match GitObjectType::from(obj_type) {
            GitObjectType::Blob | GitObjectType::Commit => {
                print!("{}", util::bytes_to_string(content));
            }
            GitObjectType::Tree => {
                handle_cat_file_tree_object(obj_len, content)?;
            }
        }
    } else if args.obj_type {
        println!("{obj_type}");
    } else if args.show_size {
        println!("{obj_len}");
    } else {
        unimplemented!("only stdin is currently supported")
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
fn handle_cat_file_tree_object(obj_len: String, content: &[u8]) -> io::Result<()> {
    let mut consumed = 0usize;
    let len = obj_len.as_str().parse::<usize>().expect("invalid length");
    while consumed < len {
        let index = util::find_null_byte_index(&content[consumed..]);
        let end = consumed + index;
        assert!(end < content.len());
        let tree_row_prefix = &mut content[consumed..end].split(|x| *x == b' ');
        let mode = util::bytes_to_string(tree_row_prefix.next().unwrap());
        let file = util::bytes_to_string(tree_row_prefix.next().unwrap());
        consumed += index + 1; // +1 for SP (0x20) char
        let hash = hex::encode(&content[consumed..consumed + 20]);
        consumed += 20; // sizeof SHA-1 hash
        let obj_contents = &mut util::get_object(hash.as_str())?;
        let index = util::find_null_byte_index(obj_contents);
        let (obj_type, _) = util::get_object_header(obj_contents, index);
        println!("{:0>6} {} {}    {}", mode, obj_type, hash, file);
    }

    Ok(())
}
