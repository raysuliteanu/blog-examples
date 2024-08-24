use crate::commands::GitCommandResult;
use clap::Args;
use lazy_regex::regex_captures;
use lazy_static::lazy_static;
use log::debug;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;

const GIT_USER_CONFIG_FILE_NAME: &str = ".gitconfig";

lazy_static! {
    pub(crate) static ref GIT_CONFIG: HashMap<String, String> =
        load_git_config().unwrap_or_else(|_| HashMap::default());
}

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub(crate) struct ConfigArgs {
    #[arg(short, long, default_value = "false")]
    pub(crate) list: bool,
    #[arg(long, default_value = "false")]
    pub(crate) global: bool,
    #[arg(long, default_value = "false")]
    pub(crate) system: bool,
    #[arg(long, default_value = "false")]
    pub(crate) local: bool,
}

pub(crate) fn config_command(args: ConfigArgs) -> GitCommandResult {
    if args.list {
        // todo: filter by local/system/global; if none, print all
        GIT_CONFIG
            .iter()
            .for_each(|entry| println!("{}={}", entry.0, entry.1))
    }

    Ok(())
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
pub(crate) fn load_git_config() -> io::Result<HashMap<String, String>> {
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
