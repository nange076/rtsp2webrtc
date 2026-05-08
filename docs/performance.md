# Performance Targets

# Latency

Preferred:

100ms ~ 500ms

Maximum acceptable:

<1 second

---

# CPU Usage

Primary objective:

Avoid transcoding.

Preferred workload:

- RTP forwarding
- packet remux
- SRTP transport

Avoid:

- decode/encode loops
- frame processing
- unnecessary copies

---

# Memory

Goals:

- bounded buffering
- stream reuse
- zero/minimal copy
- avoid unbounded queues

---

# Scalability

Target architecture:

1 RTSP pull
→ multiple WebRTC subscribers

The system should scale through:

- stream fanout
- async runtime
- efficient RTP forwarding

NOT through:

- duplicated media pipelines

---

# Observability

Required metrics:

- active streams
- active subscribers
- RTP throughput
- packet loss
- ICE state
- connection duration
- RTSP reconnect count

Preferred stack:

- tracing
- metrics
- prometheus