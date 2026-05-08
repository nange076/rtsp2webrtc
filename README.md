# RTSP → WebRTC Gateway

A low-latency RTSP-to-WebRTC streaming gateway written in Rust.

The system receives RTSP camera streams and exposes them to browser clients using native WebRTC playback.

---

# Goals

- low latency streaming
- browser-native playback
- minimal transcoding
- efficient RTP relay
- scalable multi-client fanout
- async Rust architecture

---

# Architecture

RTSP Camera
→ Rust Gateway
→ WebRTC
→ Browser

---

# Core Technologies

- Rust
- Tokio
- Axum
- webrtc-rs
- retina
- WebSocket signaling

---

# Development Phases

## Phase 1

- single stream playback
- LAN support
- basic signaling

## Phase 2

- multi-client fanout
- shared RTSP sessions
- reconnect handling

## Phase 3

- STUN/TURN
- authentication
- public network support

## Phase 4

- metrics
- recording
- distributed deployment
- GPU transcoding fallback

---

# Design Philosophy

The system prioritizes:

- RTP forwarding
- low latency
- minimal transcoding
- WebRTC-native playback
- efficient async concurrency

The system is NOT intended to be:

- a video editing platform
- a heavy transcoding service
- a browser-side RTSP decoder