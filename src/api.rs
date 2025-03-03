use axum::{extract::State, http::{StatusCode, Uri}, routing, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{links::Entry, AppState};

pub type HttpError = (StatusCode, String);

pub mod jsend {
    use std::ops::FromResidual;

    use axum::{response::IntoResponse, Json, http::status::StatusCode};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "status", content = "data", rename_all = "lowercase")]
    pub enum Jsend<T, F> {
        Success(T),
        Fail(F),
        Error(String),
    }
    impl<T,F> Jsend<T, F> {
        pub fn is_success(&self) -> bool {
            matches!(self, Jsend::Success(_))
        }
        pub fn is_fail(&self) -> bool {
            matches!(self, Jsend::Fail(_))
        }
        pub fn is_error(&self) -> bool {
            matches!(self, Jsend::Error(_))
        }

        pub fn success(self) -> Option<T> {
            match self {
                Jsend::Success(data) => Some(data),
                _ => None
            }
        }
        pub fn fail(self) -> Option<F> {
            match self {
                Jsend::Fail(fail) => Some(fail),
                _ => None
            }
        }
        pub fn error(self) -> Option<String> {
            match self {
                Jsend::Error(message) => Some(message),
                _ => None
            }
        }
    }

    impl<T: Serialize, F: Serialize> IntoResponse for Jsend<T, F> {
        fn into_response(self) -> axum::response::Response {
            match &self {
                Jsend::Success(_)   => (StatusCode::OK, Json(self)).into_response(),
                Jsend::Fail(_)      => (StatusCode::OK, Json(self)).into_response(),
                Jsend::Error(_)     => (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response(),                
            }
        }
    }

    impl <T, F> From<Result<T, F>> for Jsend<T, F> {
        fn from(result: Result<T, F>) -> Self {
            match result {
                Ok(data) => Jsend::Success(data),
                Err(fail) => Jsend::Fail(fail)
            }
        }
    }

    impl<T, F> FromResidual<Result<std::convert::Infallible, String>> for Jsend<T, F> {
        fn from_residual(residual: Result<std::convert::Infallible, String>) -> Self {
            match residual {                
                Err(message) => Jsend::Error(message)
            }
        }
    }

    impl<T, F> From<T> for Jsend<T, F> {
        fn from(data: T) -> Self {
            Jsend::Success(data)
        }
    }
    
}
use jsend::*;

trait Validator {
    type Fail;
    async fn validate(&self, state: &AppState) -> Option<Self::Fail>;
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
        .route(
            "/validate/add_link",
            routing::post(validate_add_link)
        )
}

#[derive(Serialize, Deserialize)]
struct ResponseEntry {
    key: String,
    link: String,
    metadata: crate::links::EntryMetadata,
}
impl From<(String, Entry)> for ResponseEntry {
    fn from((key, entry): (String, Entry)) -> Self {
        Self {
            key,
            link: entry.link,
            metadata: entry.metadata
        }
    }
}

#[derive(Serialize, Deserialize)]
struct AddLinkRequest {
    key: Option<String>,
    link: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddLinkSuccessResponse {
    key: String,
    entry: Entry,
}

#[derive(Serialize, Deserialize)]
pub struct AddLinkFailResponse {
    key: Option<String>,
    link: Option<String>,
}

impl Validator for AddLinkRequest {
    type Fail = AddLinkFailResponse;
    async fn validate(&self, state: &AppState) -> Option<Self::Fail> {
        let mut fail = AddLinkFailResponse {
            key: None,
            link: None
        };
    
        if self.link.is_empty() {
            fail.link = Some("Link cannot be empty".to_string());
        }
        else {
            match self.link.parse::<Uri>() {
                Ok(uri) => {
                    if uri.host().is_none() {
                        fail.link = Some("Invalid URL".to_string());
                    }                          
                },
                Err(_) => {
                    fail.link = Some("Invalid URL".to_string());           
                }
            }
        }
    
        if let Some(key) = &self.key {
            if key.len() < 4 {
                fail.key = Some("Key cannot be less than 4 characters".to_string());
            }
            else if key.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                fail.key = Some("Key can only contain 0-9, A-Z, a-z, _ or -".to_string());
            }
            else if state.config.key_blacklist.iter().any(|k| k == key) {
                fail.key = Some(format!("Key '{key}' is disallowed"));
            }
            else if state.links.read().await.get(key).is_some() {
                fail.key = Some("Key already in use".to_string());
            }
        }
    
