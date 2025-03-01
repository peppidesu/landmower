use std::fmt::Display;

use axum::{extract::State, http::StatusCode, routing, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{links::Entry, AppState};


pub type HttpError = (StatusCode, String);

#[derive(Serialize, Deserialize)]
struct AddLinkRequestBody {
    key: Option<String>,
    link: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddLinkResponseBody {
    key: String,
    entry: Entry,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/links", 
            routing::get(get_links)
                    .post(add_link)
        )
        .route(
            "/links/:key", 
            routing::get(get_link)
                    .delete(delete_link)
        )
}


#[axum::debug_handler]
async fn add_link(
    State(state): State<AppState>,
    Json(req): Json<AddLinkRequestBody>,
) -> Result<Json<AddLinkResponseBody>, HttpError> {
    let mut links = state.links.write().await;
        
    let (key, entry) = match req.key {
        Some(key) => links.add_named(key.clone(), req.link).map(|e| (key, e)),
        None => links.add(req.link)
    }.map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    
    links.save(&state.config.link_data_path)
        .map_err(|e| server_error(e, "Could not create link: IO error"))?;

    Ok(Json(AddLinkResponseBody {
        key,
        entry
    }))
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

async fn get_links(
    State(state): State<AppState>
) -> Json<Vec<(String, Entry)>> {
    let links = state.links.read().await;
    let res = links.iter()
    .map(|(k,v)| (k.clone(), v.clone()))
    .collect::<Vec<_>>();
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::{env::temp_dir, sync::Arc};

    use tokio::net::TcpListener;
    use tokio::sync::mpsc;

    use crate::Config;

    use super::*; 

    fn cleanup(path: &Path) {
        std::fs::remove_file(path)
            .unwrap_or(());
    }
    fn random_links_path() -> PathBuf {
        let suffix = rand::random::<u64>();
        temp_dir().join(format!("links-{}.toml", suffix))
    }

    async fn setup_test_api(links_path: &Path) -> (String, mpsc::Sender<()>) {
        let state = AppState {
            config: Arc::new(Config { 
                link_data_path: PathBuf::from(links_path)
            }),
            links: std::sync::Arc::new(tokio::sync::RwLock::new(crate::Links::default())),
            access_event_queue: std::sync::Arc::new(concurrent_queue::ConcurrentQueue::unbounded())
        };

        let router = router().with_state(state);
        
        let port = 54500;
        let mut listener = TcpListener::bind(format!("127.0.0.1:{port}")).await;
        while listener.is_err() {
            let port = port + 1;
            listener = TcpListener::bind(format!("127.0.0.1:{port}")).await;            
        }
        let listener = listener.unwrap();

        let addr = format!("http://{}", listener.local_addr().unwrap());        

        let (sender, mut receiver) = mpsc::channel(1);              

        tokio::spawn(async move {
            axum::serve(listener, router.into_make_service())
                .with_graceful_shutdown(async move {
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {}
                        _ = receiver.recv() => {}
                    }
                })
                .await.unwrap();
        });

        (addr, sender)
    }

    mod add_link {
        use super::*;
        #[tokio::test]
        async fn without_key() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: None, link: "https://example.com".to_string() })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);
            let body = res.json::<AddLinkResponseBody>().await.unwrap();
            assert_eq!(body.entry.link, "https://example.com");

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }

        #[tokio::test]
        async fn with_key() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: Some("test".to_string()), link: "https://example.com".to_string() })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);
            let body = res.json::<AddLinkResponseBody>().await.unwrap();
            assert_eq!(body.entry.link, "https://example.com");

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }


        #[tokio::test]
        async fn key_already_exists() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: Some("test".to_string()), link: "https://example.com".to_string() })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: Some("test".to_string()), link: "https://example.com".to_string() })
                .send().await.unwrap();

            assert_eq!(res.status(), 400);

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }

        #[tokio::test]
        async fn link_already_exists() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: None, link: "https://example.com".to_string() })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);        
            
            let res = client.post(format!("{addr}/links"))
            .json(&AddLinkRequestBody { key: None, link: "https://example.com".to_string() })
            .send().await.unwrap();

            assert_eq!(res.status(), 400);

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }
    }

    mod get_link {
        use super::*;
        #[tokio::test]
        async fn base_case() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;
    
            let client = reqwest::Client::new();
    
            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: Some("test".to_string()), link: "https://example.com".to_string() })
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
    
            let res = client.get(format!("{addr}/links/test"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
            let body = res.json::<Entry>().await.unwrap();
            assert_eq!(body.link, "https://example.com");
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }

        #[tokio::test]
        async fn not_found() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;
    
            let client = reqwest::Client::new();
    
            let res = client.get(format!("{addr}/links/test"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 404);
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }
    }    
    
    mod delete_link {
        use super::*;
        #[tokio::test]
        async fn base_case() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;
    
            let client = reqwest::Client::new();
    
            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: Some("test".to_string()), link: "https://example.com".to_string() })
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
    
            let res = client.delete(format!("{addr}/links/test"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
    
            let res = client.get(format!("{addr}/links/test"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 404);
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }

        #[tokio::test]
        async fn not_found() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;
    
            let client = reqwest::Client::new();
    
            let res = client.delete(format!("{addr}/links/test"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 404);
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }
    }

    mod get_links {
        use super::*;
        #[tokio::test]
        async fn base_case() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;
    
            let client = reqwest::Client::new();
    
            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequestBody { key: Some("test".to_string()), link: "https://example.com".to_string() })
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
    
            let res = client.get(format!("{addr}/links"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
            let body = res.json::<Vec<(String, Entry)>>().await.unwrap();
            assert_eq!(body.len(), 1);
            assert_eq!(body[0].0, "test");
            assert_eq!(body[0].1.link, "https://example.com");
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }
    
        #[tokio::test]
        async fn empty_table() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;
    
            let client = reqwest::Client::new();
    
            let res = client.get(format!("{addr}/links"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);
            let body = res.json::<Vec<(String, Entry)>>().await.unwrap();
            assert_eq!(body.len(), 0);
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }
    }
}