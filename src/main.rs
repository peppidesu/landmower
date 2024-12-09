use std::{collections::HashMap, ops::Add, sync::Arc, time::{Duration, SystemTime}};
use axum::{extract::State, http::StatusCode, response::Redirect, routing, Json, Router};
use concurrent_queue::ConcurrentQueue;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

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

#[derive(Clone)]
pub struct Links(HashMap<String, Entry>);

pub type HttpError = (StatusCode, String);

impl Links {
    pub fn load() -> Self {
        let data = std::fs::read_to_string("data.toml").unwrap();
        let map: HashMap<_, _> = toml::from_str(&data).unwrap();
        let map = map.into_iter().collect();
        Self(map)
    }

    pub fn add(&mut self, link: String) -> String {
        let key = self.generate_key();
        let entry = Entry {
            link,
            metadata: EntryMetadata {
                used: 0,
                last_used: std::time::SystemTime::now(),
                created: std::time::SystemTime::now()
            }
        };
        self.0.insert(key.clone(), entry);
        key
    }
    pub fn add_named(&mut self, key: String, link: String) -> Option<Entry>{
        let entry = Entry {
            link,
            metadata: EntryMetadata {
                used: 0,
                last_used: std::time::SystemTime::now(),
                created: std::time::SystemTime::now()
            }
        };
        self.0.insert(key, entry)
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.0.get(key)
    }
    
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.0.get_mut(key)
    }

    pub fn generate_key(&self) -> String {
        todo!()
    }

    pub fn save(&self) {
        let data = toml::to_string(&self.0.iter().collect::<HashMap<_, _>>()).unwrap();
        std::fs::write("data.toml", data).unwrap();
    }
}

pub struct Config {
    url_base: String,
}
impl Config {
    pub fn from_env() -> Self {
        let url_base = std::env::var("URL_BASE").unwrap_or_else(
            |_| "http://localhost:8080/".to_string()
        );
        Self { url_base }
    }
}

#[derive(Clone)]
struct AppState {
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
) -> Json<HashMap<String, Entry>> {
    let links = state.links.read().await;
    Json(links.0.clone())
}

#[axum::debug_handler]
async fn add_link(
    State(state): State<AppState>,
    Json(req): Json<RequestLinkData>,
) -> Result<Json<String>, HttpError> {
    let mut links = state.links.write().await;
    let key = match req.key {
        Some(key) => key,
        None => links.generate_key()
    };
    links.add_named(key.clone(), req.link)
        .ok_or((StatusCode::BAD_REQUEST, "Link already exists".to_string()))?;
    links.save();
    Ok(Json(key))
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

async fn update_link(
    State(state): State<AppState>,
    key: axum::extract::Path<String>,
    Json(RequestLinkData { key: new_key, link }): Json<RequestLinkData>
) -> Result<Json<Entry>, HttpError> {
    let mut links = state.links.write().await;
    if let Some(new_key) = new_key {
        let entry = links.0.remove(key.as_str())
            .ok_or((StatusCode::NOT_FOUND, "Link not found".to_string()))?;
        links.0.insert(new_key, entry.clone())
            .ok_or((StatusCode::BAD_REQUEST, "Link already exists".to_string()))?;

        Ok(Json(entry))
    }
    else {
        
        let entry = links.get_mut(&key)
            .ok_or((StatusCode::NOT_FOUND, "Link not found".to_string()))?;
        entry.link = link;

        Ok(Json(entry.clone()))
    }
    
}

async fn delete_link(
    State(state): State<AppState>,
    key: axum::extract::Path<String>
) -> Result<Json<String>, HttpError> {
    let mut links = state.links.write().await;
    links.0.remove(key.as_str())
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
        links: Arc::new(RwLock::new(Links::load())), 
        access_event_queue: Arc::new(ConcurrentQueue::unbounded()) 
    };

    let app = Router::new()
        .route("/s/:key", routing::get(redirect))
        .route("/api/links", routing::get(get_links))
        .route("/api/links", routing::post(add_link))
        .route("/api/links/:key", routing::get(get_link))
        .route("/api/links/:key", routing::post(update_link))
        .route("/api/links/:key", routing::delete(delete_link))
        .with_state(state.clone());
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let worker_handle = tokio::task::spawn(metadata_update_worker(state.clone()));

    axum::serve(listener, app).await.unwrap();
    worker_handle.await.unwrap();
}