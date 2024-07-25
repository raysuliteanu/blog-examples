use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, BufReader, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};

use clap::{command, Args, Parser, Subcommand};
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lazy_static::lazy_static;
use log::{debug, trace};
use sha1::{Digest, Sha1};

use crate::GitObjectType::{Blob, Commit, Tree};

const GIT_DIR_NAME: &str = ".git";
const GIT_OBJ_DIR_NAME: &str = ".git/objects";
const GIT_OBJ_BRANCHES_DIR_NAME: &str = ".git/objects/branches";
const GIT_OBJ_HOOKS_DIR_NAME: &str = ".git/objects/hooks";
const GIT_OBJ_INFO_DIR_NAME: &str = ".git/objects/info";
const GIT_OBJ_PACK_DIR_NAME: &str = ".git/objects/pack";
const GIT_REFS_DIR_NAME: &str = ".git/refs";
const GIT_REFS_HEADS_DIR_NAME: &str = ".git/refs/heads";
const GIT_REFS_TAGS_DIR_NAME: &str = ".git/refs/tags";

lazy_static! {
    static ref GIT_PARENT_DIR: PathBuf = find_git_parent_dir();
    static ref GIT_HEAD: PathBuf = GIT_PARENT_DIR.join(".git/HEAD");
    static ref GIT_CONFIG: PathBuf = GIT_PARENT_DIR.join(".git/config");
}

#[derive(Debug)]
pub enum GitObjectType {
    Blob,
    Tree,
    Commit,
}

impl From<String> for GitObjectType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "blob" => Blob,
            "tree" => Tree,
            "commit" => Commit,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "git")]
struct Git {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init(InitArgs),
    CatFile(CatFileArgs),
    HashObject(HashObjectArgs),
}

/*
       git hash-object [-t <type>] [-w] [--path=<file>|--no-filters] [--stdin [--literally]] [--] <file>...
       git hash-object [-t <type>] [-w] --stdin-paths [--no-filters]
*/
#[derive(Debug, Args)]
struct HashObjectArgs {
    #[arg(short = 't', default_value = "blob")]
    obj_type: String,
    #[arg(short = 'w', default_value = "false")]
    write_to_db: bool,
    #[arg(long)]
    path: Option<OsString>,
    #[arg(long, default_value = "false")]
    no_filters: bool,
    #[arg(long, default_value = "false")]
    stdin: bool,
    #[arg(long, default_value = "false")]
    literally: bool,
    #[arg(long)]
    stdin_paths: bool,
    #[arg(last = true)]
    file: Option<Vec<OsString>>,
}

/*
git init [-q | --quiet] [--bare] [--template=<template_directory>]
                [--separate-git-dir <git dir>] [--object-format=<format>]
                [-b <branch-name> | --initial-branch=<branch-name>]
                [--shared[=<permissions>]] [directory]
*/
#[derive(Debug, Args)]
struct InitArgs {
    #[arg(short, long, default_value_t)]
    quiet: bool,
    #[arg(long, default_value_t)]
    bare: bool,
    #[arg(long)]
    template: Option<OsString>,
    #[arg(long)]
    separate_git_dir: Option<OsString>,
    #[arg(long, default_value = "sha1")]
    object_format: String,
    #[arg(short = 'b', long)]
    initial_branch: Option<String>,
    // false|true|umask|group|all|world|everybody|0xxx
    #[arg(long)]
    shared: Option<String>,
    directory: Option<OsString>,
}

/*
git cat-file (-t [--allow-unknown-type]| -s [--allow-unknown-type]| -e | -p | <type> | --textconv | --filters ) [--path=<path>] <object>
git cat-file (--batch[=<format>] | --batch-check[=<format>]) [ --textconv | --filters ] [--follow-symlinks]
 */
#[derive(Debug, Args)]
struct CatFileArgs {
    #[arg(short, default_value = "false")]
    pretty: bool,
    #[arg(short = 't', default_value = "false")]
    obj_type: bool,
    #[arg(short, default_value = "false")]
    show_size: bool,
    #[arg(long, default_value = "false")]
    allow_unknown_type: bool,
    #[arg(short, default_value = "false")]
    exists: bool,
    object: String,
}

// todos:
// - expand on the Clap details for help and such
// - handle errors better e.g. "custom" errors this thiserror/anyhow crates

