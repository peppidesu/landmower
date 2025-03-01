use std::{path::PathBuf, sync::Arc};

pub mod api;
pub mod links;

use concurrent_queue::ConcurrentQueue;
use links::Links;
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
}

impl Config {
    pub fn from_env() -> Self {     
        let link_data_path = std::env::var("LANDMOWER_LINK_DATA_PATH")
            .map(|s| s.into())
            .unwrap_or_else(|_| default_link_data_path());

        Self { link_data_path }
    }
}

fn default_link_data_path() -> PathBuf {
    let mut result = dirs::data_local_dir().unwrap();
    result.push("landmower/links.toml");
    result
}