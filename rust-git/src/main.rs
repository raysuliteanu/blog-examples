use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, BufReader, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::{env, fs, io, path};

use clap::{command, Args, Parser, Subcommand};
use flate2::bufread::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lazy_regex::regex_captures;
use lazy_static::lazy_static;
use log::{debug, trace};
use sha1::{Digest, Sha1};

use crate::GitObjectType::{Blob, Commit, Tree};

const GIT_DEFAULT_BRANCH_NAME: &str = "master";
const GIT_DIR_NAME: &str = ".git";
const GIT_OBJ_DIR_NAME: &str = "objects";
const GIT_OBJ_BRANCHES_DIR_NAME: &str = "objects/branches";
const GIT_OBJ_HOOKS_DIR_NAME: &str = "objects/hooks";
const GIT_OBJ_INFO_DIR_NAME: &str = "objects/info";
const GIT_OBJ_PACK_DIR_NAME: &str = "objects/pack";
const _GIT_REFS_DIR_NAME: &str = "refs";
const GIT_REFS_HEADS_DIR_NAME: &str = "refs/heads";
const GIT_REFS_TAGS_DIR_NAME: &str = "refs/tags";
const GIT_USER_CONFIG_FILE_NAME: &str = ".gitconfig";

lazy_static! {
    static ref GIT_CONFIG: HashMap<String, String> =
        load_git_config().unwrap_or_else(|_| HashMap::default());

    // NOTE: GIT_PARENT_DIR will panic if used during 'init' command processing
    // since there of course is no .git dir to find the parent of yet!
    static ref GIT_PARENT_DIR: PathBuf = find_git_parent_dir();

    static ref GIT_HEAD: PathBuf = PathBuf::from("HEAD");
    static ref GIT_REPO_CONFIG_FILE: PathBuf = PathBuf::from("config");
}

#[derive(Debug)]
pub enum GitObjectType {
    Blob,
    Tree,
    Commit,
}

impl From<String> for GitObjectType {
    fn from(value: String) -> Self {
        GitObjectType::from(value.as_str())
    }
}

impl From<&str> for GitObjectType {
    fn from(value: &str) -> Self {
        match value {
            "blob" => Blob,
            "tree" => Tree,
            "commit" => Commit,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Parser)]
struct Git {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init(InitArgs),
    CatFile(CatFileArgs),
    HashObject(HashObjectArgs),
    Config(ConfigArgs),
}

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
    #[arg(long)]
    shared: Option<String>,
    directory: Option<OsString>,
}

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
struct ConfigArgs {
    #[arg(short, long, default_value = "false")]
    list: bool,
    #[arg(long, default_value = "false")]
    global: bool,
    #[arg(long, default_value = "false")]
    system: bool,
    #[arg(long, default_value = "false")]
    local: bool,
}

// git hash-object [-t <type>] [-w] [--path=<file>|--no-filters] [--stdin [--literally]] [--] <file>...
// git hash-object [-t <type>] [-w] --stdin-paths [--no-filters]
#[derive(Debug, Args)]
struct HashObjectArgs {
    #[arg(short = 't', default_value = "blob")]
    obj_type: String,
    #[arg(short = 'w', default_value = "false")]
    write_to_db: bool,
    #[arg(long, default_value = "false")]
    stdin: bool,
    #[arg(long, default_value = "false")]
    literally: bool,
    #[arg(last = true)]
    file: Option<Vec<OsString>>,
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

fn main() -> io::Result<()> {
    env_logger::init();

    let git = Git::parse();

    match git.command {
        Commands::Init(args) => init_command(args),
        Commands::CatFile(args) => cat_file_command(args),
        Commands::HashObject(args) => hash_object_command(args),
        Commands::Config(args) => config_command(args),
    }
}

fn config_command(args: ConfigArgs) -> io::Result<()> {
    if args.list {
        // todo: filter by local/system/global; if none, print all
        GIT_CONFIG
            .iter()
            .for_each(|entry| println!("{}={}", entry.0, entry.1))
    }

    Ok(())
}

fn hash_object_command(args: HashObjectArgs) -> io::Result<()> {
    if args.obj_type != "blob" {
        unimplemented!("only 'hash' object type is currently supported");
    }

    if args.stdin {
        let mut stdin = stdin();
        let mut input = Vec::new();
        let read = stdin.read_to_end(&mut input)?;

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
    } else {
        debug!("{:?}", args);
        unimplemented!("only stdin is currently supported")
    }

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

fn cat_file_command(args: CatFileArgs) -> io::Result<()> {
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
            Commit => unimplemented!("commit object type currently not supported"),
        }
    } else if args.obj_type {
        println!("{obj_type}");
    } else if args.show_size {
        println!("{obj_len}");
    } else {
        // todo: work on the errors
        return Err(io::Error::from(ErrorKind::Other));
    }

    Ok(())
}

fn get_object_header(decoded_content: &mut [u8], index: usize) -> (String, String) {
    let header = &mut decoded_content[0..index].split(|x| *x == b' ');
    let obj_type = bytes_to_string(header.next().unwrap());
    let obj_len = bytes_to_string(header.next().unwrap());
    (obj_type, obj_len)
}

