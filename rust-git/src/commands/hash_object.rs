use crate::commands::{GitCommandResult, GitError, GitResult};
use crate::util;
use clap::Args;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use log::debug;
use sha1::digest::FixedOutputReset;
use sha1::{Digest, Sha1};
use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, Write};
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

        let files = paths
            .iter()
            .map(PathBuf::from)
            .map(File::open)
            .collect::<Vec<io::Result<File>>>();

        for file in files {
            debug!("hash_object_command() processing: {:?}", file);
            match file {
                Ok(mut f) => {
                    hash_object(&args, &mut f)?;
                }
                Err(e) => return Err(GitError::Io { source: e }),
            }
        }
    } else {
        unimplemented!("args not supported: {:?}", args);
    };

    Ok(())
}

fn hash_object(args: &HashObjectArgs, input: &mut File) -> GitCommandResult {
    let temp_file = &make_temp_file()?;

    let mut hash_writer = HashObjectWriter::new(temp_file);

    encode_content(input, &mut hash_writer)?;

    let hash = hash_writer.hash();

    if args.write_to_db {
        let obj_dir = format!("{}/{}", util::get_git_object_dir().display(), &hash[..2]);
        fs::create_dir_all(&obj_dir)?;
        let from = temp_file.path().display().to_string();
        let to = format!("{}/{}", &obj_dir, &hash[2..]);
        debug!("moving {} to {}", from, to);
        fs::rename(from, to)?;
    }

    println!("{hash}");

    Ok(())
}

fn encode_content<W: Write>(input: &mut File, writer: W) -> GitResult<()> {
    let mut encoding_writer = EncodingWriter::new(writer);

    let len = input.metadata()?.len();
    let header = format!("blob {}\0", len);
    debug!("header: '{}'", header);
    write!(encoding_writer, "{}", &header)?;

    std::io::copy(input, &mut encoding_writer)?;

    encoding_writer.finalize()?;

    Ok(())
}

fn hash_object_stdin(args: &HashObjectArgs) -> GitCommandResult {
    let mut temp_file = make_temp_file()?;
    let mut stdin = stdin();

    std::io::copy(&mut stdin, &mut temp_file)?;

    hash_object(args, &mut temp_file.reopen()?)
}

fn make_temp_file() -> GitResult<NamedTempFile> {
    let temp_file = Builder::new().prefix("rg").suffix(".tmp").tempfile()?;
    debug!("temp file: {:?}", temp_file.path());
    Ok(temp_file)
}

struct HashObjectWriter<W: Write> {
    hasher: Sha1,
    writer: W,
}

impl<W: Write> HashObjectWriter<W> {
    fn new(writer: W) -> Self {
        HashObjectWriter {
            hasher: Sha1::new(),
            writer,
        }
    }

    fn hash(&mut self) -> String {
        let sha1 = self.hasher.finalize_fixed_reset();
        hex::encode(sha1)
    }
}

impl<W: Write> Write for HashObjectWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.update(buf);
        let n = self.writer.write(buf)?;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct EncodingWriter<W: Write> {
    encoder: ZlibEncoder<W>,
}

impl<W: Write> EncodingWriter<W> {
    fn new(writer: W) -> Self {
        EncodingWriter {
            encoder: ZlibEncoder::new(writer, Compression::default()),
        }
    }

    fn finalize(&mut self) -> io::Result<()> {
        self.encoder.try_finish()
    }
}

impl<W: Write> Write for EncodingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.encoder.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.encoder.flush()
    }
}