        if fail.key.is_some() || fail.link.is_some() {
            Some(fail)
        } else {
            None
        }
    }
}

async fn add_link(
    State(state): State<AppState>,
    Json(req): Json<AddLinkRequest>,
) -> Jsend<AddLinkSuccessResponse, AddLinkFailResponse> {
    if let Some(fail) = req.validate(&state).await {
        return Jsend::Fail(fail);
    }

    let mut links = state.links.write().await;
    
    let (key, entry) = match req.key {
        Some(key) => (key.clone(), links.add_named(key, req.link)
            .map_err(|_| "Duplicate key after validation (unreachable state)".to_string())?),  
        None => links.add(req.link)
    };
    
    links.save(&state.config.link_data_path)
        .map_err(|_| "Could not create link: IO error".to_string())?;

    Jsend::Success(AddLinkSuccessResponse { key, entry })
}

type GetLinkResponse = ResponseEntry;
async fn get_link(
    State(state): State<AppState>,
    key: axum::extract::Path<String>
) -> Jsend<GetLinkResponse, String> {
    let links = state.links.read().await;
    links.get(&key)
        .map(|entry| (key.clone(), entry.clone()).into())
        .ok_or("Link not found".to_string())
        .into()
}

async fn delete_link(
    State(state): State<AppState>,
    key: axum::extract::Path<String>
) -> Jsend<(), String> {
    let mut links = state.links.write().await;
    links.remove(key.as_str())
        .map(|_| ())    
        .ok_or("Link not found".to_string())
        .into()
}



type GetLinksResponse = Vec<ResponseEntry>;
async fn get_links(
    State(state): State<AppState>
) -> Jsend<GetLinksResponse, ()> {
    let links = state.links.read().await;
    let res = links.iter()
        .map(|(k, v)| (k.clone(), v.clone()).into())
        .collect::<Vec<_>>();
    Jsend::Success(res)
}

