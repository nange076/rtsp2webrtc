# Engineering Constraints

# Forbidden Architectures

DO NOT implement:

- browser RTSP playback
- MQTT video transport
- HLS as primary real-time path
- one RTSP pull per browser client
- mandatory transcoding
- CPU-heavy relay pipelines

---

# Forbidden Assumptions

Do NOT assume:

- browser H265 support
- stable public IP connectivity
- infinite camera connection capacity

---

# FFmpeg Usage Rules

FFmpeg is allowed ONLY for:

- codec conversion
- unsupported stream fallback
- debugging
- testing

FFmpeg should NOT become:

- the primary architecture
- the default relay path

---

# Rust Runtime Rules

All services should:

- use Tokio async runtime
- avoid blocking operations
- support graceful shutdown
- support cancellation
- avoid global mutable state

---

# Deployment Constraints

Initial deployment target:

- Linux
- LAN
- Docker-compatible environment

Future deployment:

- Kubernetes
- distributed relay nodes
- TURN infrastructure