use crate::util::get_git_object_dir;
use clap::Args;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use log::debug;
use sha1::{Digest, Sha1};
use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, Read, Write};
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug, Args)]
pub(crate) struct HashObjectArgs {
    #[arg(short = 't', default_value = "blob")]
    pub(crate) obj_type: String,
    #[arg(short = 'w', default_value = "false")]
    pub(crate) write_to_db: bool,
    #[arg(long, default_value = "false")]
    pub(crate) stdin: bool,
    #[arg(long, default_value = "false")]
    pub(crate) literally: bool,
    #[arg(last = true)]
    pub(crate) files: Option<Vec<OsString>>,
}

pub(crate) fn hash_object_command(args: HashObjectArgs) -> io::Result<()> {
    if args.obj_type != "blob" {
        unimplemented!("only 'blob' object type is currently supported");
    }

    if args.stdin {
        let stdin = stdin();
        hash_object(&args, stdin)?;
    } else if let Some(paths) = &args.files {
        let files = paths
            .iter()
            .map(PathBuf::from)
            .map(File::open)
            .collect::<Vec<io::Result<File>>>();

        for file in files {
            debug!("hash_object_command() processing: {:?}", file);
            match file {
                Ok(f) => {
                    hash_object(&args, f)?;
                }
                Err(e) => return Err(e),
            }
        }
    } else {
        debug!("{:?}", args);
        unimplemented!("args not supported: {:?}", args);
    };

    Ok(())
}

fn read_content_to_hash(mut file: impl Read) -> io::Result<(Vec<u8>, usize)> {
    let mut input = Vec::new();
    let read = file.read_to_end(&mut input)?;
    Ok((input, read))
}

fn hash_object(args: &HashObjectArgs, file: impl Read) -> io::Result<()> {
    let (mut input, read) = read_content_to_hash(file)?;
    let obj_header = format!("{} {read}\0", args.obj_type);
    let obj_header = obj_header.as_bytes();
    let mut buf = Vec::with_capacity(obj_header.len() + input.len());
    buf.append(&mut obj_header.to_vec());
    buf.append(&mut input);

    let hash = generate_hash(&buf);
    let encoded = encode_obj_content(&buf)?;

    if args.write_to_db {
        write_object(&encoded, &hash)?;
    }

    println!("{}", hash);

    Ok(())
}

fn generate_hash(buf: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(buf);
    let sha1_hash = hasher.finalize();
    hex::encode(sha1_hash)
}

fn encode_obj_content(content: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content)?;
    let result = encoder.finish()?;
    Ok(result)
}

fn write_object(encoded: &[u8], hash: &str) -> io::Result<()> {
    let (dir, name) = hash.split_at(2);
    let git_object_dir = get_git_object_dir();
    let full_dir = git_object_dir.join(dir);
    let file_path = full_dir.join(name);
    fs::create_dir_all(full_dir)?;

    debug!("writing to {}", file_path.display());

    let mut file = File::create(file_path)?;
    file.write_all(encoded)?;

    Ok(())
}
