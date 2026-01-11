use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod stream;

#[derive(Clone)]
struct AppState {
    started: Arc<AtomicBool>,
    latest: Arc<RwLock<Option<LatestSlot>>>,
}

#[derive(Debug, Clone, Serialize)]
struct LatestSlot {
    slot: u64,
    parent: Option<u64>,
    status: String,
    created_at_rfc3339: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    let state = AppState {
        started: Arc::new(AtomicBool::new(false)),
        latest: Arc::new(RwLock::new(None)),
    };

    // Start on boot (so the first request already has stream warming up)
    ensure_stream_started(state.clone()).await;

    let app = Router::new()
        .route("/health", get(health))
        .route("/start", post(start))
        .route("/latest", get(latest))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn start(State(state): State<AppState>) -> impl IntoResponse {
    ensure_stream_started(state).await;
    (StatusCode::OK, "started\n")
}

async fn latest(State(state): State<AppState>) -> impl IntoResponse {
    let guard = state.latest.read().await;
    if let Some(v) = guard.clone() {
        (StatusCode::OK, Json(v)).into_response()
    } else {
        (StatusCode::NOT_FOUND, "no data yet\n").into_response()
    }
}

async fn ensure_stream_started(state: AppState) {
    let was_started = state.started.swap(true, Ordering::SeqCst);
    if was_started {
        return;
    }

    info!("starting LaserStream background task");

    tokio::spawn(async move {
        if let Err(e) = stream::run_slot_stream(state.clone()).await {
            error!("LaserStream task failed: {:#}", e);
        }
    });
}
