use crate::config::Config;
use crate::stream::StreamManager;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct ApiState {
    pub stream_manager: Arc<StreamManager>,
    pub config: Config,
    pub start_time: Instant,
}

// ── Response types ──

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub uptime_secs: u64,
    pub configured_streams: usize,
    pub active_streams: usize,
    pub total_peers: usize,
}

#[derive(Serialize)]
pub struct StreamItem {
    pub id: String,
    pub name: String,
    /// Masked URL (credentials hidden)
    pub url: String,
    pub subscribers: usize,
    pub connected: bool,
    pub codec: Option<String>,
    pub payload_type: Option<u8>,
}

#[derive(Serialize)]
pub struct StreamsResponse {
    pub streams: Vec<StreamItem>,
}

// ── Handlers ──

pub async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    let active = state.stream_manager.list_streams().await;
    Json(HealthResponse {
        status: "ok",
        uptime_secs: state.start_time.elapsed().as_secs(),
        configured_streams: state.config.streams.len(),
        active_streams: active.iter().filter(|s| s.connected).count(),
        total_peers: state.stream_manager.total_peers(),
    })
}

pub async fn list_streams(State(state): State<ApiState>) -> Json<StreamsResponse> {
    let runtime = state.stream_manager.list_streams().await;

    // Merge configured streams with runtime state
    let items: Vec<StreamItem> = state
        .config
        .streams
        .iter()
        .map(|cfg| {
            let rt = runtime.iter().find(|r| r.id == cfg.id);
            StreamItem {
                id: cfg.id.clone(),
                name: if cfg.name.is_empty() {
                    cfg.id.clone()
                } else {
                    cfg.name.clone()
                },
                url: mask_url(&cfg.url),
                subscribers: rt.map(|r| r.subscribers).unwrap_or(0),
                connected: rt.map(|r| r.connected).unwrap_or(false),
                codec: rt.and(Some("h264".to_string())),
                payload_type: rt.and(Some(96u8)),
            }
        })
        .collect();

    Json(StreamsResponse { streams: items })
}

pub async fn stream_detail(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<StreamItem>, String> {
    let cfg = state
        .config
        .find_stream(&id)
        .ok_or_else(|| "stream not found".to_string())?;

    let runtime = state.stream_manager.list_streams().await;
    let rt = runtime.iter().find(|r| r.id == id);

    Ok(Json(StreamItem {
        id: cfg.id.clone(),
        name: if cfg.name.is_empty() {
            cfg.id.clone()
        } else {
            cfg.name.clone()
        },
        url: mask_url(&cfg.url),
        subscribers: rt.map(|r| r.subscribers).unwrap_or(0),
        connected: rt.map(|r| r.connected).unwrap_or(false),
        codec: rt.and(Some("h264".to_string())),
        payload_type: rt.and(Some(96u8)),
    }))
}

fn mask_url(url: &str) -> String {
    if let Ok(u) = url::Url::parse(url) {
        let mut masked = format!("{}://", u.scheme());
        if u.username().is_empty() {
            masked.push_str(u.host_str().unwrap_or("?"));
        } else {
            masked.push_str(&format!("***@{}", u.host_str().unwrap_or("?")));
        }
        masked.push_str(&format!(":{}", u.port().unwrap_or(554)));
        masked
    } else {
        url.to_string()
    }
}