fn get_object(object: &str, decoded_content: &mut Vec<u8>) -> io::Result<()> {
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

fn decode_obj_content(file: File, decoded_content: &mut Vec<u8>) -> io::Result<()> {
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

fn init_command(args: InitArgs) -> io::Result<()> {
    let (git_parent_dir, separate_git_dir) = get_git_dirs(args.directory, args.separate_git_dir)?;

    debug!(
        "git dir: {:?}\tseparate dir: {:?}",
        git_parent_dir, separate_git_dir
    );

    let actual_git_parent_dir = match separate_git_dir {
        Some(dir) => {
            // make link to dir
            if !git_parent_dir.exists() {
                debug!("creating {:?}", git_parent_dir);
                fs::create_dir_all(&git_parent_dir)?;
            }

            let dot_git_file = git_parent_dir.join(GIT_DIR_NAME);
            debug!("creating {:?}", dot_git_file);
            fs::write(&dot_git_file, format!("gitdir: {}\n", dir.display()))?;

            dir
        }
        None => {
            if !git_parent_dir.exists() {
                debug!("creating {:?}", git_parent_dir);
                fs::create_dir_all(&git_parent_dir)?;
            }

            git_parent_dir.join(GIT_DIR_NAME)
        }
    };

    for dir in [
        GIT_OBJ_BRANCHES_DIR_NAME,
        GIT_OBJ_HOOKS_DIR_NAME,
        GIT_OBJ_PACK_DIR_NAME,
        GIT_OBJ_INFO_DIR_NAME,
        GIT_REFS_TAGS_DIR_NAME,
        GIT_REFS_HEADS_DIR_NAME,
    ] {
        let path = actual_git_parent_dir.join(dir);
        fs::create_dir_all(&path)?;
        trace!("created {}", &path.display());
    }

    let path_buf = actual_git_parent_dir.join(GIT_HEAD.as_path());

    let branch_name =
        args.initial_branch
            .unwrap_or_else(|| match GIT_CONFIG.get("init.defaultBranch") {
                Some(branch) => branch.to_string(),
                None => GIT_DEFAULT_BRANCH_NAME.to_string(),
            });

    fs::write(
        path_buf.as_path(),
        format!("ref: refs/heads/{branch_name}\n"),
    )?;

    let mut dot_git_config = String::from(
        r"[core]
    repositoryformatversion = 0
    filemode = true
    logallrefupdates = true
    ",
    );

    dot_git_config.push_str(format!("bare = {}\n\n", args.bare).as_str());

    let config_file_path = actual_git_parent_dir.join(GIT_REPO_CONFIG_FILE.as_path());
    fs::write(config_file_path.as_path(), dot_git_config)?;

    println!(
        "Initialized empty Git repository in {}",
        actual_git_parent_dir.display()
    );

    Ok(())
}

fn get_git_dirs(
    directory: Option<OsString>,
    separate_git_dir: Option<OsString>,
) -> io::Result<(PathBuf, Option<PathBuf>)> {
    let git_parent_dir = if let Some(dir) = directory {
        path::absolute(dir.to_str().unwrap()).unwrap()
    } else {
        env::current_dir()?
    };

    let separate_parent_dir =
        separate_git_dir.map(|dir| path::absolute(dir.to_str().unwrap()).unwrap());

    Ok((git_parent_dir, separate_parent_dir))
}

fn find_git_parent_dir() -> PathBuf {
    let current_dir = env::current_dir().expect("failed to get current directory");
    let mut current_dir = current_dir.to_path_buf();

    loop {
        let git_dir = current_dir.join(GIT_DIR_NAME);
        if git_dir.is_dir() {
            debug!("found .git dir: {:?}", git_dir.parent().unwrap());
            return git_dir.parent().unwrap().to_path_buf();
        }

        if !current_dir.pop() {
            break;
        }
    }

    panic!("not a git repository (or any of the parent directories): .git")
}

/// Load the contents of ~/.gitconfig if it exists, returning a map of config items as key/value pairs
/// Section headers are prefixed to individual config item names e.g.
/// ```
/// [init]
/// defaultBranch = foo
/// ```
/// becomes `init.defaultBranch` in the map as the key for the value `foo`.
///
/// _NOTE_: since the Git config format is not standard (not INI not TOML) gotta do it myself
///
/// _TODO_: load and merge the global git config if it exists, and be able to differentiate local/global/system
fn load_git_config() -> io::Result<HashMap<String, String>> {
    let mut config = HashMap::new();
    if let Some(home_dir) = dirs::home_dir() {
        let git_config_path = home_dir.join(GIT_USER_CONFIG_FILE_NAME);
        if git_config_path.try_exists().is_ok() {
            let mut file = File::open(git_config_path)?;
            let buf = &mut String::new();
            let _ = file.read_to_string(buf);
            let mut section = "";
            for it in buf.split_terminator('\n') {
                let line = it.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                    continue;
                }

                if let Some((_whole, matched)) = regex_captures!(r#"\[(.+)\]"#, line) {
                    section = matched;
                    continue;
                }

                let (key, value) = get_config_pair(line);
                let full_key = [section, key].join(".");
                debug!("adding config: {}={}", full_key, value);
                config.insert(full_key, String::from(value));
            }
        }
    }

    Ok(config)
}

fn get_config_pair(line: &str) -> (&str, &str) {
    let mut parts = line.split('=');
    let key = parts.next().unwrap().trim();
    let value = parts.next().unwrap().trim();

    (key, value)
}

fn get_git_object_dir() -> PathBuf {
    GIT_PARENT_DIR.join(GIT_DIR_NAME).join(GIT_OBJ_DIR_NAME)
}
