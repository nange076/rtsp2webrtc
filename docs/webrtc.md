# WebRTC Design

# Purpose

WebRTC is the browser-native playback protocol.

The browser should receive media using:

- RTCPeerConnection
- MediaStream
- SRTP

---

# Signaling

Signaling transport:

- WebSocket

Responsibilities:

- SDP offer/answer
- ICE candidates
- connection lifecycle

Media is NOT transported over WebSocket.

---

# ICE Strategy

Initial deployment:

- LAN-only
- host ICE candidates

Future deployment:

- STUN
- TURN
- NAT traversal

---

# Track Design

Preferred:

- TrackLocalStaticRTP

Alternative:

- TrackLocalStaticSample

Track should receive:

- RTP packets from RTSP source

Avoid unnecessary transcoding.

---

# Browser Compatibility

Assume support:

- H264
- VP8

Do NOT assume:

- H265

---

# Session Lifecycle

Browser connects:

1. request stream
2. receive SDP offer/answer
3. ICE negotiation
4. media starts

Browser disconnects:

1. remove subscriber
2. cleanup peer
3. release resources

If no subscribers remain:

- stop idle stream
- cleanup RTSP session