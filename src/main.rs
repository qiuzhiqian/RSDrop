use serde::{Serialize, Deserialize};
use log::{debug, error, log_enabled, info, Level};

use tokio::net::{UdpSocket,TcpListener,TcpStream};
use tokio::io::{self, AsyncRead, AsyncWrite, AsyncReadExt,AsyncWriteExt};
use std::io::Write;
use std::process;
use std::io::Read;

use std::net::{Ipv4Addr, SocketAddr};

use std::sync::{Arc,Mutex};

use std::time::{SystemTime,UNIX_EPOCH};

use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs8::{EncodePublicKey,DecodePublicKey};

use md5::{Md5, Digest};
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
            id: format!("{}-{}",since_the_epoch.as_micros(),id),
        }
    }

    fn share(&self) -> String {
        format!("drop://{}/{}#{}",self.id,self.r#type,self.name)
    }
}

#[derive(Clone,Debug)]
struct RemoteTcpDevice {
    addr: SocketAddr,
    device: Device,
}

impl RemoteTcpDevice {
    pub fn new(ip: &str,port: u16,dev: Device) -> Self {
        Self{
            addr: SocketAddr::new(ip.parse().expect("ip is invalid"), port),
            device: dev,
        }
    }

    fn share(&self) -> String {
        self.device.share()
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
    env_logger::init();
    let mut controller = Controller::new();
    controller.start_discovery_service().await?;
    controller.cmd_loop().await?;
    Ok(())
}

#[derive(Debug)]
struct Server {
    tcp_ip: String,
    udp_socket: Arc<UdpSocket>,
    host: Device,
    discoveryed: Arc<Mutex<Vec<RemoteTcpDevice>>>,
}

impl Server {
    async fn new(ip: &str) -> io::Result<Self>{
        let socket = UdpSocket::bind(format!("{}:{}",ip ,UDP_PORT)).await?;
        let inter = Ipv4Addr::new(0,0,0,0);
        socket.join_multicast_v4(MULTICAST_IP,inter).expect("join failed");
        socket.set_multicast_ttl_v4(50)?;
        socket.set_multicast_loop_v4(false)?;

        Ok(Server{
            tcp_ip: String::from(ip),
            udp_socket: Arc::new(socket),
            host: Device::default(),
            discoveryed: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn start(&self,self_key: &RsaPublicKey) -> io::Result<()> {
        self.discovery_listen().await?;
        self.send_discovery(&MULTICAST_IP.to_string()).await.unwrap();
        self.tcp_server_listen(&self_key).await.expect("has error");
        Ok(())
    }

    async fn send_discovery(&self,addr:&str) -> std::io::Result<()> {
        let discovery_req = DiscoveryReq::new(&self.host, true);
        let data = serde_json::to_string(&discovery_req)?;
        debug!("send discovery request to {}:{}",addr,UDP_PORT);
        self.udp_socket.send_to(data.as_bytes(), format!("{}:{}",addr,UDP_PORT)).await?;
        debug!("send end...");
        Ok(())
    }

    async fn discovery_listen(&self) -> io::Result<()> {
        let send_socket = self.udp_socket.clone();
        let host_device = self.host.clone();
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

                        devices.push(RemoteTcpDevice { addr, device: discovery.device });
                    }
                    // for ack
                    if discovery.ack {
                        let discovery_resp = DiscoveryReq::new(&host_device, false);
                        let data = serde_json::to_string(&discovery_resp).unwrap();
                        send_socket.send_to(data.as_bytes(), format!("{}:{}",addr.ip(),UDP_PORT)).await.unwrap();
                    }
                }
                debug!("one device has discoveryed.");
            }
            
        });
        Ok(())
    }

    async fn tcp_server_listen(&self,self_key: &RsaPublicKey) -> io::Result<()> {
        let accepter = ServerAccepter::new(&self.tcp_ip, TCP_PORT).await?;
        let key = self_key.clone();
        tokio::spawn(async move{
            loop {
                info!("start tcp server for receive file");
                let (mut stream,addr) = accepter.accept(&key).await.expect("has error");
                info!("accept addr {}",addr);
                tokio::spawn(async move {
                    ServerAccepter::recv_files(&mut stream).await.expect("receive failed");
                });
            }
        });
        
        Ok(())
    }

    fn get_remote_device_addr(&self,id: &str) -> Option<SocketAddr> {
        let mut devs = self.discoveryed.lock().unwrap();
        for dev in devs.iter_mut() {
            if dev.device.id == id {
                return Some(dev.addr);
            }
        }
        return None;
    }
}

#[derive(Debug)]
struct ServerAccepter {
    tcp_listener: TcpListener,
}

#[derive(Clone,Debug, Serialize, Deserialize)]
struct KeyObject {
    r#type: String,
    data: String,
}


#[derive(Clone,Debug, Serialize, Deserialize)]
struct FileMeta {
    name: String,
    size: u64,
    verity: FileVerity,
}

