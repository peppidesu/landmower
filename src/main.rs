use std::{
    collections::{hash_map, HashMap}, 
    fmt::Display, 
    hash::{Hash, Hasher}, 
    path::{Path, PathBuf}, 
    sync::Arc, 
    time::Duration
};
use axum::{
    extract::State, 
    http::StatusCode, 
    response::Redirect, 
    routing, 
    Json, 
    Router
};

use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use base64::prelude::*;
use concurrent_queue::ConcurrentQueue;

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    link: String,
    metadata: EntryMetadata
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EntryMetadata {
    used: u64,
    last_used: std::time::SystemTime,
    created: std::time::SystemTime
}

#[derive(Debug)]
pub struct LinkAccessEvent {
    key: String,
    timestamp: std::time::SystemTime,
}

/// Stores alias->link mappings and the reverse mapping.
#[derive(Clone)]
pub struct Links { 
    /// Forward hashmap is used for finding the associated link for a given alias.
    forward_map: HashMap<String, Entry>, 
    /// Inverse of the forward hashmap.
    /// The forward mapping is surjective, so each link can have multiple associated aliases.
    /// 
    /// Note: might be worth benching to see if linear search is actually slower.
    reverse_map: HashMap<String, Vec<String>>,
}

pub type HttpError = (StatusCode, String);

impl Links {
    /// Load link data from the given file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {        
        let path = path.as_ref();

        if !path.exists() {
            // Create the directory if it doesn't exist.
            std::fs::create_dir_all(
                path.parent()
                    .ok_or(format!("Invalid link data path: '{}'", path.display()))?
            ).map_err(|e| format!("Could not create directory: {e}"))?;
            
            // Create empty link storage & write to file
            let result: Self = Self { 
                forward_map: HashMap::new(), 
                reverse_map: HashMap::new() 
            };
            result.save(path)?;
            Ok(result)
        } else {
            // Read file contents
            let data = std::fs::read_to_string(path)
                .map_err(|e| format!("Could not load links: {e}"))?;

            let forward_map: HashMap<String, Entry> = toml::from_str(&data).unwrap();

            // Build reverse lookup
            let mut reverse_map: HashMap<String, Vec<String>> = HashMap::new();
            for (k, v) in &forward_map {
                if reverse_map.contains_key(&v.link) {
                    // link already has associated key; add to existing list
                    reverse_map.get_mut(&v.link).unwrap().push(k.clone());
                } else {
                    // create a new entry for this link
                    reverse_map.insert(v.link.clone(), vec![k.clone()]);
                }
            }
            Ok(Self { forward_map, reverse_map })
        }
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.forward_map.get(key)
    }
    
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.forward_map.get_mut(key)
    }

    /// Insert a new mapping with a generated key and the given link.
    ///
    /// ## Errors
    ///
    /// This function will return an error if the key is already in use, a.k.a. the link
    /// already has an associated mapping 
    pub fn add(&mut self, link: String) -> Result<Entry, String> {
        let key = self.generate_key(&link)        
            .ok_or("Link already has an associated alias.".to_string())?;
        self.add_named(key, link)
    }
    
    fn generate_key(&self, link: &str) -> Option<String> {
        // hash + base64 encode
        let mut hasher = std::hash::DefaultHasher::new();
        link.hash(&mut hasher);
        let hash = BASE64_URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        // take first 4 characters, keep adding if there is a collision
        for i in 4..=hash.len() {
            let key = &hash[..i];
            if let Some(other) = self.forward_map.get(key) { 
                if other.link == link {
                    return None;
                }                
                continue;
            }
            return Some(key.into());
        }

        None // hash collision -> link already present in storage
    }

    /// Insert a new mapping with the given key and link.
    ///
    /// ## Errors
    ///
    /// This function will return an error if the given key is already in use.
    pub fn add_named(&mut self, key: String, link: String) -> Result<Entry, String> {
        let entry = Entry {
            link,
            metadata: EntryMetadata {
                used: 0,
                last_used: std::time::SystemTime::now(),
                created: std::time::SystemTime::now()
            }
        };
        // Update reverse hashmap
        match self.reverse_map.entry(entry.link.clone()) {
            hash_map::Entry::Occupied(mut e) => { 
                e.get_mut().push(key.clone()); 
            },
            hash_map::Entry::Vacant(e) => { 
                e.insert(vec![key.clone()]); 
            },
        }
        // Update forward hashmap
        if let hash_map::Entry::Vacant(e) = self.forward_map.entry(key) {
            e.insert(entry.clone());
            Ok(entry)
        } else {
            Err("Key already in use.".into())
        }
    }

    /// Remove the given mapping.
    /// 
    /// Returns `None` if the link alias does not exist.
    pub fn remove(&mut self, key: &str) -> Option<Entry> {
        let entry = self.forward_map.remove(key);
        
        // Update reverse hashmap
        if let Some(e) = entry {
            let reverse = self.reverse_map.get_mut(&e.link)
                .expect("Missing reverse lookup entry (invalid state)");
            let idx = reverse.iter().position(|x| *x == e.link)
                .expect("Missing reverse lookup entry (invalid state)");
            reverse.remove(idx);
            Some(e)
        } else {
            None
        }
    }

    /// Find aliases that map to the given link.
    /// 
    /// Returns `None` if the link has no associated aliases.
    pub fn find_by_link(&self, link: impl AsRef<str>) -> Option<&[String]> {
        self.reverse_map.get(link.as_ref()).map(|v| v.as_slice())
    }

    /// Save link data to the given file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String>{
        let path = path.as_ref();
        let data = toml::to_string(&self.forward_map.iter().collect::<HashMap<_, _>>())
            .unwrap();
        std::fs::write("data.toml", data)
            .map_err(|e| format!("Could not write to file '{}': {}", path.display(), e))?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Config {
    link_data_path: PathBuf,
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

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    links: Arc<RwLock<Links>>,
    access_event_queue: Arc<ConcurrentQueue<LinkAccessEvent>>
}