fn main() -> std::io::Result<()> {
    env_logger::init();

    let git = Git::parse();

    match git.command {
        Commands::Init(args) => init_command(args),
        Commands::CatFile(args) => cat_file_command(args),
        Commands::HashObject(args) => hash_object_command(args),
    }
}

fn hash_object_command(args: HashObjectArgs) -> std::io::Result<()> {
    if args.stdin {
        let mut stdin = stdin();
        let mut buf_in = Vec::new();
        let read = stdin.read_to_end(&mut buf_in)?;
        let mut header = args.obj_type;
        if header == "blob" {
            header.push_str(format!(" {read}\0").as_str());
        } else {
            unimplemented!();
        }

        let mut buf = Vec::from(header);
        buf.append(&mut buf_in);

        let mut hasher = Sha1::new();
        hasher.update(&buf[..]);
        let sha1_hash = hasher.finalize();

        let encoded = encode_obj_content(&mut buf)?;

        let hash = hex::encode(sha1_hash);

        if args.write_to_db {
            write_object(&encoded, &hash)?;
        }

        println!("{}", hash);
    } else {
        unimplemented!("not implemented yet")
    }

    Ok(())
}

fn write_object(encoded: &[u8], hash: &str) -> std::io::Result<()> {
    let (dir, name) = hash.split_at(2);
    let full_dir = format!("{}/{}/{}", GIT_PARENT_DIR.display(), GIT_OBJ_DIR_NAME, dir);
    let full_dir = full_dir.as_str();
    debug!("writing to {full_dir}");
    if !Path::new(full_dir).exists() {
        debug!("creating full dir");
        fs::create_dir(full_dir)?;
    }

    let file_path = Path::new(full_dir).join(name);
    debug!("writing to {}", file_path.display());

    let mut file = File::create(file_path)?;
    file.write_all(encoded)?;
    Ok(())
}

fn encode_obj_content(content: &mut [u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content)?;
    let result = encoder.finish()?;
    Ok(result)
}

fn cat_file_command(args: CatFileArgs) -> std::io::Result<()> {
    let decoded_content = &mut Vec::new();

    get_object(&args.object, decoded_content)?;

    let index = find_null_byte_index(decoded_content);

    let (obj_type, obj_len) = get_object_header(decoded_content, index);

    let content = &decoded_content[index + 1..];

    if args.pretty {
        match GitObjectType::from(obj_type) {
            Blob => {
                print!("{}", bytes_to_string(content));
            }
            Tree => {
                // each line of content is of the form
                // [filemode] [filename]\0[sizeof(sha1_hash)==20b]
                let mut consumed = 0usize;
                let len = obj_len.as_str().parse::<usize>().expect("invalid length");
                while consumed < len {
                    let index = find_null_byte_index(&content[consumed..]);
                    let end = consumed + index;
                    assert!(end < content.len());
                    let tree_row_prefix = &mut content[consumed..end].split(|x| *x == b' ');
                    let mode = bytes_to_string(tree_row_prefix.next().unwrap());
                    let file = bytes_to_string(tree_row_prefix.next().unwrap());
                    consumed += index + 1;
                    let hash = hex::encode(&content[consumed..consumed + 20]);
                    consumed += 20;
                    let tmp_buf = &mut Vec::new();
                    get_object(hash.as_str(), tmp_buf)?;
                    let index = find_null_byte_index(tmp_buf);
                    let (obj_type, _) = get_object_header(tmp_buf, index);
                    println!("{:0>6} {} {}    {}", mode, obj_type, hash, file);
                }
            }
            Commit => {}
        }
    } else if args.obj_type {
        println!("{obj_type}");
    } else if args.show_size {
        println!("{obj_len}");
    } else {
        // todo: work on the errors
        return Err(std::io::Error::from(ErrorKind::Other));
    }

    Ok(())
}

fn get_object_header(decoded_content: &mut [u8], index: usize) -> (String, String) {
    let header = &mut decoded_content[0..index].split(|x| *x == b' ');
    let obj_type = bytes_to_string(header.next().unwrap());
    let obj_len = bytes_to_string(header.next().unwrap());
    (obj_type, obj_len)
}

fn get_object(object: &str, decoded_content: &mut Vec<u8>) -> std::io::Result<()> {
    let object_file = get_object_file(object);

    if let Ok(file) = File::open(object_file) {
        decode_obj_content(file, decoded_content)?;
    }

    Ok(())
}

