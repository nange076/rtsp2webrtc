# RTSP → WebRTC Gateway

低延迟 RTSP 转 WebRTC 流媒体网关。将 IP 摄像头 RTSP 流实时转发为浏览器原生 WebRTC 播放，无需插件、无需转码。

> English version: [README_EN.md](README_EN.md)

---

## 核心特性

- **零转码 RTP 转发** — 不解码帧，不重编码，最低 CPU 开销和延迟
- **多客户端共享** — 一路 RTSP 拉流，N 个浏览器同时观看
- **懒加载 + 空闲自动停止** — 有观众才拉流，所有人离开后 30 秒自动断开摄像头
- **动态流创建** — 客户端通过 API 提交 RTSP URL，无需服务端预配置
- **断线重连 + 保活** — 指数退避自动重连，55 秒超时保活防止摄像头断开会话
- **API 鉴权/限流** — 可选的 API key + 创建频率限制
- **监控就绪** — `/health`、`/metrics` (Prometheus)、结构化日志

---

## 快速开始

### 1. 配置文件

创建 `config.toml`（或被 `CONFIG_PATH` 环境变量覆盖）：

```toml
[server]
bind_addr = "0.0.0.0:3000"
# api_key = "changeme"

[[streams]]
id = "camera-1"
name = "前门"
url = "rtsp://admin:password@192.168.1.100:554/stream"

[limits]
max_peers = 50
max_per_stream = 20
create_per_min = 10

[logging]
format = "text"    # "text" 或 "json"

[cors]
allowed_origins = ["*"]
```

### 2. 启动服务

```bash
cargo run --release
# 或设置日志级别
RUST_LOG=info cargo run --release
```

### 3. 打开浏览器

直接打开 `web/index.html`，确认网关地址为 `http://localhost:3000`，点击"创建并播放"或点击流列表中的"播放"按钮。

---

## 使用方式

### 方式一：配置流（服务端预设）

`config.toml` 中预先配置 RTSP 流，客户端直接通过流 ID 连接：

```
ws://localhost:3000/ws?stream=camera-1
```

### 方式二：动态流（客户端按需创建）

客户端提交 RTSP URL，服务端自动拉起转流任务：

```bash
curl -X POST http://localhost:3000/api/streams \
  -H "Content-Type: application/json" \
  -d '{"url":"rtsp://admin:pass@192.168.1.100:554/stream"}'
```

返回 `{"stream_id":"uuid"}`，然后浏览器连接：

```
ws://localhost:3000/ws?stream=uuid
```

---

## REST API

所有响应均为 `application/json`。

### 健康检查

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

### 流列表

```http
GET /api/streams
```

```json
{
  "streams": [
    {
      "id": "camera-1",
      "name": "前门",
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

### 创建动态流

```http
POST /api/streams
Authorization: Bearer changeme    # 如果配置了 api_key
Content-Type: application/json

{ "url": "rtsp://admin:pass@192.168.1.100:554/stream" }
```

```json
{ "stream_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890" }
```

### 流详情

```http
GET /api/streams/camera-1
```

### 删除动态流

```http
DELETE /api/streams/a1b2c3d4-e5f6-7890-abcd-ef1234567890
Authorization: Bearer changeme
```

→ `204 No Content`

### Prometheus 指标

```http
GET /metrics
```

---

## WebSocket 信令协议

### 客户端 → 服务端

```json
{ "type": "request_stream" }
{ "type": "sdp_answer", "sdp": "v=0\r\n..." }
{ "type": "ice_candidate", "candidate": "...", "sdpMid": "0" }
```

### 服务端 → 客户端

```json
{ "type": "connected" }
{ "type": "sdp_offer", "sdp": "v=0\r\n..." }
{ "type": "ice_candidate", "candidate": {...}, "sdpMid": "0" }
{ "type": "error", "message": "..." }
```

### 连接参数

| 参数 | 必填 | 说明 |
|------|:--:|------|
| `stream` | 否 | 流 ID，不填则使用配置中第一个流 |
| `key` | 否 | API key（若服务端配置了 `api_key` 则必填） |

示例：`ws://localhost:3000/ws?stream=camera-1&key=changeme`

---

## 架构

```
RTSP Camera ──→ RtspPuller ──→ RtpRelay (broadcast) ──┬──→ WebRTC Peer 1 ──→ Browser 1
                                           ├──→ WebRTC Peer 2 ──→ Browser 2
                                           └──→ WebRTC Peer N ──→ Browser N
```

- **RtspPuller** — retina 库连接摄像头，接收原始 RTP 包
- **RtpRelay** — tokio broadcast channel 实现一路 RTP → N 个 WebRTC track
- **WebRtcPeer** — webrtc-rs 管理 PeerConnection，创建 H264 video track
- **StreamManager** — 流注册中心，管理生命周期、订阅者计数、连接限制
- **Signaling** — WebSocket SDP/ICE 交换

---

## 配置参考

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `server.bind_addr` | string | `0.0.0.0:3000` | HTTP/WS 监听地址 |
| `server.api_key` | string | (空) | API 鉴权密钥，空则不启用 |
| `streams[].id` | string | — | 流唯一标识 |
| `streams[].name` | string | — | 流显示名称 |
| `streams[].url` | string | — | RTSP 地址（含用户名密码） |
| `limits.max_peers` | int | 50 | 全局最大 WebRTC 连接数 |
| `limits.max_per_stream` | int | 20 | 单流最大观众数 |
| `limits.create_per_min` | int | 0 | 每分钟最大创建流次数，0=不限 |
| `cors.allowed_origins` | []string | [] | `["*"]` 全部放行，空=同源限制 |
| `logging.format` | string | text | `"text"` 或 `"json"` |
| `tls.cert` | path | — | TLS 证书路径 |
| `tls.key` | path | — | TLS 私钥路径 |

---

## 开发路线

见 [docs/roadmap.md](docs/roadmap.md)

## 技术栈

Rust · Tokio · Axum · webrtc-rs · retina · WebSocket · H264
