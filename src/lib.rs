use std::{path::PathBuf, sync::Arc};

pub mod api;
pub mod links;

use concurrent_queue::ConcurrentQueue;
use links::Links;
use minijinja::context;
use tokio::sync::RwLock;


#[derive(Debug)]
pub struct LinkAccessEvent {
    pub key: String,
    pub timestamp: std::time::SystemTime,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub links: Arc<RwLock<Links>>,
    pub access_event_queue: Arc<ConcurrentQueue<LinkAccessEvent>>
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(Config::from_env()),
            links: Arc::new(RwLock::new(Links::default())),
            access_event_queue: Arc::new(ConcurrentQueue::unbounded())
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub link_data_path: PathBuf,
    pub bind_address: String,
    pub server_base_url: String,
    pub key_blacklist: Vec<String>
}

impl Config {
    pub fn from_env() -> Self {     
        let link_data_path = std::env::var("LANDMOWER_LINK_DATA_PATH")
            .map(|s| s.into())
            .unwrap_or_else(|_| default_link_data_path());

        let bind_address = std::env::var("LANDMOWER_BIND_ADDRESS")
            .unwrap_or_else(|_| "localhost:7171".to_string());

        let server_base_url = std::env::var("LANDMOWER_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:7171/".to_string());

        let key_blacklist: Vec<_> = std::env::var("LANDMOWER_KEY_BLACKLIST")
            .unwrap_or_else(|_| "".to_string())
            .split(" ")
            .filter_map(|s| if s.is_empty() { None } else { Some(s.trim().to_string()) })
            .collect();

        Self { link_data_path, bind_address, server_base_url, key_blacklist }
    }

    pub fn jinja_context(&self) -> minijinja::Value {
        context! {
            server_base_url => self.server_base_url.clone(),
            bind_address => self.bind_address.clone(),
            link_data_path => self.link_data_path.to_string_lossy().to_string()
        }
    }
}

fn default_link_data_path() -> PathBuf {
    let mut result = dirs::data_local_dir().unwrap();
    result.push("landmower/links.toml");
    result
}