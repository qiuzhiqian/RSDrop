mod controller;
mod device;
mod utils;
mod file_meta;
mod key_object;

use log::debug;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    //env_logger::init();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    debug!("start simple rust drop");
    let mut controller = controller::Controller::new();
    controller.start_discovery_service().await?;
    controller.start_service().await?;
    controller.cmd_loop().await?;
    Ok(())
}
