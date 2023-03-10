mod controller;
mod device;
mod utils;
mod file_meta;
mod key_object;

use serde::{Serialize, Deserialize};
use log::{debug, error, log_enabled, info, Level};

use tokio::net::{UdpSocket,TcpListener,TcpStream};
use tokio::io::{self, AsyncRead, AsyncWrite, AsyncReadExt,AsyncWriteExt};
use std::io::Write;
use std::io::Read;

use std::net::{Ipv4Addr, SocketAddr};

use std::sync::{Arc,Mutex};

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