impl FileMeta {
    pub fn new(file: &std::path::PathBuf) -> io::Result<Self> {
        let meta = file.metadata()?;

        let md5_val = file_md5(file)?;
        Ok(Self{
            name: file.file_name().expect("is not file").to_str().expect("is not file").to_string(),
            size: meta.len(),
            verity: FileVerity{
                r#type: "rsa".to_string(),
                data: md5_val,
            },
        })
    }
}

#[derive(Clone,Debug, Serialize, Deserialize)]
struct FileVerity {
    r#type: String,
    data: String,
}

#[derive(Clone,Debug, Serialize, Deserialize)]
struct MetaList {
    files: Vec<FileMeta>,
}

impl ServerAccepter{
    async fn new(ip:&str,port:u16) -> io::Result<Self> {
        let tcp_listener = TcpListener::bind(format!("{}:{}",ip,port)).await?;

        Ok(Self{tcp_listener})
    }

    async fn accept(&self,self_key:& RsaPublicKey) -> io::Result<(TcpStream,std::net::SocketAddr)> {
        let (mut stream, addr) = self.tcp_listener.accept().await?;

        let mut buf = Vec::<u8>::new();
        let public_key = loop {
            let mut data = Vec::<u8>::new();
            let lens = stream.read_buf(&mut data).await?;
            if lens == 0 {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted,"connect is closed"));
            }
            buf.append(&mut data);
            if let Ok(keyobject) = serde_json::from_slice::<KeyObject>(&buf){
                buf.clear();
                if keyobject.r#type != "rsa" {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "public key type must rsa"));
                }

                let key_data = RsaPublicKey::from_public_key_pem(&keyobject.data).expect("public key is invalid");
                break key_data;
            }
        };

        debug!("send public key");
        self.send_public_key(self_key,&mut stream).await?;

        Ok((stream,addr))
    }

    async fn send_public_key<T: AsyncWrite + Unpin + Send>(&self, self_key: &RsaPublicKey, tx: &mut T) -> io::Result<()> {
        let key_response = KeyObject {
            r#type: "rsa".to_string(),
            data: self_key.to_public_key_pem(base64ct::LineEnding::LF).expect("can't covert to pem"),
        };
        let response_data = serde_json::to_vec(&key_response).expect("cannot find key");
        tx.write_all(&response_data).await?;
        Ok(())
    }

    async fn recv_files<T: AsyncWrite + AsyncRead + Unpin + Send>(stream: &mut T) -> io::Result<()> {
        debug!("wait recv files meta");
        let mut buf = Vec::<u8>::new();
        //receive meta
        let meta_list = loop {
            let mut data = Vec::<u8>::new();
            let lens = stream.read_buf(&mut data).await?;
            if lens == 0 {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted,"connect is close"));
            }
            buf.append(&mut data);
            if let Ok(meta_list) = serde_json::from_slice::<MetaList>(&buf) {
                buf.clear();
                break meta_list;
            }
        };
        debug!("recv files meta success");

        // wait for ack
        stream.write_u8(0x01).await?;
        
        for meta in meta_list.files {
            // recv files
            let temp_name = meta.name.clone() + ".tmp";
            let mut file = std::fs::File::create(&temp_name).expect("create failed");
            let mut need_size = meta.size;
            while need_size > 0 {
                let lens = if need_size >= 10240 {
                    let mut data = [0;10240];
                    let lens = stream.read_exact(&mut data).await.expect("read failed");
                    if lens == 0{
                        break;
                    }
                    file.write_all(&data).expect("write failed");
                    lens
                } else {
                    let mut data = Vec::<u8>::new();
                    let lens = stream.read_to_end(&mut data).await.expect("read failed");
                    if lens == 0 {
                        break;
                    }
                    file.write_all(&data).expect("write failed");
                    lens
                };
                
                need_size = need_size - lens as u64;
                debug!("Percent: {}%",(meta.size - need_size) * 100 / meta.size)
            }
            file.flush().expect("write failed");

            // file check
            let path = std::path::PathBuf::from(&meta.name);
            let md5 = file_md5(&path)?;
            if md5 == meta.verity.data {
                std::fs::rename(&temp_name, &meta.name).expect("rename failed");
                debug!("recv {} success", meta.name);
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "file check failed"));
            }
        }
        Ok(())
    }

}

struct ClientConnector {
    tcp_connector: TcpStream,
}

