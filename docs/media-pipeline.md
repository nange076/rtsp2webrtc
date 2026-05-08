# Media Pipeline Design

# Primary Media Flow

RTSP Camera
→ RTP(H264)
→ Rust Relay
→ WebRTC RTP
→ Browser

---

# Core Principle

The gateway should relay encoded RTP payloads whenever possible.

The system should NOT:

- decode frames
- manipulate pixels
- re-encode video

unless absolutely required.

---

# RTP Relay Pipeline

## Input

RTSP stream:

- RTP packets
- H264 payloads
- SPS/PPS NALUs

---

## Internal Processing

Allowed operations:

- timestamp normalization
- payload type rewriting
- sequence adjustment
- SPS/PPS injection
- pacing

Avoid:

- frame decoding
- CPU image transforms
- software encoding

---

## Output

WebRTC-compatible RTP stream.

Transport:

- SRTP
- RTCP
- ICE-managed transport

---

# Codec Compatibility

## Supported

Preferred codecs:

- H264
- VP8

---

## Unsupported

Potentially unsupported:

- H265
- proprietary codecs

---

# H265 Handling

If camera outputs H265:

Options:

1. reject stream
2. fallback transcoding
3. external transcoding service

Do NOT silently transcode by default.

---

# Latency Goals

Preferred:

100ms ~ 500ms

Acceptable:

<1 second

Avoid:

- buffering queues
- segmented streaming
- large GOP delays