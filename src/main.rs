mod config;

use axum::{
    extract::{Json, Path, State},
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use config::load_config;
use env_logger::{Builder, Env};
use log::{error, info};
use rand::SeedableRng;
use rand::{rngs::StdRng, seq::IndexedRandom};
use reqwest::{Client, Proxy};
use serde_json::{from_str, Value};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn logging_init() {
    Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] {} {} - {}",
                record.target(),
                "이",
                record.level(),
                record.args()
            )
        })
        .init();
}

#[derive(Clone)]
struct AppState {
    clients: Arc<Vec<Client>>,
    counter: Arc<AtomicUsize>,
    block_engine_urls: Vec<String>,
}

#[tokio::main]
async fn main() {
    logging_init();
    info!("Starting SolArbX Jito Proxy... pid={}", std::process::id());

    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load config: {:?}", e);
            return;
        }
    };

    let clients = config
        .proxy
        .clone()
        .into_iter()
        .map(|url| {
            let proxy = Proxy::all(&url).expect("Invalid proxy");
            reqwest::Client::builder()
                .proxy(proxy)
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to build client")
        })
        .collect::<Vec<_>>();

    let state = AppState {
        clients: Arc::new(clients),
        counter: Arc::new(AtomicUsize::new(0)),
        block_engine_urls: config.block_engine_urls,
    };

    let app = Router::new()
        .route("/api/v1/{*path}", any(handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.http_port);

    let listener = match tokio::net::TcpListener::bind(addr.clone()).await {
        Ok(listener) => listener,
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            return;
        }
    };

    match listener.local_addr() {
        Ok(local_addr) => info!("Listening on http://{}", local_addr),
        Err(e) => {
            error!("Failed to get local address: {}", e);
            return;
        }
    }

    match axum::serve(listener, app).await {
        Ok(_) => {}
        Err(e) => {
            error!("Server failed: {}", e);
            return;
        }
    }
}

async fn handler(
    method: Method,
    headers: HeaderMap,
    Path(path): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let mut rng = StdRng::from_rng(&mut rand::rng());

    let block_engine_url = match state.block_engine_urls.choose(&mut rng) {
        Some(url) => url,
        None => {
            error!("No block engine URLs available");
            return (
                StatusCode::BAD_GATEWAY,
                format!("Request failed: {}", "No block engine URLs available"),
            )
                .into_response();
        }
    };

    let target_url = format!("{}/{}", block_engine_url, path);
    let index = state.counter.fetch_add(1, Ordering::Relaxed) % state.clients.len();
    let client = &state.clients[index];

    let mut req = client.request(method, target_url);

    for (key, value) in headers.iter() {
        if key == "content-length" {
            continue;
        }
        req = req.header(key, value);
    }

    let response = match req.json(&body).send().await {
        Ok(resp) => resp,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, format!("Request failed: {}", e)).into_response();
        }
    };

    let status = response.status();
    let text = response
        .text()
        .await
        .unwrap_or_else(|_| "No body".to_string());

    if let Ok(json) = from_str::<Value>(&text) {
        (status, Json(json)).into_response()
    } else {
        (status, text).into_response()
    }
}
