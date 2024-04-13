use color_eyre::eyre::Result;
use log::info;

mod config;

fn main() -> Result<()> {
    log4rs::init_file("log_config.yaml", Default::default()).unwrap();
    #[cfg(feature = "pretty-backtrace")]
    color_eyre::install()?;

    info!("loading config");
    let _ = config::load_config("config.yaml")?;

    panic!("oh crap!");
}

