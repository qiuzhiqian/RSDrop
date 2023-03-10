use serde::{Serialize, Deserialize};
use md5::{Md5, Digest};
use std::io::{self,Read};


#[derive(Clone,Debug, Serialize, Deserialize)]
pub struct FileMeta {
    pub name: String,
    pub size: u64,
    pub verity: FileVerity,
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
pub struct FileVerity {
    pub r#type: String,
    pub data: String,
}

#[derive(Clone,Debug, Serialize, Deserialize)]
pub struct MetaList {
    pub files: Vec<FileMeta>,
}

pub fn file_md5(file: &std::path::PathBuf) -> io::Result<String> {
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