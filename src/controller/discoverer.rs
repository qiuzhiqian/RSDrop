use tokio::net::UdpSocket;
use tokio::io;
use serde::{Serialize, Deserialize};
use std::sync::{Arc,Mutex};
use tokio::io::{ AsyncReadExt, AsyncWriteExt, AsyncRead, AsyncWrite};
use log::{debug, info};
use std::net::{SocketAddr, Ipv4Addr};

use crate::device::{Device,RemoteTcpDevice};
use super::accepter;

const VERSION: u32 = 1u32;
const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
const UDP_PORT:u16 = 52637u16;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryReq {
    pub version: u32,
    pub device: Device,
    pub port: u16,
    pub ack: bool,
}
impl DiscoveryReq {
    fn new(device: &Device, port: u16, ack: bool) -> Self {
        DiscoveryReq {
            version: VERSION,
            device: device.clone(),
            port: port,
            ack,
        }
    }
}

/// discovery other devices by udp multicast 
#[derive(Debug)]
pub struct Discovery {
    socket : Arc<UdpSocket>,
}

impl Discovery {
    pub async fn new(ip: &str)  -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("{}:{}",ip ,UDP_PORT)).await?;
        let inter = Ipv4Addr::new(0,0,0,0);
        socket.join_multicast_v4(MULTICAST_IP,inter).expect("join failed");
        socket.set_multicast_ttl_v4(50)?;
        socket.set_multicast_loop_v4(false)?;
        Ok(Self{socket: Arc::new(socket),})
    }

    pub async fn start(&self, dev: &Device,tx: tokio::sync::mpsc::Sender<crate::device::RemoteTcpDevice>) -> io::Result<tokio::sync::mpsc::Sender<String>> {
        send_discovery(&self.socket,&MULTICAST_IP.to_string(), &dev).await?;
        
        let (add_tx,rx) = tokio::sync::mpsc::channel(20);
        // recv
        // add
        let recv_socket = self.socket.clone();
        let service_socket = self.socket.clone();
        let recv_device = dev.clone();
        let service_dev = dev.clone();
        tokio::spawn(async move {
            service(service_socket,&service_dev,tx).await.expect("abc");
        });
        tokio::spawn(async move {
            receive_handle(recv_socket,&recv_device,rx).await.expect("abc");
        });

        Ok(add_tx)
    }
}

async fn service(socket: Arc<tokio::net::UdpSocket>, dev: &Device, tx: tokio::sync::mpsc::Sender<crate::device::RemoteTcpDevice>) -> io::Result<()> {
    let mut buf = Vec::<u8>::new();
    let local_addr = socket.local_addr().unwrap();
    info!("local addr: {}",local_addr);
    let host_device = dev.clone();
    loop {
        let mut data = [0; 1024];
        let (lens, addr) = socket.recv_from(&mut data).await.expect("disconnect.");
        buf.append(&mut data[..lens].to_vec());
        if let Ok(discovery) = serde_json::from_slice::<DiscoveryReq>(&buf){
            buf.clear();
            if host_device.id == discovery.device.id.clone() {
                continue;
            }

            debug!("send for notify");
            let remote_device = RemoteTcpDevice { addr: SocketAddr::new(addr.ip(), discovery.port), device: discovery.device };
            tx.send(remote_device).await.expect("send failed");
            
            // for ack
            if discovery.ack {
                let discovery_resp = DiscoveryReq::new(&host_device, accepter::TCP_ACCEPTER_PORT , false);
                let data = serde_json::to_string(&discovery_resp).unwrap();
                socket.send_to(data.as_bytes(), format!("{}:{}",addr.ip(),UDP_PORT)).await.unwrap();
            }
        }
        debug!("one device has discoveryed.");
    }
}

async fn receive_handle(socket: Arc<tokio::net::UdpSocket>, dev: &Device,mut rx: tokio::sync::mpsc::Receiver<String>) -> io::Result<()> {
    let host_device = dev.clone();
    loop{
        if let Some(ip) = rx.recv().await {
            debug!("do add device: {}", ip);
            //do add device
            send_discovery(&socket,&ip,&host_device).await?;
        }
    }
}

async fn send_discovery(socket: &Arc<tokio::net::UdpSocket>,addr:&str,dev: &Device) -> std::io::Result<()> {
    let discovery_req = DiscoveryReq::new(dev, accepter::TCP_ACCEPTER_PORT, true);
    let data = serde_json::to_string(&discovery_req)?;
    debug!("send discovery request to {}:{}",addr,UDP_PORT);
    socket.send_to(data.as_bytes(), format!("{}:{}",addr,UDP_PORT)).await?;
    //stream.write(src)
    debug!("send end...");
    Ok(())
}

