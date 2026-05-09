use crate::error::{AppError, AppResult};
use crate::rtp_relay::RtpRelay;
use crate::rtsp::{H264CodecInfo, RtspPuller};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::info;
use uuid::Uuid;

pub type StreamId = String;

/// Holds all runtime objects for an active RTSP stream.
struct ActiveStream {
    url: String,
    relay: Arc<RtpRelay>,
    codec_info: H264CodecInfo,
    subscriber_count: usize,
    puller: Option<RtspPuller>,
    idle_timer: Option<JoinHandle<()>>,
}

/// Central registry for all active RTSP streams.
///
/// Ensures one RTSP source → N WebRTC consumers.
/// Starts RTSP pull when stream is created via the API.
/// Reuses existing pull if the same RTSP URL is requested.
/// Stops pull after idle timeout when last subscriber leaves.
pub struct StreamManager {
    streams: RwLock<HashMap<StreamId, ActiveStream>>,
    total_peers: AtomicUsize,
}

/// Lightweight summary for the REST API.
#[derive(Clone, Serialize)]
pub struct StreamSummary {
    pub id: String,
    pub subscribers: usize,
    pub connected: bool,
    pub url: String,
}

/// Detailed info for a single stream.
#[derive(Clone, Serialize)]
pub struct StreamDetail {
    pub id: String,
    pub subscribers: usize,
    pub connected: bool,
    pub codec: String,
    pub payload_type: u8,
    pub url: String,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            streams: RwLock::default(),
            total_peers: AtomicUsize::new(0),
        }
    }

    /// Total WebRTC peers across all streams.
    pub fn total_peers(&self) -> usize {
        self.total_peers.load(Ordering::Relaxed)
    }

    /// List active streams for the API.
    pub async fn list_streams(&self) -> Vec<StreamSummary> {
        let streams = self.streams.read().await;
        streams
            .iter()
            .map(|(id, s)| StreamSummary {
                id: id.clone(),
                subscribers: s.subscriber_count,
                connected: s.subscriber_count > 0,
                url: s.url.clone(),
            })
            .collect()
    }

    /// Detailed info for a single stream.
    pub async fn stream_info(&self, id: &str) -> Option<StreamDetail> {
        let streams = self.streams.read().await;
        streams.get(id).map(|s| StreamDetail {
            id: id.to_string(),
            subscribers: s.subscriber_count,
            connected: s.subscriber_count > 0,
            codec: "h264".to_string(),
            payload_type: s.codec_info.payload_type,
            url: s.url.clone(),
        })
    }

    /// Create a dynamic stream from an RTSP URL. Starts RTSP pull immediately.
    /// If a stream for the same URL already exists, returns its ID (reuse).
    pub async fn create_dynamic(
        self: &Arc<Self>,
        rtsp_url: &str,
    ) -> AppResult<String> {
        // Check if a stream for this URL already exists
        {
            let streams = self.streams.read().await;
            if let Some((id, _)) = streams.iter().find(|(_, s)| s.url == rtsp_url) {
                info!("Stream {id}: reusing existing RTSP pull for {rtsp_url}");
                return Ok(id.clone());
            }
        }

        let sid = Uuid::new_v4().to_string();
        let relay = Arc::new(RtpRelay::new(256));
        info!("Dynamic stream {sid}: starting RTSP pull for {rtsp_url}");

        let relay_for_pull = Arc::clone(&relay);
        let puller = RtspPuller::start(rtsp_url, relay_for_pull).await?;
        let codec_info = puller.codec_info.clone();

        let mut streams = self.streams.write().await;
        streams.insert(
            sid.clone(),
            ActiveStream {
                url: rtsp_url.to_string(),
                relay,
                codec_info,
                subscriber_count: 0,
                puller: Some(puller),
                idle_timer: None,
            },
        );

        info!("Dynamic stream {sid}: puller active, waiting for subscribers");
        Ok(sid)
    }

    /// Subscribe to an existing stream (created dynamically).
    /// Used when the stream already has an active RTSP pull.
    pub async fn subscribe_existing(
        self: &Arc<Self>,
        stream_id: &str,
        max_peers: usize,
        max_per_stream: usize,
    ) -> AppResult<(Arc<RtpRelay>, H264CodecInfo)> {
        let current_total = self.total_peers.load(Ordering::Relaxed);
        if current_total >= max_peers {
            return Err(AppError::Other(format!(
                "global peer limit reached ({max_peers})"
            )));
        }

        let mut streams = self.streams.write().await;
        let active = streams
            .get_mut(stream_id)
            .ok_or_else(|| AppError::Other(format!("stream '{stream_id}' not found")))?;

        if active.subscriber_count >= max_per_stream {
            return Err(AppError::Other(format!(
                "stream limit reached ({max_per_stream})"
            )));
        }

        if let Some(timer) = active.idle_timer.take() {
            timer.abort();
        }
        active.subscriber_count += 1;
        self.total_peers.fetch_add(1, Ordering::Relaxed);
        info!(
            "Stream {stream_id}: {} subscriber(s)",
            active.subscriber_count
        );
        Ok((Arc::clone(&active.relay), active.codec_info.clone()))
    }

    /// Force-remove a dynamic stream (for DELETE API).
    pub async fn remove_stream(&self, stream_id: &str) -> Result<(), String> {
        let mut streams = self.streams.write().await;
        if let Some(mut active) = streams.remove(stream_id) {
            if let Some(timer) = active.idle_timer.take() {
                timer.abort();
            }
            active.puller.take(); // drop → abort RTSP
            info!("Stream {stream_id}: removed");
            Ok(())
        } else {
            Err(format!("stream '{stream_id}' not found"))
        }
    }

    /// Called when a browser disconnects.
    pub async fn unsubscribe(self: &Arc<Self>, stream_id: &str) {
        let count = {
            let mut streams = self.streams.write().await;
            if let Some(active) = streams.get_mut(stream_id) {
                active.subscriber_count = active.subscriber_count.saturating_sub(1);
                active.subscriber_count
            } else {
                return;
            }
        };

        self.total_peers.fetch_sub(1, Ordering::Relaxed);
        info!("Stream {stream_id}: {count} subscriber(s) remaining");

        if count == 0 {
            let this = Arc::clone(self);
            let sid = stream_id.to_string();
            let idle_secs = 30;

            let timer = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(idle_secs)).await;

                let mut streams = this.streams.write().await;
                if let Some(active) = streams.get_mut(&sid) {
                    if active.subscriber_count == 0 {
                        info!("Stream {sid}: idle timeout, stopping RTSP pull");
                        active.puller.take();
                        streams.remove(&sid);
                    }
                }
            });

            {
                let mut streams = self.streams.write().await;
                if let Some(active) = streams.get_mut(stream_id) {
                    active.idle_timer = Some(timer);
                }
            }
        }
    }
}
