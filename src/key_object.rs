use serde::{Serialize, Deserialize};

#[derive(Clone,Debug, Serialize, Deserialize)]
pub struct KeyObject {
    pub r#type: String,
    pub data: String,
}