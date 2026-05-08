use crate::error::AppResult;
use crate::rtp_relay::RtpRelay;
use crate::rtsp::{H264CodecInfo, RtspPuller};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info};
use uuid::Uuid;

pub type StreamId = String;
pub type SubscriberId = String;

/// Holds all runtime objects for an active RTSP stream.
struct ActiveStream {
    relay: Arc<RtpRelay>,
    codec_info: H264CodecInfo,
    subscriber_count: usize,
    puller: Option<RtspPuller>,
    idle_timer: Option<JoinHandle<()>>,
}

/// Central registry for all active RTSP streams.
///
/// Ensures one RTSP source → N WebRTC consumers.
/// Starts RTSP pull lazily on first subscriber.
/// Stops pull after idle timeout when last subscriber leaves.
pub struct StreamManager {
    streams: RwLock<HashMap<StreamId, ActiveStream>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            streams: RwLock::default(),
        }
    }

    /// Get or create a stream for the given RTSP URL.
    ///
    /// Returns the shared relay and codec info. On first call for a URL,
    /// starts an RTSP puller in the background.
    pub async fn subscribe(
        self: &Arc<Self>,
        rtsp_url: &str,
    ) -> AppResult<(Arc<RtpRelay>, H264CodecInfo, StreamId)> {
        // Check if any active stream is already running
        // (Phase 2: single-stream, so any active stream matches)
        let existing_id = {
            let streams = self.streams.read().await;
            streams
                .iter()
                .find(|(_, s)| s.subscriber_count > 0)
                .map(|(id, _)| id.clone())
        };

        // If we have an active stream, increment count
        if let Some(ref stream_id) = existing_id {
            let mut streams = self.streams.write().await;
            if let Some(active) = streams.get_mut(stream_id) {
                // Cancel idle timer if one is running
                if let Some(timer) = active.idle_timer.take() {
                    timer.abort();
                    info!("Cancelled idle timer for stream {stream_id}");
                }
                active.subscriber_count += 1;
                let relay = Arc::clone(&active.relay);
                let codec_info = active.codec_info.clone();
                info!(
                    "Stream {stream_id}: {} subscriber(s)",
                    active.subscriber_count
                );
                return Ok((relay, codec_info, stream_id.clone()));
            }
        }

        // No active stream — create one and start RTSP pull
        let stream_id = Uuid::new_v4().to_string();
        let relay = Arc::new(RtpRelay::new(256));

        info!("Stream {stream_id}: starting RTSP pull for {rtsp_url}");

        // Start RTSP pull (slow, no lock held)
        let relay_for_pull = Arc::clone(&relay);
        let puller = RtspPuller::start(rtsp_url, relay_for_pull).await?;
        let codec_info = puller.codec_info.clone();

        // Store in registry
        {
            let mut streams = self.streams.write().await;
            streams.insert(
                stream_id.clone(),
                ActiveStream {
                    relay: Arc::clone(&relay),
                    codec_info: codec_info.clone(),
                    subscriber_count: 1,
                    puller: Some(puller),
                    idle_timer: None,
                },
            );
        }

        info!("Stream {stream_id}: active with 1 subscriber");
        Ok((relay, codec_info, stream_id))
    }

    /// Called when a browser disconnects.
    ///
    /// Decrements the subscriber count. If it reaches zero, starts an idle
    /// timer that will stop the RTSP pull after a delay.
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
                        info!(
                            "Stream {sid}: idle timeout ({idle_secs}s), stopping RTSP pull"
                        );
                        active.puller.take(); // Drop → abort RTSP task
                        streams.remove(&sid);
                        debug!("Stream {sid}: removed from registry");
                    }
                }
            });

            // Store the timer handle so we can cancel it if someone subscribes
            {
                let mut streams = self.streams.write().await;
                if let Some(active) = streams.get_mut(stream_id) {
                    active.idle_timer = Some(timer);
                }
            }
        }
    }
}
