use std::io;
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("could not load configuration")]
    ConfigLoadError(#[from] io::Error),

    #[error("invalid config file format: {0}")]
    ConfigParseError(String),

    #[error("unknown config property '{key}:{value}'")]
    UnknownConfigProperty {
        key: String,
        value: String,
    },
}

#[derive(Deserialize)]
pub struct MyConfig {
    value: String,
}

pub fn load_config(file_name: &str) -> Result<MyConfig, ConfigError> {
    let config_str = std::fs::read_to_string(file_name)?;
    let config: MyConfig = toml::from_str(&config_str)
        .map_err(|e| ConfigError::ConfigParseError(e.to_string()))?;
    Ok(config)
}
