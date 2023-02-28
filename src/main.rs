//use std::net::UdpSocket;
use serde::{Serialize, Deserialize};
//use std::thread;

use tokio::net::UdpSocket;
use tokio::io::{AsyncWrite,AsyncRead};
use std::io;
use std::process;

use std::net::Ipv4Addr;

use std::sync::{Arc,Mutex};

use std::time::{SystemTime,UNIX_EPOCH};

#[derive(Clone,Debug, Serialize, Deserialize)]
struct Device {
    name: String,
    r#type: String,
    id: String,
}

impl Device {
    fn default() -> Self {
        let id = process::id();
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

        Device{
            name: hostname(),
            r#type: device_type(),
            id: format!("simp_drop://{}/{}",since_the_epoch.as_micros(),id),
        }
    }
}

const VERSION: u32 = 1u32;
const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 123);
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
                    server.send_discovery(&MULTICAST_IP.to_string()).await.unwrap();
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
        } else if params[0].to_lowercase() == "send" {
            for server in &servers {
                server.send_file(params[1]).await?;
            }
        }
    }
}

#[derive(Debug,Clone)]
struct Server {
    udp_socket: Arc<UdpSocket>,
    host: Device,
    discoveryed: Arc<Mutex<Vec<Device>>>,
}

impl Server {
    async fn new(ip: &str) -> io::Result<Self>{
        let socket = UdpSocket::bind(format!("{}:{}",ip ,UDP_PORT)).await?;
        let inter = Ipv4Addr::new(0,0,0,0);
        socket.join_multicast_v4(MULTICAST_IP,inter).expect("join failed");
        socket.set_multicast_ttl_v4(50)?;

        Ok(Server{
            udp_socket: Arc::new(socket),
            host: Device::default(),
            discoveryed: Arc::new(Mutex::new(Vec::new())),
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
        let child_devices = self.discoveryed.clone();
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
                    if host_device.id == discovery.device.id {
                        continue;
                    }
                    println!("discovery {:#?}",discovery);
                    {
                        let mut devices = child_devices.lock().unwrap();
                        devices.push(discovery.device);
                    }
                    // for ack
                    if discovery.ack {
                        let discovery_resp = DiscoveryReq::new(&host_device, false);
                        let data = serde_json::to_string(&discovery_resp).unwrap();
                        send_socket.send_to(data.as_bytes(), format!("{}:{}",addr.ip(),UDP_PORT)).await.unwrap();
                        buf.clear();
                    }
                } else {
                    println!("data: {}",String::from_utf8(data[..lens].to_vec()).expect("failed"));
                    buf.append(&mut data[..lens].to_vec())
                }
                println!("handle end.");
            }
            
        });
        Ok(())
    }

    async fn send_file(&self,id: &str,file: std::path::PathBuf) -> io::Result<()> {
        let dev = self.discoveryed.lock().unwrap();
        for device in  dev.iter() {
            if device.id == id {
                println!("find device {:#?}",device);
            }
        }
        Ok(())
    }
}

fn hostname() -> String {
    // Linux
    let data = std::fs::read_to_string("/etc/hostname").unwrap();
    let hostname = data.trim_end().to_string();
    if let Ok(user) = std::env::var("USER") {
        return format!("{}@{}",user,hostname);
    }
    hostname
}

fn device_type() -> String {
    if cfg!(target_os = "windows") {
        return "windows".to_string();
    } else if cfg!(target_os = "linux") {
        return "linux".to_string();
    } else if cfg!(target_os = "macos") {
        return "macos".to_string();
    } else if cfg!(target_os = "ios") {
        return "ios".to_string();
    } else if cfg!(target_os = "android") {
        return "android".to_string();
    } else {
        return "unknow".to_string();
    }
}
