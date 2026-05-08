# Architecture Overview

# System Topology

RTSP Camera
    ↓
RTSP Puller
    ↓
RTP Packet Relay
    ↓
WebRTC Track
    ↓
Browser

---

# High-Level Components

## 1. Signaling Server

Responsibilities:

- WebSocket signaling
- SDP offer/answer exchange
- ICE candidate exchange
- session control
- authentication

Protocols:

- HTTP
- WebSocket

Media is NOT transported through signaling.

---

## 2. Stream Manager

Responsibilities:

- RTSP stream lifecycle
- stream registry
- subscriber tracking
- stream reuse
- cleanup handling

Goals:

- avoid duplicate RTSP pulls
- support multiple WebRTC consumers
- reduce camera load

---

## 3. RTSP Puller

Responsibilities:

- connect to RTSP source
- receive RTP packets
- parse codec information
- extract SPS/PPS
- maintain RTSP session

Preferred library:

- retina

---

## 4. RTP Relay Layer

Responsibilities:

- receive RTP packets
- normalize timestamps
- rewrite payload metadata if required
- forward packets to WebRTC tracks

Goals:

- zero/minimal copy
- no transcoding
- low latency

---

## 5. WebRTC Peer Manager

Responsibilities:

- manage PeerConnections
- create media tracks
- handle ICE state
- manage SRTP transport

Preferred library:

- webrtc-rs

---

# Core Design Principles

## Shared Stream Sessions

Correct:

1 RTSP source
→ N WebRTC consumers

Incorrect:

N RTSP pulls
→ N browser consumers

---

## Minimal Processing

Preferred:

RTP packet forwarding

Avoid:

decode → encode pipelines

---

## Async-First Design

Entire pipeline should use:

- Tokio runtime
- async IO
- bounded channels
- backpressure-aware flow

---

# Future Expansion

Potential future modules:

- recording subsystem
- stream persistence
- GPU transcoding
- distributed relay nodes
- metrics service
- TURN relay cluster