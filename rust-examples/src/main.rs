use log::info;

fn main() {
    log4rs::init_file("log_config.yaml", Default::default()).unwrap();
    info!("Hello, world!");
}
