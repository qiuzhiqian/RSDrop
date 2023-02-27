//use std::net::UdpSocket;
use serde::{Serialize, Deserialize};
//use std::thread;

use tokio::net::UdpSocket;
use tokio::io::{AsyncWrite,AsyncRead};
use std::io;
use std::sync::Arc;
use std::process;

use std::net::Ipv4Addr;

#[derive(Clone,Debug, Serialize, Deserialize)]
struct Device {
    name: String,
    r#type: String,
    id: String,
}

impl Device {
    fn default() -> Self {
        let id = process::id();
        Device{
            name: "xml-pc".to_string(),
            r#type: "Linux".to_string(),
            id: format!("sdfasdfjlsga-{}",id),
        }
    }
}

const VERSION: u32 = 1u32;
const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(234, 2, 2, 2);
const UDP_PORT:u16 = 52637u16;
const TCP_PORT: u16 = 52638u16;

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryReq {
    version: u32,
    device: Device,
    port: u16,
    ack: bool,
}
impl DiscoveryReq {
    fn new(device: &Device,ack: bool) -> Self {
        DiscoveryReq {
            version: VERSION,
            device: device.clone(),
            port: TCP_PORT,
            ack,
        }
    }
}
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut servers = Vec::<Server>::new();
    for interface in pnet::datalink::interfaces() {
        if interface.is_up() && !interface.ips.is_empty() && !interface.is_loopback() && !interface.name.contains("docker") {
            println!("{}", interface);
            for ip in interface.ips {
                if ip.is_ipv4() {
                    println!("ip:{} boardcast: {}", ip.ip().to_string(),ip.broadcast().to_string());
                    let server = Server::new(&ip.ip().to_string()).await.unwrap();
                    server.listen_device().await?;
                    server.send_discovery("234.2.2.2").await.unwrap();
                    servers.push(server);
                }
            }
        }
    }
    
    loop{
        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer)?;
        let params: Vec<&str> = buffer.split_ascii_whitespace().collect();
        println!("param: {:#?}",params);
        if params.is_empty() {
            continue;
        }

        if params[0].to_lowercase() == "add" {
            println!("add {}",params[1]);

            for server in &servers {
                server.send_discovery(params[1]).await?;
            }
        } else if params[0].to_lowercase() == "dump" {
            println!("servers: {:#?}",&servers);
        }
    }
}

#[derive(Debug,Clone)]
struct Server {
    udp_socket: Arc<UdpSocket>,
    host: Device,
    discoveryed: Vec<Device>,
}

impl Server {
    async fn new(ip: &str) -> io::Result<Self>{
        let socket = UdpSocket::bind(format!("{}:{}",ip ,UDP_PORT)).await?;
        let inter = Ipv4Addr::new(0,0,0,0);
        socket.join_multicast_v4(MULTICAST_IP,inter).expect("join failed");

        Ok(Server{
            udp_socket: Arc::new(socket),
            host: Device::default(),
            discoveryed: Vec::new(),
        })
    }
    async fn send_discovery(&self,addr:&str) -> std::io::Result<()>{
        let discovery_req = DiscoveryReq::new(&self.host, true);
        let data = serde_json::to_string(&discovery_req)?;
        println!("data {}",data);
        println!("send data to {}:{}",addr,UDP_PORT);
        self.udp_socket.send_to(data.as_bytes(), format!("{}:{}",addr,UDP_PORT)).await?;
        Ok(())
    }

    async fn listen_device(&self) -> io::Result<()>{
        let send_socket = self.udp_socket.clone();
        let host_device = self.host.clone();
        tokio::spawn(async move {
            let mut buf = Vec::<u8>::new();
            let local_addr = send_socket.local_addr().unwrap();
            println!("local: {}",local_addr);
            loop {
                //let mut data = Vec::<u8>::new();
                let mut data = [0; 1024];
                let (lens, addr) = send_socket.recv_from(&mut data).await.expect("disconnect.");
                println!("lens {},addr: {}",lens,addr);
                if let Ok(discovery) = serde_json::from_slice::<DiscoveryReq>(&data[..lens]){
                    println!("discovery {:#?}",discovery);
                    if discovery.ack && host_device.id != discovery.device.id {
                        let discovery_resp = DiscoveryReq::new(&host_device, false);
                        let data = serde_json::to_string(&discovery_resp).unwrap();
                        send_socket.send_to(data.as_bytes(), format!("{}:{}",addr.ip(),UDP_PORT)).await.unwrap();
                        buf.clear();
                    }
                } else {
                    buf.append(&mut data[..lens].to_vec())
                }
                println!("handle end.");
            }
            
        });
        Ok(())
    }
}
