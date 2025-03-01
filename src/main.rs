use std::{
    sync::Arc, time::Duration
};

use axum::{
    body::Body, extract::{Path, State}, http::StatusCode, response::Redirect, routing, Router
};
use axum_embed::ServeEmbed;
use rust_embed::Embed;
use tokio::sync::RwLock;
use concurrent_queue::ConcurrentQueue;
use tower_http::trace::TraceLayer;
use http_body_util::BodyExt;

pub use landmower::*;
use links::Links;

#[derive(Embed, Clone)]
#[folder = "static"]
struct PageAssets;

async fn redirect(
    Path(key): Path<String>, 
    State(state): State<AppState>
) -> Result<Redirect, api::HttpError> {
    let links = state.links.read().await;
    let mut link = links.get(&key)
        .ok_or((StatusCode::NOT_FOUND, "Link does not exist.".to_string()))?
        .link.clone();   
    
    if !(link.starts_with("http://") || link.starts_with("https://")) {
        link = format!("http://{}", link);
    }

    let req = LinkAccessEvent {
        key: key.clone(),
        timestamp: std::time::SystemTime::now()
    };

    if let Err(e) = state.access_event_queue.push(req) {
        eprintln!("Failed to push update request for link '{}': {:?}",  key.as_str(), e);
    }

    Ok(Redirect::to(&link))
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
async fn inject_environment(
    req: axum::extract::Request,
    next: axum::middleware::Next
) -> axum::response::Response {
    let res = next.run(req).await;
    let (parts, body) = res.into_parts();
    let content = String::from_utf8(
        body.collect().await.unwrap().to_bytes().to_vec()
    ).unwrap();    
    
    let replaced = content.replace("{% server_url %}", "http://localhost:3000/");    
    
    axum::http::Response::from_parts(parts, Body::from(replaced))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    
    let config = Arc::new(Config::from_env());
    let state = AppState { 
        config: config.clone(),
        links: RwLock::new(Links::load(&config.link_data_path).unwrap()).into(), 
        access_event_queue: ConcurrentQueue::unbounded().into()
    };

    

    let app = Router::new()
        .nest("/api", api::router())
        .nest_service("/static", ServeEmbed::<PageAssets>::new())                
        .layer(axum::middleware::from_fn(inject_environment))
        .route("/:key", routing::get(redirect))
        .with_state(state.clone())
        .layer(TraceLayer::new_for_http());
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let worker_handle = tokio::task::spawn(metadata_update_worker(state.clone()));

    axum::serve(listener, app).await.unwrap();
    worker_handle.await.unwrap();
}