fn find_null_byte_index(content: &[u8]) -> usize {
    debug!("{:?}", content);
    for (i, v) in content.iter().enumerate() {
        if *v == 0 {
            return i;
        }
    }

    content.len()
}

fn bytes_to_string(content: &[u8]) -> String {
    content
        .iter()
        .map(|b| *b as char)
        .fold(String::new(), |mut acc, c| {
            acc.push(c);
            acc
        })
}

fn decode_obj_content(file: File, decoded_content: &mut Vec<u8>) -> std::io::Result<()> {
    let content: &mut Vec<u8> = &mut Vec::new();
    let mut reader = BufReader::new(file);
    let _ = reader.read_to_end(content);
    let mut decoder = ZlibDecoder::new(&content[..]);
    decoder.read_to_end(decoded_content)?;

    Ok(())
}

fn get_object_file(obj_id: &str) -> PathBuf {
    if obj_id.len() < 3 {
        panic!("Not a valid object name {obj_id}")
    }
    let (dir, id) = obj_id.split_at(2);
    let obj_dir = GIT_PARENT_DIR.join(GIT_OBJ_DIR_NAME).join(dir);
    if !obj_dir.exists() || !obj_dir.is_dir() {
        debug!("can't access {}", obj_dir.display());
        panic!("Not a valid object name {obj_id}")
    }

    let mut obj_file = obj_dir.join(id);
    if !obj_file.exists() || !obj_file.is_file() {
        // maybe not a full hash so do a partial match
        for entry in obj_dir
            .read_dir()
            .unwrap_or_else(|_| panic!("Not a valid object name {obj_id}"))
            .flatten()
        {
            let os_string = entry.file_name();
            let filename = os_string.to_str().unwrap();
            if filename.starts_with(id) {
                obj_file = obj_dir.join(filename);
            }
        }
    }

    debug!("found {:?}", obj_file);
    obj_file
}

// todo: support options in args
fn init_command(_args: InitArgs) -> std::io::Result<()> {
    fs::create_dir(GIT_DIR_NAME)?;
    trace!("created {GIT_DIR_NAME}");
    fs::create_dir(GIT_OBJ_DIR_NAME)?;
    trace!("created {GIT_OBJ_DIR_NAME}");
    fs::create_dir(GIT_OBJ_BRANCHES_DIR_NAME)?;
    trace!("created {GIT_OBJ_BRANCHES_DIR_NAME}");
    fs::create_dir(GIT_OBJ_HOOKS_DIR_NAME)?;
    trace!("created {GIT_OBJ_HOOKS_DIR_NAME}");
    fs::create_dir(GIT_OBJ_INFO_DIR_NAME)?;
    trace!("created {GIT_OBJ_INFO_DIR_NAME}");
    fs::create_dir(GIT_OBJ_PACK_DIR_NAME)?;
    trace!("created {GIT_OBJ_PACK_DIR_NAME}");
    fs::create_dir(GIT_REFS_DIR_NAME)?;
    trace!("created {GIT_REFS_DIR_NAME}");
    fs::create_dir(GIT_REFS_TAGS_DIR_NAME)?;
    trace!("created {GIT_REFS_TAGS_DIR_NAME}");
    fs::create_dir(GIT_REFS_HEADS_DIR_NAME)?;
    trace!("created {GIT_REFS_HEADS_DIR_NAME}");

    // todo: initial head pointer should come from
    // -b <name> or --initial-branch=<name> or
    // ~/.gitconfig/init.defaultBranch or
    // 'master'
    fs::write(GIT_HEAD.as_path(), "ref: refs/heads/main\n")?;

    // todo: write config
    fs::write(GIT_CONFIG.as_path(), "")?;

    println!(
        "Initialized empty Git repository in {}/{}",
        env::current_dir()?.display(),
        GIT_DIR_NAME
    );

    Ok(())
}

fn find_git_parent_dir() -> PathBuf {
    let current_dir = env::current_dir().expect("failed to get current directory");
    let mut current_dir = current_dir.to_path_buf();

    loop {
        let git_dir = current_dir.join(GIT_DIR_NAME);
        if git_dir.is_dir() {
            return git_dir.parent().unwrap().to_path_buf();
        }

        if !current_dir.pop() {
            break;
        }
    }

    panic!("not a git repository (or any of the parent directories): .git")
}
