# AGENTS.md

# Project Identity

This project is a low-latency RTSP-to-WebRTC streaming gateway written in Rust.

The system receives RTSP camera streams and exposes them to browser clients through native WebRTC playback.

Primary architecture:

RTSP Camera
→ Rust Gateway
→ WebRTC
→ Browser

---

# Core Architectural Principles

## 1. This is NOT a transcoding platform

The system is primarily:

- RTP relay
- RTP remux
- WebRTC gateway
- Session manager
- Signaling service
- Real-time streaming infrastructure

The system is NOT:

- a video editor
- a media processing pipeline
- a CPU-heavy FFmpeg transcoder
- a browser-side RTSP decoder

---

## 2. Avoid transcoding whenever possible

Preferred flow:

RTSP(H264 RTP)
→ RTP relay/remux
→ WebRTC RTP
→ Browser

Avoid:

RTSP
→ decode
→ encode
→ WebRTC

Transcoding should only occur when:

- source codec is incompatible with browser playback
- H265 → H264 conversion is required
- resolution/bitrate adaptation is necessary

Otherwise:

- preserve encoded payloads
- avoid frame decoding
- minimize CPU usage

---

# Browser Playback Requirements

Browser playback MUST use:

- RTCPeerConnection
- MediaStream
- native WebRTC APIs

Do NOT implement:

- browser RTSP playback
- raw TCP browser streaming
- MQTT video transport
- custom RTP browser parsers

---

# WebSocket Usage

WebSocket is ONLY for signaling.

Allowed responsibilities:

- SDP exchange
- ICE candidate exchange
- session lifecycle events
- stream control messages

Media transport MUST occur through:

- SRTP/WebRTC media transport

---

# Preferred Technology Stack

Preferred Rust stack:

- tokio
- axum
- webrtc-rs
- retina
- tokio-tungstenite
- tracing
- anyhow
- thiserror

Avoid unnecessary FFmpeg shell pipelines.

FFmpeg/GStreamer should only be fallback tools.

---

# Stream Fanout Requirement

DO NOT create:

1 browser client
= 1 RTSP pull

Correct design:

1 RTSP source
→ shared stream session
→ multiple WebRTC consumers

Required capabilities:

- stream registry
- subscriber management
- shared RTSP sessions
- automatic cleanup
- idle timeout handling

---

# Latency Targets

Target latency:

- preferred: 100ms ~ 500ms
- acceptable: <1s

Avoid:

- HLS
- DASH
- segmented streaming
- large buffering

Prefer:

- direct RTP forwarding
- low buffering
- packet pacing
- async streaming

---

# Networking Constraints

Initial development target:

- LAN deployment
- local network testing
- simplified ICE handling

Later phases may include:

- STUN
- TURN
- NAT traversal
- public network deployment

---

# Codec Rules

Assume browser support:

- H264
- VP8

Do NOT assume support:

- H265

If RTSP source is H265:

- transcode to H264
OR
- reject unsupported stream

---

# Required System Components

- Signaling Server
- Stream Manager
- RTSP Puller
- RTP Relay
- WebRTC Peer Manager
- Subscriber Registry
- Session Lifecycle Controller

---

# Rust Engineering Standards

All code should:

- use async/await
- avoid blocking operations
- minimize memory copies
- avoid unnecessary allocations
- avoid unnecessary frame decoding
- support graceful shutdown
- use structured logging
- use typed error handling

---

# Forbidden Anti-Patterns

DO NOT:

- use MQTT for video transport
- use browser-side RTSP parsing
- fully transcode by default
- create one FFmpeg process per browser client
- decode frames unnecessarily
- implement HLS as primary playback path
- perform CPU-heavy image processing in relay path

---

# Development Roadmap

## Phase 1

- single RTSP stream
- single browser playback
- LAN only
- basic signaling

## Phase 2

- multi-client fanout
- shared stream sessions
- reconnect handling
- stream lifecycle management

## Phase 3

- authentication
- STUN/TURN
- public network support
- metrics and observability

## Phase 4

- recording support
- distributed scaling
- GPU transcoding fallback
- cluster deployment