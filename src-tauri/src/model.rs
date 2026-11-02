use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attr {
    pub raw: String,
    pub num: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Detail {
    pub attrs: BTreeMap<String, Attr>,
    pub equipment: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Role {
    pub code: String,
    pub name: String,
    pub title: String,
    pub job: String,
    pub price: Option<f64>,
    pub server_id: String,
    pub status: u64,
    pub listed_at: String,
    pub offline_at: String,
    #[serde(default)]
    pub detail: Option<Detail>,
    #[serde(default)]
    pub detail_at: Option<u64>,
    #[serde(default)]
    pub checked_at: u64,
    #[serde(default)]
    pub detail_error: Option<String>,
    #[serde(default)]
    pub needs_refresh: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Server {
    #[serde(rename = "serverId")]
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    pub server: Vec<Server>,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.bytes().all(|b| b.is_ascii_digit())
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
