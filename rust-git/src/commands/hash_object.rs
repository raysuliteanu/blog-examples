use crate::commands::{GitCommandResult, GitResult};
use crate::util;
use clap::Args;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use log::debug;
use sha1::{Digest, Sha1};
use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, BufWriter, Write};
use std::path::PathBuf;
use std::{fs, io};
use tempfile::{Builder, NamedTempFile};

#[derive(Debug, Args)]
pub(crate) struct HashObjectArgs {
    #[arg(short = 't', default_value = "blob")]
    pub(crate) obj_type: String,
    #[arg(short, default_value = "false")]
    pub(crate) write_to_db: bool,
    #[arg(long, default_value = "false")]
    pub(crate) stdin: bool,
    #[arg(long, default_value = "false")]
    pub(crate) literally: bool,
    pub(crate) files: Option<Vec<OsString>>,
}

pub(crate) fn hash_object_command(args: HashObjectArgs) -> GitCommandResult {
    if args.obj_type != "blob" {
        unimplemented!("only 'blob' object type is currently supported");
    }

    if args.stdin {
        hash_object_stdin(&args)?;
    } else if let Some(paths) = &args.files {
        let paths = if paths.len() > 1 && paths.first() == Some(&OsString::from("--")) {
            // Git hash-object works with or without specifying '--' before file list
            &paths[1..]
        } else {
            &paths[..]
        };

        paths
            .iter()
            .map(PathBuf::from)
            .map(File::open)
            .filter_map(Result::ok)
            .try_for_each(|mut f| hash_object(&args, &mut f))?;
    } else {
        unimplemented!("args not supported: {:?}", args);
    };

    Ok(())
}

fn hash_object_stdin(args: &HashObjectArgs) -> GitCommandResult {
    let mut temp_file = make_temp_file()?;
    let mut stdin = stdin();

    std::io::copy(&mut stdin, &mut temp_file)?;

    hash_object(args, &mut temp_file.reopen()?)
}

fn hash_object(args: &HashObjectArgs, input: &mut File) -> GitCommandResult {
    let (path, hash) = encode_content(input)?;

    if args.write_to_db {
        let obj_dir = format!("{}/{}", util::get_git_object_dir().display(), &hash[..2]);
        fs::create_dir_all(&obj_dir)?;
        let to = format!("{}/{}", &obj_dir, &hash[2..]);
        debug!("moving {} to {}", path.display(), to);
        fs::rename(path, to)?;
    }

    println!("{hash}");

    Ok(())
}

fn make_temp_file() -> GitResult<NamedTempFile> {
    let temp_file = Builder::new().prefix("rg").suffix(".tmp").tempfile()?;
    debug!("temp file: {:?}", temp_file.path());
    Ok(temp_file)
}

fn encode_content(input: &mut File) -> GitResult<(PathBuf, String)> {
    let file = make_temp_file()?;
    let writer = BufWriter::new(&file);
    let mut hasher = HashObjectWriter::new(writer);

    let len = input.metadata()?.len();
    let header = format!("blob {}\0", len);
    debug!("header: '{}'", header);
    write!(hasher, "{}", &header)?;

    std::io::copy(input, &mut hasher)?;


    Ok((file.path().to_path_buf(), hash(hasher)))
}

struct HashObjectWriter<W: Write> {
    encoder: ZlibEncoder<W>,
    hasher: Sha1,
}

impl<W: Write> HashObjectWriter<W> {
    fn new(writer: W) -> Self {
        HashObjectWriter {
            hasher: Sha1::new(),
            encoder: ZlibEncoder::new(writer, Compression::default()),
        }
    }
}

    fn hash<W: Write>(how: HashObjectWriter<W>) -> String {
        let _ = how.encoder.finish();
        let sha1 = how.hasher.finalize();
        hex::encode(sha1)
    }

impl<W: Write> Write for HashObjectWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.update(buf);
        let n = self.encoder.write(buf)?;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