async fn redirect(
    key: axum::extract::Path<String>, 
    State(state): State<AppState>
) -> Redirect {
    let links = state.links.read().await;
    let link = &links.get(&key).unwrap().link;

    let req = LinkAccessEvent {
        key: key.clone(),
        timestamp: std::time::SystemTime::now()
    };
    if let Err(e) = state.access_event_queue.push(req) {
        eprintln!("Failed to push update request for link '{}': {:?}",  key.as_str(), e);
    }

    Redirect::permanent(link)
}

#[derive(Deserialize)]
struct RequestLinkData {
    key: Option<String>,
    link: String,
}

async fn get_links(
    State(state): State<AppState>
) -> Json<Vec<(String, Entry)>> {
    let links = state.links.read().await;
    let res = links.forward_map.clone().into_iter().collect::<Vec<_>>();
    Json(res)
}

fn server_error<E: Display>(e: E, msg: impl AsRef<str>) -> HttpError {
    let msg = msg.as_ref();
    tracing::error!("{e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR, 
        format!("{msg}\nSee logs for more info.")
    )
}

#[axum::debug_handler]
async fn add_link(
    State(state): State<AppState>,
    Json(req): Json<RequestLinkData>,
) -> Result<Json<Entry>, HttpError> {
    let mut links = state.links.write().await;
        
    let entry = match req.key {
        Some(key) => links.add_named(key, req.link),
        None => links.add(req.link)
    }.map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    links.save(&state.config.link_data_path)
        .map_err(|e| server_error(e, "Could not create link: IO error"))?;

    Ok(Json(entry))
}

async fn get_link(
    State(state): State<AppState>,
    key: axum::extract::Path<String>
) -> Result<Json<Entry>, HttpError> {
    let links = state.links.read().await;
    let link = links.get(&key)
        .ok_or((StatusCode::NOT_FOUND, "Link not found".to_string()))?;
    Ok(Json(link.clone()))
}

async fn delete_link(
    State(state): State<AppState>,
    key: axum::extract::Path<String>
) -> Result<Json<String>, HttpError> {
    let mut links = state.links.write().await;
    links.remove(key.as_str())
        .ok_or((StatusCode::NOT_FOUND, "Link not found".to_string()))?;
    Ok(Json("ok".to_string()))
}

async fn metadata_update_worker(state: AppState) {
    loop {
        if !state.access_event_queue.is_empty() {
            let mut links = state.links.write().await;
            while let Ok(el) = state.access_event_queue.pop() {
                let link = links.get_mut(&el.key).unwrap();
                link.metadata.used += 1;
                link.metadata.last_used = link.metadata.last_used.max(el.timestamp);
            }        
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let state = AppState { 
        config: Config::from_env().into(),
        links: RwLock::new(Links::load("data.toml").unwrap()).into(), 
        access_event_queue: ConcurrentQueue::unbounded().into()
    };

    let app = Router::new()
        .route(
            "/s/:key", 
            routing::get(redirect)
        )
        .route(
            "/api/links", 
            routing::get(get_links)
                    .post(add_link)
        )
        .route(
            "/api/links/:key", 
            routing::get(get_link)
                    .delete(delete_link)
        )        
        .with_state(state.clone());
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let worker_handle = tokio::task::spawn(metadata_update_worker(state.clone()));

    axum::serve(listener, app).await.unwrap();
    worker_handle.await.unwrap();
}