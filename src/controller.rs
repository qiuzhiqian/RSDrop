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
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    host: device::Device,
    devices: Arc<Mutex<Vec<RemoteTcpDevice>>>,
    ui_ctx: egui::Context,

    disc_txs: Vec<tokio::sync::mpsc::Sender<String>>,
    //disc_rxs: tokio::sync::mpsc::Receiver<String>, // for device notify

    //file_txs: tokio::sync::mpsc::Sender<String>, // for add_device
    //file_rxs: Vec<tokio::sync::mpsc::Receiver<String>>,
    tx: Option<tokio::sync::mpsc::Sender<String>>,
    rx: Option<tokio::sync::mpsc::Receiver<String>>,
}

impl Controller {
    pub fn new(ctx: egui::Context) -> Self {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
        let public_key = RsaPublicKey::from(&private_key);
        
        //let (tx1, rx1) = tokio::sync::mpsc::channel(10);
        //let (tx2, rx2) = tokio::sync::mpsc::channel(10);
        Self {
            private_key,
            public_key,
            host: Device::default(),
            devices: Arc::new(Mutex::new(Vec::<RemoteTcpDevice>::new())),
            ui_ctx: ctx,
            rx: None,
            tx: None,
            disc_txs: Vec::new(),
        }
    }

    pub fn gen_ctx(&mut self) -> (tokio::sync::mpsc::Sender<String>,tokio::sync::mpsc::Receiver<String>) {
        let (tx1, rx1) = tokio::sync::mpsc::channel(10);
        let (tx2, rx2) = tokio::sync::mpsc::channel(10);
        self.tx = Some(tx1);
        self.rx = Some(rx2);
        (tx2,rx1)
    }

    pub fn set_device_container(&mut self,devices: Arc<Mutex<Vec<RemoteTcpDevice>>>) {
        self.devices = devices;
    }

    pub async fn start_loop(&mut self) -> io::Result<()> {
        debug!("controller start...");
        let rx = self.start_discovery_service().await?;
        debug!("controller start 1...");
        //self.start_service().await?;
        debug!("controller start 2...");
        self.sync_device_loop(rx).await?;
        Ok(())
    }

    pub async fn start_discovery_service(&mut self) -> io::Result<tokio::sync::mpsc::Receiver<device::RemoteTcpDevice>> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        for interface in pnet::datalink::interfaces() {
            if interface.is_up() && !interface.ips.is_empty() && !interface.is_loopback() && !interface.name.contains("docker") {
                for ip in interface.ips {
                    if ip.is_ipv4() {
                        let discoverer = discoverer::Discovery::new(&ip.ip().to_string()).await?;
                        let add_tx = discoverer.start(&self.host,tx.clone()).await?;
                        self.disc_txs.push(add_tx);
                    }
                }
            }
        }
        Ok(rx)
    }

    pub async fn start_service(&mut self) -> io::Result<()> {
        for interface in pnet::datalink::interfaces() {
            if interface.is_up() && !interface.ips.is_empty() && !interface.is_loopback() && !interface.name.contains("docker") {
                for ip in interface.ips {
                    if ip.is_ipv4() {
                        let accepter = accepter::Accepter::new(&ip.ip().to_string()).await?;
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
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn sync_device_loop(&mut self,mut rx: tokio::sync::mpsc::Receiver<device::RemoteTcpDevice>) -> io::Result<()> {
        debug!("sync device loop");
        if let Some(rx1) = self.rx.as_mut() {

            loop {
                debug!("wait for recv...");
                tokio::select! {
                    device = rx.recv() => {
                        match device {
                            Some(d) => {
                                debug!("receive device {:#?}",d);
                                let mut devices = self.devices.lock().unwrap();
                                devices.push(d);
                                self.ui_ctx.request_repaint();
                            },
                            None => break,
                        }
                    }
                    recv_ip = rx1.recv() => {
                        match recv_ip {
                            Some(ip) => {
                                debug!("add ip {}",ip);
                                for tx in &self.disc_txs {
                                    tx.send(ip.clone()).await.expect("send failed");
                                }
                            },
                            None => {
                                break;
                            }
                        }
                    }
                };
            }
        }
        
        Ok(())
    }

    pub async fn send_files(&self, addr: SocketAddr,files: &Vec<std::path::PathBuf>) -> io::Result<()> {
        debug!("send file {:?} to {}", files,addr);
        let mut conn = ClientConnector::connect(addr).await?;
        conn.send_public_key(&self.public_key).await?;
        conn.send_files(files).await?;
        Ok(())
    }

    /// get remote device tcp socket address
    fn get_device_addr(&self,id: &str) -> Option<SocketAddr> {
        let devices = self.devices.lock().unwrap();
        for disc in devices.iter() {
            if &disc.device.id == id {
                return Some(disc.addr.clone())
            }
        }
        return None;
    }
}

//pub async fn add_device(ip: &str) -> io::Result<()> {
//    for tx in &self.disc_txs {
//        if let Err(e) = tx.send(ip.to_string()).await {
//            debug!("receive drop: {}",e);
//            break;
//        }
//    }
//    Ok(())
//}
