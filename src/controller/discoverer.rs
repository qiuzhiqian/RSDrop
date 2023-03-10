use pnet::packet::ip::IpNextHeaderProtocols::Mux;
use tokio::net::{UdpSocket,TcpListener,TcpStream};
use tokio::io;
use serde::{Serialize, Deserialize};
use std::sync::{Arc,Mutex};
use log::{debug, error, log_enabled, info, Level};
use std::net::{IpAddr, SocketAddr, Ipv4Addr};

use crate::device::{Device,RemoteTcpDevice};
use crate::file_meta::FileMeta;
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
#[derive(Debug, Clone)]
pub struct Discovery {
    pub socket : Arc<UdpSocket>,
    pub discoveryed: Arc<Mutex<Vec<RemoteTcpDevice>>>,
}

impl Discovery {
    pub async fn new(ip: &str)  -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("{}:{}",ip ,UDP_PORT)).await?;
        let inter = Ipv4Addr::new(0,0,0,0);
        socket.join_multicast_v4(MULTICAST_IP,inter).expect("join failed");
        socket.set_multicast_ttl_v4(50)?;
        socket.set_multicast_loop_v4(false)?;
        Ok(Self{socket: Arc::new(socket),discoveryed: Arc::new(Mutex::new(Vec::<RemoteTcpDevice>::new()))})
    }

    pub async fn send_discovery(&self,addr:&str,dev: &Device) -> std::io::Result<()> {
        let discovery_req = DiscoveryReq::new(dev, accepter::TCP_ACCEPTER_PORT, true);
        let data = serde_json::to_string(&discovery_req)?;
        debug!("send discovery request to {}:{}",addr,UDP_PORT);
        self.socket.send_to(data.as_bytes(), format!("{}:{}",addr,UDP_PORT)).await?;
        debug!("send end...");
        Ok(())
    }

    pub async fn start(&self,dev: &Device) -> io::Result<()> {
        let send_socket = self.socket.clone();
        let host_device = dev.clone();
        let child_devices = self.discoveryed.clone();
        tokio::spawn(async move {
            let mut buf = Vec::<u8>::new();
            let local_addr = send_socket.local_addr().unwrap();
            info!("local addr: {}",local_addr);
            loop {
                let mut data = [0; 1024];
                let (lens, addr) = send_socket.recv_from(&mut data).await.expect("disconnect.");
                buf.append(&mut data[..lens].to_vec());
                if let Ok(discovery) = serde_json::from_slice::<DiscoveryReq>(&buf){
                    buf.clear();
                    if host_device.id == discovery.device.id {
                        continue;
                    }
                    
                    {
                        let mut devices = child_devices.lock().unwrap();

                        devices.push(RemoteTcpDevice { addr: SocketAddr::new(addr.ip(), discovery.port), device: discovery.device });
                    }
                    // for ack
                    if discovery.ack {
                        let discovery_resp = DiscoveryReq::new(&host_device, accepter::TCP_ACCEPTER_PORT , false);
                        let data = serde_json::to_string(&discovery_resp).unwrap();
                        send_socket.send_to(data.as_bytes(), format!("{}:{}",addr.ip(),UDP_PORT)).await.unwrap();
                    }
                }
                debug!("one device has discoveryed.");
            }
            
        });

        self.send_discovery(&MULTICAST_IP.to_string(), &dev).await
    }

    pub fn get_remote_device_addr(&self,id: &str) -> Option<SocketAddr> {
        let mut devs = self.discoveryed.lock().unwrap();
        debug!("id {}",id);
        for dev in devs.iter_mut() {
            if dev.device.id == id {
                return Some(dev.addr);
            }
        }
        return None;
    }
}