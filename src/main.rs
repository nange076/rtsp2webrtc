mod config;
mod error;
mod rtp_relay;
mod rtsp;
mod signaling;
mod stream;
mod webrtc_peer;

use crate::config::Config;
use crate::error::AppResult;
use crate::stream::StreamManager;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    stream_manager: Arc<StreamManager>,
    config: Config,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rtsp2webrtc=info".into()),
        )
        .init();

    let config = Config::from_env();
    info!("Starting RTSP → WebRTC gateway on {}", config.bind_addr);
    info!("RTSP source (lazy): {}", config.rtsp_url);

    let stream_manager = Arc::new(StreamManager::new());
    let bind_addr = config.bind_addr;

    let state = AppState {
        stream_manager,
        config,
    };

    let app = axum::Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!("HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down...");
        })
        .await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    "ok"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        // Subscribe to the configured RTSP stream (lazy start on first client)
        match state
            .stream_manager
            .subscribe(&state.config.rtsp_url)
            .await
        {
            Ok((relay, codec_info, stream_id)) => {
                if let Err(e) =
                    signaling::handle_signaling(socket, relay, codec_info, stream_id, state.stream_manager).await
                {
                    error!("Signaling error: {e}");
                }
            }
            Err(e) => {
                warn!("Failed to subscribe to stream: {e}");
            }
        }
    })
}
