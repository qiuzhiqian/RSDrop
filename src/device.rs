use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use std::time::{SystemTime,UNIX_EPOCH};

use crate::utils;

#[derive(Clone,Debug, Serialize, Deserialize)]
pub struct Device {
    pub name: String,
    pub r#type: String,
    pub id: String,
}

impl Device {
    pub fn default() -> Self {
        let id = std::process::id();
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        Device{
            name: utils::hostname(),
            r#type: Self::host_device_type(),
            id: format!("{}-{}",since_the_epoch.as_micros(),id),
        }
    }

    fn host_device_type() -> String {
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

    fn device_type(&self) -> &str {
        &self.r#type
    }

    fn share(&self) -> String {
        format!("drop://{}/{}#{}",self.id,self.r#type,self.name)
    }
}

#[derive(Clone,Debug)]
pub struct RemoteTcpDevice {
    pub addr: SocketAddr,
    pub device: Device,
}

impl RemoteTcpDevice {
    pub fn new(ip: &str,port: u16,dev: Device) -> Self {
        Self{
            addr: ip.parse().expect("ip is invalid"),
            device: dev,
        }
    }

    fn device_type(&self) -> &str {
        &self.device.device_type()
    }

    pub fn share(&self) -> String {
        self.device.share()
    }
}
