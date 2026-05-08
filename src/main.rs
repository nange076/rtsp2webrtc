mod api;
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
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[derive(Clone)]
struct AppState {
    stream_manager: Arc<StreamManager>,
    config: Config,
    start_time: Instant,
}

#[derive(serde::Deserialize)]
struct WsParams {
    #[serde(default)]
    stream: Option<String>,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let config = Config::load();

    // ── logging setup ──
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "rtsp2webrtc=info".into());

    match config.logging.format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_current_span(false)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }

    let start_time = Instant::now();

    info!(
        "Starting RTSP → WebRTC gateway on {}",
        config.server.bind_addr
    );
    for s in &config.streams {
        info!("  stream '{}' ({}) → {}", s.id, s.name, mask_url(&s.url));
    }

    let stream_manager = Arc::new(StreamManager::new());

    let api_state = api::ApiState {
        stream_manager: Arc::clone(&stream_manager),
        config: config.clone(),
        start_time,
    };

    // ── CORS ──
    let cors = if config.cors.allowed_origins.iter().any(|o| o == "*") {
        CorsLayer::permissive()
    } else if config.cors.allowed_origins.is_empty() {
        CorsLayer::new() // restrictive: same-origin only
    } else {
        let origins: Vec<_> = config
            .cors
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::list(origins))
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(AppState {
            stream_manager: Arc::clone(&stream_manager),
            config: config.clone(),
            start_time,
        })
        .merge(
            Router::new()
                .route("/health", get(api::health))
                .route("/api/streams", get(api::list_streams))
                .route("/api/streams/{id}", get(api::stream_detail))
                .with_state(api_state),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let bind_addr = config.server.bind_addr;

    match &config.tls {
        Some(tls) => {
            info!("TLS enabled, loading certs");
            use axum_server::tls_rustls::RustlsConfig;
            use axum_server::Handle;
            let tls_config = RustlsConfig::from_pem_file(&tls.cert, &tls.key)
                .await
                .expect("failed to load TLS cert/key");
            let handle = Handle::new();
            tokio::spawn({
                let h = handle.clone();
                async move {
                    tokio::signal::ctrl_c().await.ok();
                    info!("Shutting down (grace period 5s)...");
                    h.graceful_shutdown(Some(Duration::from_secs(5)));
                }
            });
            axum_server::bind_rustls(bind_addr, tls_config)
                .handle(handle)
                .serve(app.into_make_service())
                .await?;
        }
        None => {
            let listener = tokio::net::TcpListener::bind(bind_addr).await?;
            info!("HTTP server listening");
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    tokio::signal::ctrl_c().await.ok();
                    info!("Shutting down (grace period 5s)...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await?;
        }
    }

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
) -> impl IntoResponse {
    let stream_id = params
        .stream
        .unwrap_or_else(|| state.config.default_stream_id().to_string());

    let stream_config = match state.config.find_stream(&stream_id) {
        Some(sc) => sc.clone(),
        None => {
            return axum::response::Response::builder()
                .status(404)
                .body(format!("stream '{stream_id}' not found").into())
                .unwrap();
        }
    };

    ws.on_upgrade(move |socket| async move {
        // Panic isolation: catch panics in the signaling task so one
        // peer crash does not bring down the whole server.
        let result = tokio::task::spawn(async move {
            match state
                .stream_manager
                .subscribe(
                    &stream_id,
                    &stream_config.url,
                    state.config.limits.max_peers,
                    state.config.limits.max_per_stream,
                )
                .await
            {
                Ok((relay, codec_info, sid)) => {
                    if let Err(e) = signaling::handle_signaling(
                        socket,
                        relay,
                        codec_info,
                        sid,
                        state.stream_manager,
                    )
                    .await
                    {
                        error!("Signaling error: {e}");
                    }
                }
                Err(e) => {
                    warn!("Subscription rejected: {e}");
                }
            }
        })
        .await;

        if let Err(join_err) = result {
            if join_err.is_panic() {
                error!("WebSocket handler panicked: {join_err}");
            }
        }
    })
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
