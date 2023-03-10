use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt,AsyncWriteExt};

use log::debug;

use rsa::RsaPublicKey;
use rsa::pkcs8::{EncodePublicKey,DecodePublicKey};
use crate::key_object::KeyObject;
use crate::file_meta::{FileMeta,MetaList};
use std::io::Read;

const TCP_CONNECTOR_PORT: u16 = 52638u16;

pub struct ClientConnector {
    pub tcp_connector: TcpStream,
}

impl ClientConnector {
    pub async fn connect<A: tokio::net::ToSocketAddrs>(addr: A) -> io::Result<Self> {
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