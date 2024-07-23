use std::fs::File;
use std::io::{stdin, BufReader, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::{env, fs};

use clap::{command, Args, Parser, Subcommand};
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use log::{debug, trace};
use sha1::{Digest, Sha1};

const GIT_DIR: &str = ".git";
const GIT_OBJ_DIR: &str = ".git/objects";
const GIT_OBJ_BRANCHES_DIR: &str = ".git/objects/branches";
const GIT_OBJ_HOOKS_DIR: &str = ".git/objects/hooks";
const GIT_OBJ_INFO_DIR: &str = ".git/objects/info";
const GIT_OBJ_PACK_DIR: &str = ".git/objects/pack";
const GIT_REFS_DIR: &str = ".git/refs";
const GIT_REFS_HEADS_DIR: &str = ".git/refs/heads";
const GIT_REFS_TAGS_DIR: &str = ".git/refs/tags";
const GIT_HEAD: &str = ".git/HEAD";
const _GIT_CONFIG: &str = ".git/config";

#[derive(Debug, Parser)]
#[command(name = "git")]
struct Git {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init(InitArgs),
    #[command(arg_required_else_help = true)]
    CatFile {
        #[arg(short)]
        pretty: bool,
        object: String,
    },
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
    path: Option<String>,
    #[arg(long, default_value = "false")]
    no_filters: bool,
    #[arg(long, default_value = "false")]
    stdin: bool,
    #[arg(long, default_value = "false")]
    literally: bool,
    #[arg(long)]
    stdin_paths: Option<String>,
    file: Option<String>, // list of files I think; also how to support '--' indicating now come the file(s)
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
    template: Option<String>,
    #[arg(long)]
    separate_git_dir: Option<String>,
    #[arg(long, default_value = "sha1")]
    object_format: String,
    #[arg(short = 'b', long)]
    initial_branch: Option<String>,
    // false|true|umask|group|all|world|everybody|0xxx
    #[arg(long)]
    shared: Option<String>,
    directory: Option<String>,
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
        Commands::CatFile {
            pretty,
            object: obj_id,
        } => cat_file_command(pretty, &obj_id),
        Commands::HashObject(args) => hash_object_command(args),
    }
}

fn hash_object_command(args: HashObjectArgs) -> std::io::Result<()> {
    if args.stdin {
        let mut stdin = stdin();
        let mut buf_in = String::new();
        let read = stdin.read_to_string(&mut buf_in)?;
        debug!("read ({}B): {}", read, &buf_in);
        if args.obj_type == "blob" {
            buf_in = format!("blob {read}\0{}", buf_in);
        } else {
            unimplemented!();
        }

        let mut hasher = Sha1::new();
        hasher.update(&buf_in[..]);
        let sha1_hash = hasher.finalize();

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(buf_in.as_bytes())?;
        let _encoded = encoder.finish()?;

        if args.write_to_db {
            unimplemented!("not implemented yet")
        } else {
            println!("{}", hex::encode(sha1_hash));
        }
    } else {
        unimplemented!("not implemented yet")
    }

    Ok(())
}

fn cat_file_command(pretty: bool, obj_id: &String) -> std::io::Result<()> {
    if pretty {
        let object_file = get_object_file(&obj_id);

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

fn get_object_file(obj_id: &&String) -> PathBuf {
    let (dir, id) = parse_obj_id(obj_id);
    let obj_dir = PathBuf::from(GIT_OBJ_DIR).join(dir);
    if !obj_dir.exists() || !obj_dir.is_dir() {
        eprintln!("can't access {:#?}", obj_dir);
    }

    let obj_file = obj_dir.join(id);
    if !obj_file.exists() || !obj_file.is_file() {
        eprintln!("can't access {:#?}", obj_file);
    }

    obj_file
}

fn parse_obj_id(obj_id: &str) -> (&str, &str) {
    obj_id.split_at(2)
}

// todo: support options in args
fn init_command(_args: InitArgs) -> std::io::Result<()> {
    fs::create_dir(GIT_DIR)?;
    trace!("created {GIT_DIR}");
    fs::create_dir(GIT_OBJ_DIR)?;
    trace!("created {GIT_OBJ_DIR}");
    fs::create_dir(GIT_OBJ_BRANCHES_DIR)?;
    trace!("created {GIT_OBJ_BRANCHES_DIR}");
    fs::create_dir(GIT_OBJ_HOOKS_DIR)?;
    trace!("created {GIT_OBJ_HOOKS_DIR}");
    fs::create_dir(GIT_OBJ_INFO_DIR)?;
    trace!("created {GIT_OBJ_INFO_DIR}");
    fs::create_dir(GIT_OBJ_PACK_DIR)?;
    trace!("created {GIT_OBJ_PACK_DIR}");
    fs::create_dir(GIT_REFS_DIR)?;
    trace!("created {GIT_REFS_DIR}");
    fs::create_dir(GIT_REFS_TAGS_DIR)?;
    trace!("created {GIT_REFS_TAGS_DIR}");
    fs::create_dir(GIT_REFS_HEADS_DIR)?;
    trace!("created {GIT_REFS_HEADS_DIR}");

    // todo: initial head pointer should come from
    // -b <name> or --initial-branch=<name> or
    // ~/.gitconfig/init.defaultBranch or
    // 'master'
    fs::write(GIT_HEAD, "ref: refs/heads/main\n")?;

    // todo: create .git/config file

    println!(
        "Initialized empty Git repository in {}/{}",
        env::current_dir()?.display(),
        GIT_DIR
    );

    Ok(())
}
