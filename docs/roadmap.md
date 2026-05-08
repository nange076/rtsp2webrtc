# Development Roadmap

## Completed

| Feature | Description |
|---------|-------------|
| RTSP → WebRTC gateway | Low-latency RTP relay from IP cameras to browsers |
| Multi-client fanout | One RTSP pull → N browser viewers sharing the same session |
| Lazy RTSP start | Only pulls from camera when a browser is watching |
| Idle auto-stop | Stops RTSP pull 30s after last viewer disconnects |
| Reconnect + backoff | Automatic reconnection with exponential backoff (1s → 30s) |
| RTSP authentication | Supports username/password in RTSP URL |
| H264 SPS/PPS extraction | Parses codec parameters from RTSP SDP, generates correct WebRTC fmtp line |
| Raw RTP forwarding | Preserves all RTP header fields (marker bit, CSRC, etc.) — zero decode |
| Keepalive | 55s timeout per RTP recv() prevents camera session expiry |
| TOML configuration | Multi-stream config with limits, CORS, logging, TLS settings |
| Dynamic streams | `POST /api/streams` creates on-demand streams from any RTSP URL |
| REST API | `/health`, `/api/streams`, `/metrics` (Prometheus) |
| WebSocket signaling | SDP offer/answer + ICE candidate exchange |
| Connection limits | Global `max_peers` + per-stream `max_per_stream` |
| API authentication | Optional `api_key` enforced on mutating endpoints |
| API rate limiting | Configurable `create_per_min` on stream creation |
| CORS control | Configurable allowed origins |
| JSON logging | Optional structured log output |
| Panic isolation | Single peer crash does not bring down the server |
| Graceful shutdown | 5s drain timeout on SIGINT |
| TLS/WSS | Optional HTTPS + WebSocket Secure via config |
| Web test client | RTSP URL input, auto-list streams, one-click play |

## Explicitly Excluded

| Feature | Reason |
|---------|--------|
| **Audio support** | Requires separate RtpRelay + audio track architecture. Not needed for pure video surveillance use cases. |
| GPU transcoding | Against design philosophy — the gateway is an RTP relay, not a media processor |
| Distributed deployment | Single-node deployment covers the target use case |
| Browser-side RTSP | Defeats the purpose of WebRTC gateway; browsers cannot decode RTSP natively |
| HLS/DASH | Segmented streaming contradicts the low-latency design goal |
| FFmpeg pipeline | Shell pipelines per client violate the shared-session fanout model |

## Future Candidates

| Priority | Feature | Effort |
|:--------:|---------|:------:|
| Medium | Multi-substream selection (main/sub) | Small |
| Medium | Hot config reload | Medium |
| Medium | H265 → H264 fallback transcoding | Large |
| Low | STUN/TURN for NAT traversal | Medium |
| Low | Recording to disk | Large |
| Low | Kubernetes / distributed nodes | Large |