async fn validate_add_link(
    State(state): State<AppState>,
    Json(req): Json<AddLinkRequest>,
) -> Jsend<(), AddLinkFailResponse> {
    match req.validate(&state).await {
        Some(fail) => Jsend::Fail(fail),
        None => Jsend::Success(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::{env::temp_dir, sync::Arc};

    use rand::{RngCore, SeedableRng};
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;    
    use crate::Config;

    use super::*; 

    fn cleanup(path: &Path) {
        std::fs::remove_file(path)
            .unwrap_or(());
    }
    fn random_links_path() -> PathBuf {        
        let mut rng = rand::rngs::SmallRng::seed_from_u64(1);
        let suffix = rng.next_u64();
        temp_dir().join(format!("links-{}.toml", suffix))
    }

    async fn setup_test_api(links_path: &Path) -> (String, mpsc::Sender<()>) {
        let state = AppState {
            config: Arc::new(Config { 
                link_data_path: PathBuf::from(links_path),
                bind_address: "".to_string(),
                server_base_url: "".to_string(),
                key_blacklist: vec![],
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
                .json(&AddLinkRequest { 
                    key: None, link: 
                    "https://example.com".to_string() 
                })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);

            let body: Jsend<AddLinkSuccessResponse, AddLinkFailResponse> = res.json().await.unwrap();
            assert!(body.is_success());

            let data = body.success().unwrap();
            assert_eq!(data.entry.link, "https://example.com");

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }

        #[tokio::test]
        async fn with_key() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: Some("test".to_string()), 
                    link: "https://example.com".to_string() 
                })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);

            let body: Jsend<AddLinkSuccessResponse, AddLinkFailResponse> = res.json().await.unwrap();
            assert!(body.is_success());

            let data = body.success().unwrap();
            assert_eq!(data.entry.link, "https://example.com");

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }


        #[tokio::test]
        async fn key_already_exists() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: Some("test".to_string()), 
                    link: "https://example1.com".to_string() 
                })
                .send().await.unwrap();            

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: Some("test".to_string()), 
                    link: "https://example2.com".to_string() 
                })
                .send().await.unwrap();   

            assert_eq!(res.status(), 200);

            let body: Jsend<AddLinkSuccessResponse, AddLinkFailResponse> = res.json().await.unwrap();
            assert!(body.is_fail());

            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }

        #[tokio::test]
        async fn link_already_exists() {
            let links_path = random_links_path();
            let (addr, shutdown) = setup_test_api(&links_path).await;

            let client = reqwest::Client::new();

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: None, 
                    link: "https://example.com".to_string() 
                })
                .send().await.unwrap();

            let key1 = res
                .json::<Jsend<AddLinkSuccessResponse, AddLinkFailResponse>>().await.unwrap()
                .success().unwrap()
                .key;

            let res = client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: None, 
                    link: "https://example.com".to_string() 
                })
                .send().await.unwrap();

            assert_eq!(res.status(), 200);

            let body: Jsend<AddLinkSuccessResponse, AddLinkFailResponse> = res.json().await.unwrap();
            assert!(body.is_success());

            let data = body.success().unwrap();
            assert_eq!(data.key, key1);

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

            client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: Some("test".to_string()), 
                    link: "https://example.com".to_string() 
                })
                .send().await.unwrap();

            let res = client.get(format!("{addr}/links/test"))
                .send().await.unwrap();
            assert_eq!(res.status(), 200);

            let body = res.json::<Jsend<GetLinkResponse, String>>().await.unwrap();
            assert!(body.is_success());
            
            let data = body.success().unwrap();
            assert_eq!(data.link, "https://example.com");

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
            assert_eq!(res.status(), 200);

            let body = res.json::<Jsend<GetLinkResponse, String>>().await.unwrap();
            assert!(body.is_fail()); 

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
    
            client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: Some("test".to_string()), 
                    link: "https://example.com".to_string() 
                })
                .send().await.unwrap();
    
            let res = client.delete(format!("{addr}/links/test"))
                .send().await.unwrap();
    
            assert_eq!(res.status(), 200);

            let body = res.json::<Jsend<(), String>>().await.unwrap();
            assert!(body.is_success());
    
            let res = client.get(format!("{addr}/links/test"))
                .send().await.unwrap();
            assert_eq!(res.status(), 200);

            let body = res.json::<Jsend<GetLinkResponse, String>>().await.unwrap();
            assert!(body.is_fail());        
    
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
            assert_eq!(res.status(), 200);
            
            let body = res.json::<Jsend<(), String>>().await.unwrap();
            assert!(body.is_fail());
    
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
    
            client.post(format!("{addr}/links"))
                .json(&AddLinkRequest { 
                    key: Some("test".to_string()), 
                    link: "https://example.com".to_string() 
                })
                .send().await.unwrap();
    
            let res = client.get(format!("{addr}/links"))
                .send().await.unwrap();
            assert_eq!(res.status(), 200);
                        
            let body = res.json::<Jsend<GetLinksResponse, ()>>().await.unwrap();
            assert!(body.is_success());

            let data = body.success().unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].key, "test");
            assert_eq!(data[0].link, "https://example.com");
    
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

            let body = res.json::<Jsend<GetLinksResponse, ()>>().await.unwrap();
            assert!(body.is_success());

            let data = body.success().unwrap();
            assert_eq!(data.len(), 0);
    
            shutdown.send(()).await.unwrap();
            cleanup(&links_path);
        }
    }
}