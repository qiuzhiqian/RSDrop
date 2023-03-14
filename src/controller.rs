mod accepter;
mod connector;
mod discoverer;

use tokio::io;

use rsa::{RsaPrivateKey, RsaPublicKey};

use std::net::SocketAddr;

use log::{debug, info};

use connector::ClientConnector;
use crate::device::{self, Device, RemoteTcpDevice};
use std::sync::{Arc,Mutex};

use eframe::egui;

pub struct Controller {
    discoverers: Vec<discoverer::Discovery>,
    accepters: Vec<accepter::Accepter>,
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    host: device::Device,
    devices: Arc<Mutex<Vec<RemoteTcpDevice>>>,
    ctx: egui::Context,
}

impl Controller {
    pub fn new(ctx: egui::Context) -> Self {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
        let public_key = RsaPublicKey::from(&private_key);
        Self {
            private_key,
            public_key,
            discoverers: Vec::new(),
            accepters: Vec::new(),
            host: Device::default(),
            devices: Arc::new(Mutex::new(Vec::<RemoteTcpDevice>::new())),
            ctx,
        }
    }

    pub fn set_device_container(&mut self,devices: Arc<Mutex<Vec<RemoteTcpDevice>>>) {
        self.devices = devices;
    }

    pub async fn start_discovery_service(&mut self) -> io::Result<tokio::sync::mpsc::Receiver<device::RemoteTcpDevice>> {
        for interface in pnet::datalink::interfaces() {
            if interface.is_up() && !interface.ips.is_empty() && !interface.is_loopback() && !interface.name.contains("docker") {
                for ip in interface.ips {
                    if ip.is_ipv4() {
                        let discoverer = discoverer::Discovery::new(&ip.ip().to_string()).await?;
                        self.discoverers.push(discoverer);

                        let accepter = accepter::Accepter::new(&ip.ip().to_string()).await?;
                        self.accepters.push(accepter);
                    }
                }
            }
        }

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        for disc in &self.discoverers {
            disc.start(&self.host,tx.clone()).await?;
        }
        Ok(rx)
    }

    pub async fn start_service(&mut self) -> io::Result<()> {
        loop {
            if let Some(accepter) = self.accepters.pop() {
                let key = self.public_key.clone();
                tokio::spawn(async move{
                    loop {
                        info!("start tcp server for receive file");
                        let (mut stream,addr) = accepter.accept(&key).await.expect("has error");
                        info!("accept addr {}",addr);
                        tokio::spawn(async move {
                            accepter::Accepter::recv_files(&mut stream).await.expect("receive failed");
                        });
                    }
                });
            } else {
                break;
            }
        }
        
        Ok(())
    }

    pub async fn cmd_loop(&self) -> io::Result<()> {
        info!("run cmd loop");
        loop{
            let line = tokio::task::spawn_blocking(move || {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line).unwrap();
                line
            }).await.unwrap();
            let params: Vec<&str> = line.split_ascii_whitespace().collect();
            debug!("param: {:#?}",params);
            if params.is_empty() {
                continue;
            }
    
            if params[0].to_lowercase() == "add" {
                debug!("add {}",params[1]);
    
                for discoverer in &self.discoverers {
                    discoverer.send_discovery(params[1],&self.host).await?;
                }
            } else if params[0].to_lowercase() == "list" {
                for disc in &self.discoverers {
                    println!("======================");
                    let devices = disc.discoveryed.lock().unwrap();
                    for dev in devices.iter() {
                        println!("\t{}",dev.share())
                    }
                }
            } else if params[0].to_lowercase() == "send" {
                let file = std::path::PathBuf::from(params[2]);
                debug!("file: {}",file.to_str().expect("cann't file path"));
                if file.exists() && file.is_file() {
                    debug!("device id {}", params[1]);
                    let addr = self.get_device_addr(params[1]).expect("get device ip failed");
                    self.send_files(addr, &vec![file]).await?;
                }
            }
        }
    }

    pub async fn sync_device_loop(&self,mut rx: tokio::sync::mpsc::Receiver<device::RemoteTcpDevice>) -> io::Result<()> {
        loop {
            debug!("wait for recv...");
            if let Some(device) = rx.recv().await {
                debug!("receive device {:#?}",device);
                let mut devices = self.devices.lock().unwrap();
                devices.push(device);
                self.ctx.request_repaint();
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn send_files(&self, addr: SocketAddr,files: &Vec<std::path::PathBuf>) -> io::Result<()> {
        debug!("send file {:?} to {}", files,addr);
        let mut conn = ClientConnector::connect(addr).await?;
        conn.send_public_key(&self.public_key).await?;
        conn.send_files(files).await?;
        Ok(())
    }

    /// get remote device tcp socket address
    fn get_device_addr(&self,id: &str) -> Option<SocketAddr> {
        for disc in &self.discoverers {
            if let Some(addr) = disc.get_remote_device_addr(id) {
                return Some(addr);
            }
        }
        return None;
    }
}

pub fn add() {

}

pub async fn send(key: &RsaPublicKey,addr: SocketAddr,files: &Vec<std::path::PathBuf>) -> io::Result<()> {
    let mut conn = ClientConnector::connect(addr).await?;
        conn.send_public_key(&key).await?;
        conn.send_files(files).await?;
        Ok(())
}

pub fn cancel() {

}