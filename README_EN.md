# RTSP → WebRTC Gateway

Low-latency RTSP-to-WebRTC streaming gateway. Relays IP camera RTSP streams to browsers via native WebRTC — no plugins, no transcoding.

> 中文版: [README.md](README.md)

---

## Key Features

- **Zero-transcode RTP relay** — No frame decoding, no re-encoding. Minimal CPU and latency.
- **Multi-client fanout** — One RTSP pull → N concurrent browser viewers
- **Lazy start + idle auto-stop** — Only pulls from camera when someone is watching; stops 30s after last viewer leaves
- **Dynamic streams** — Clients submit RTSP URLs via API; no server-side preconfiguration needed
- **Reconnect + keepalive** — Exponential backoff auto-reconnect; 55s keepalive timeout prevents camera session expiry
- **Auth & rate limiting** — Optional API key + configurable creation rate limits
- **Monitoring ready** — `/health`, `/metrics` (Prometheus), structured logging

---

## Quick Start

### 1. Configuration

Create `config.toml` (or override via `CONFIG_PATH` env var):

```toml
[server]
bind_addr = "0.0.0.0:3000"
# api_key = "changeme"

[[streams]]
id = "camera-1"
name = "Front Door"
url = "rtsp://admin:password@192.168.1.100:554/stream"

[limits]
max_peers = 50
max_per_stream = 20
create_per_min = 10

[logging]
format = "text"    # "text" or "json"

[cors]
allowed_origins = ["*"]
```

### 2. Start Server

```bash
cargo run --release
# Or with log level
RUST_LOG=info cargo run --release
```

### 3. Open Browser

Open `web/index.html`, verify the gateway URL is `http://localhost:3000`, then click "Create & Play" or click "Play" on any listed stream.

---

## Usage

### Option A: Configured Streams (server-side preset)

Pre-configure RTSP streams in `config.toml`. Clients connect by stream ID:

```
ws://localhost:3000/ws?stream=camera-1
```

### Option B: Dynamic Streams (client-driven)

Client submits an RTSP URL; the server spins up the relay on demand:

```bash
curl -X POST http://localhost:3000/api/streams \
  -H "Content-Type: application/json" \
  -d '{"url":"rtsp://admin:pass@192.168.1.100:554/stream"}'
```

Returns `{"stream_id":"uuid"}`. Then connect via WebSocket:

```
ws://localhost:3000/ws?stream=uuid
```

---

## REST API

All responses are `application/json`.

### Health Check

```http
GET /health
```

```json
{
  "status": "ok",
  "uptime_secs": 3600,
  "configured_streams": 2,
  "active_streams": 1,
  "total_peers": 3
}
```

### List Streams

```http
GET /api/streams
```

```json
{
  "streams": [
    {
      "id": "camera-1",
      "name": "Front Door",
      "url": "rtsp://***@192.168.1.100:554",
      "dynamic": false,
      "subscribers": 2,
      "connected": true,
      "codec": "h264",
      "payload_type": 96
    }
  ]
}
```

### Create Dynamic Stream

```http
POST /api/streams
Authorization: Bearer changeme    # if api_key is configured
Content-Type: application/json

{ "url": "rtsp://admin:pass@192.168.1.100:554/stream" }
```

```json
{ "stream_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890" }
```

### Stream Detail

```http
GET /api/streams/camera-1
```

### Delete Dynamic Stream

```http
DELETE /api/streams/a1b2c3d4-e5f6-7890-abcd-ef1234567890
Authorization: Bearer changeme
```

→ `204 No Content`

### Prometheus Metrics

```http
GET /metrics
```

---

## WebSocket Signaling Protocol

### Client → Server

```json
{ "type": "request_stream" }
{ "type": "sdp_answer", "sdp": "v=0\r\n..." }
{ "type": "ice_candidate", "candidate": "...", "sdpMid": "0" }
```

### Server → Client

```json
{ "type": "connected" }
{ "type": "sdp_offer", "sdp": "v=0\r\n..." }
{ "type": "ice_candidate", "candidate": {...}, "sdpMid": "0" }
{ "type": "error", "message": "..." }
```

### Connection Parameters

| Parameter | Required | Description |
|-----------|:--------:|-------------|
| `stream` | No | Stream ID. Defaults to first configured stream. |
| `key` | No | API key (required if `api_key` is configured on the server). |

Example: `ws://localhost:3000/ws?stream=camera-1&key=changeme`

---

## Architecture

```
RTSP Camera ──→ RtspPuller ──→ RtpRelay (broadcast) ──┬──→ WebRTC Peer 1 ──→ Browser 1
                                           ├──→ WebRTC Peer 2 ──→ Browser 2
                                           └──→ WebRTC Peer N ──→ Browser N
```

- **RtspPuller** — retina crate connects to camera, receives raw RTP packets
- **RtpRelay** — tokio broadcast channel: one RTP stream → N WebRTC tracks
- **WebRtcPeer** — webrtc-rs manages PeerConnection, creates H264 video track
- **StreamManager** — stream registry, lifecycle, subscriber counting, connection limits
- **Signaling** — WebSocket SDP/ICE exchange

---

## Configuration Reference

| Field | Type | Default | Description |
|------|------|------|------|
| `server.bind_addr` | string | `0.0.0.0:3000` | HTTP/WS listen address |
| `server.api_key` | string | (empty) | API auth key; disabled if empty |
| `streams[].id` | string | — | Unique stream identifier |
| `streams[].name` | string | — | Display name |
| `streams[].url` | string | — | RTSP URL (with credentials) |
| `limits.max_peers` | int | 50 | Global max WebRTC connections |
| `limits.max_per_stream` | int | 20 | Max viewers per stream |
| `limits.create_per_min` | int | 0 | Max dynamic stream creates/min, 0=unlimited |
| `cors.allowed_origins` | []string | [] | `["*"]` for all, empty=same-origin |
| `logging.format` | string | text | `"text"` or `"json"` |
| `tls.cert` | path | — | TLS certificate path |
| `tls.key` | path | — | TLS private key path |

---

## Roadmap

See [docs/roadmap.md](docs/roadmap.md)

## Tech Stack

Rust · Tokio · Axum · webrtc-rs · retina · WebSocket · H264
