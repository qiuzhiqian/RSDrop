use tokio::net::{UdpSocket,TcpListener,TcpStream};
use tokio::io::{self, AsyncRead, AsyncWrite, AsyncReadExt,AsyncWriteExt};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs8::{EncodePublicKey,DecodePublicKey};
use log::{debug, error, log_enabled, info, Level};
use crate::key_object::KeyObject;
use crate::file_meta::{MetaList,FileMeta,file_md5};
use std::io::Write;

pub const TCP_ACCEPTER_PORT: u16 = 52638u16;

#[derive(Debug)]
pub struct Accepter {
    tcp_listener: TcpListener,
}

impl Accepter{
    pub async fn new(ip:&str) -> io::Result<Self> {
        let tcp_listener = TcpListener::bind(format!("{}:{}",ip,TCP_ACCEPTER_PORT)).await?;

        Ok(Self{tcp_listener})
    }

    pub async fn accept(&self,self_key:& RsaPublicKey) -> io::Result<(TcpStream,std::net::SocketAddr)> {
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

    pub async fn recv_files<T: AsyncWrite + AsyncRead + Unpin + Send>(stream: &mut T) -> io::Result<()> {
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
            let temp_name = meta.name.clone() + ".droptmp";
            let temp_name_path = std::path::PathBuf::from(&temp_name);
            if temp_name_path.exists() {
                if temp_name_path.is_dir() {
                    std::fs::remove_dir_all(&temp_name_path)?;
                } else {
                    std::fs::remove_file(&temp_name_path)?;
                }
            }
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
            let path = std::path::PathBuf::from(&temp_name);
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

