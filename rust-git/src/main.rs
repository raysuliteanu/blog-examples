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
// todo: add help
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
    if args.pretty {
        let object_file = get_object_file(&args.object);

        // todo: need to support partial lookup i.e. obj id could be 'abc123' vs full hash
        // so need to support looking for 'ab/c123*'
        if let Ok(file) = File::open(object_file) {
            let decoded_content = &mut Vec::new();
            decode_obj_content(file, decoded_content)?;

            let data: String = decoded_content
                .iter()
                .skip_while(|b| **b != 0)
                .skip(1)
                .map(|b| *b as char)
                .fold(String::new(), |mut acc, c| {
                    acc.push(c);
                    acc
                });
            print!("{data}");

            Ok(())
        } else {
            // todo: work on the errors
            Err(std::io::Error::from(ErrorKind::PermissionDenied))
        }
    } else {
        // todo: work on the errors
        Err(std::io::Error::from(ErrorKind::Other))
    }
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
    let (dir, id) = obj_id.split_at(2);
    let obj_dir = GIT_PARENT_DIR.join(GIT_OBJ_DIR_NAME).join(dir);
    if !obj_dir.exists() || !obj_dir.is_dir() {
        eprintln!("can't access {:#?}", obj_dir);
    }

    let obj_file = obj_dir.join(id);
    if !obj_file.exists() || !obj_file.is_file() {
        eprintln!("can't access {:#?}", obj_file);
    }

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