impl ClientConnector {
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let tcp_connector = TcpStream::connect(addr).await?;
        tcp_connector.set_nodelay(true)?;
        Ok(Self { tcp_connector, })
    }

    pub async fn send_public_key(&mut self, key: &RsaPublicKey) -> io::Result<()> {
        let key_response = KeyObject {
            r#type: "rsa".to_string(),
            data: key.to_public_key_pem(base64ct::LineEnding::LF).expect("can't covert to pem"),
        };
        let response_data = serde_json::to_vec(&key_response).expect("cannot find key");
        self.tcp_connector.write_all(&response_data).await?;

        let mut buf = Vec::<u8>::new();
        //let key = loop {
        //    let mut data = Vec::<u8>::new();
        //    self.tcp_connector.read_buf(&mut data).await?;
        //}
        let public_key = loop {
            let mut data = Vec::<u8>::new();
            let lens = self.tcp_connector.read_buf(&mut data).await?;
            if lens == 0 {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted,"connect is close"));
            }
            buf.append(&mut data);
            if let Ok(keyobject) = serde_json::from_slice::<KeyObject>(&buf){
                buf.clear();
                if keyobject.r#type != "rsa" {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "public key type must rsa"));
                }

                let key_data = RsaPublicKey::from_public_key_pem(&keyobject.data).expect("public key is invalid");
                break key_data;
            }
        };
        Ok(())
    }

    pub async fn send_files(&mut self, files: &Vec<std::path::PathBuf>) -> io::Result<()> {
        let mut file_meta_list = MetaList{
            files: Vec::new(),
        };
        for file in files {
            file_meta_list.files.push(FileMeta::new(file)?);
        }

        let request = serde_json::to_string(&file_meta_list)?;
        self.tcp_connector.write_all(request.as_bytes()).await?;
        let resp = self.tcp_connector.read_u8().await?;
        if resp != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData,"response is error"));
        }

        for file in files {
            let mut sum = 0;
            let mut f = std::fs::File::open(file)?;
            let mut buf = [0;10240];
            loop {
                let lens = f.read(&mut buf)?;
                if lens == 0{
                    break;
                }
                sum = sum + lens;

                self.tcp_connector.write_all(&buf[..lens]).await?;
            }
        }
        debug!("file send succeed!");
        Ok(())
    }
}

fn hostname() -> String {
    // Linux
    let data = std::fs::read_to_string("/etc/hostname").unwrap();
    let hostname = data.trim_end().to_string();
    if let Ok(user) = std::env::var("USER") {
        return format!("{}-{}",user.to_ascii_uppercase(),hostname.to_ascii_uppercase());
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

fn file_md5(file: &std::path::PathBuf) -> io::Result<String> {
    let mut hasher = Md5::new();
    let mut f = std::fs::File::open(&file)?;
    let mut buf = [0;10240];
    loop {
        let lens = f.read(&mut buf)?;
        if lens == 0{
            break;
        }
        hasher.update(&buf[..lens]);
    }
    let md5_str = hasher.finalize().to_vec().iter()
        .map(|x| format!("{:02x}", x))
        .collect::<String>();
    Ok(md5_str)
}

struct Controller {
    servers: Vec<Server>,
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

impl Controller {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
        let public_key = RsaPublicKey::from(&private_key);
        Self {
            private_key,
            public_key,
            servers: Vec::new(),
        }
    }
    async fn start_discovery_service(&mut self) -> io::Result<()> {
        for interface in pnet::datalink::interfaces() {
            if interface.is_up() && !interface.ips.is_empty() && !interface.is_loopback() && !interface.name.contains("docker") {
                for ip in interface.ips {
                    if ip.is_ipv4() {
                        let server = Server::new(&ip.ip().to_string()).await?;
                        server.start(&self.public_key).await?;
                        self.servers.push(server);
                    }
                }
            }
        }
        Ok(())
    }

    async fn cmd_loop(&self) -> io::Result<()> {
        info!("run cmd loop");
        loop{
            let mut buffer = String::new();
            let stdin = std::io::stdin();
            stdin.read_line(&mut buffer)?;
            let params: Vec<&str> = buffer.split_ascii_whitespace().collect();
            debug!("param: {:#?}",params);
            if params.is_empty() {
                continue;
            }
    
            if params[0].to_lowercase() == "add" {
                debug!("add {}",params[1]);
    
                for server in &self.servers {
                    server.send_discovery(params[1]).await?;
                }
            } else if params[0].to_lowercase() == "dump" {
                debug!("servers: {:#?}",&self.servers);
            } else if params[0].to_lowercase() == "send" {
                let file = std::path::PathBuf::from(params[2]);
                debug!("file: {}",file.to_str().expect("cann't file path"));
                if file.exists() && file.is_file() {
                    let addr = self.get_device_addr(params[1]).expect("get device ip failed");
                    self.send_files(addr, &vec![file]).await?;
                }
            }
        }
    }

    async fn send_files(&self, addr: SocketAddr,files: &Vec<std::path::PathBuf>) -> io::Result<()> {
        debug!("send file {:?} to {}", files,addr);
        let mut conn = ClientConnector::new(addr).await?;
        conn.send_public_key(&self.public_key).await?;
        conn.send_files(files).await?;
        Ok(())
    }

    fn get_device_addr(&self,id: &str) -> Option<SocketAddr> {
        for server in &self.servers {
            if let Some(r) = server.get_remote_device_addr(id) {
                return Some(SocketAddr::new(r.ip(),TCP_PORT));
                //return Some(r);
            }
        }
        return None;
    }
}