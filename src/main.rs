mod controller;
mod device;
mod utils;
mod file_meta;
mod key_object;
mod ui;
mod components;

use log::debug;

fn main() -> std::io::Result<()> {
    //env_logger::init();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    debug!("start simple rust drop");
    
    ui::start()?;
    Ok(())
}