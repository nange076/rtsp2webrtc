# Bug Fix 清单

## ✅ 已修复

---

### BUG-001 · 部分 Chrome 浏览器无法播放（`invalid named curve`）

**状态**：✅ 已修复（commit `64de171`）

**现象**

Chrome 116+ 打开页面后，日志出现：

```
WARN webrtc::peer_connection::peer_connection_internal: Failed to start manager dtls: invalid named curve
INFO rtsp2webRTC::webrtc_peer: WebRTC state: Failed
```

视频始终黑屏，WebRTC 连接状态变为 `failed`。Edge 浏览器可正常播放。

**根因**

Chrome 116 起在 DTLS ClientHello 的 `supported_groups` 扩展中，将后量子混合
密钥协商算法 **X25519KYBER768Draft00**（`0x6399`）放在列表首位。

`webrtc-dtls 0.11` 的 `flight0.rs` 盲目取 `elliptic_curves[0]`，该值经
`NamedCurve::from(0x6399)` 映射为 `NamedCurve::Unsupported`，随后
`generate_keypair()` 返回 `ErrInvalidNamedCurve`，DTLS 握手立即失败。

Edge 的同版本 Chromium 内核默认把 `X25519(0x001d)` 排在首位，所以不受影响。

**修复方案**

在 `patches/webrtc-dtls/src/flight/flight0.rs` 中，将盲取首个曲线的逻辑改为
遍历客户端曲线列表，选出第一个服务端**实际支持**的曲线（X25519 / P-256 / P-384）：

```rust
// 修复前
state.named_curve = e.elliptic_curves[0];

// 修复后
state.named_curve = e
    .elliptic_curves
    .iter()
    .copied()
    .find(|c| !matches!(c, NamedCurve::Unsupported))
    .unwrap_or(e.elliptic_curves[0]);
```

通过 `Cargo.toml` 的 `[patch.crates-io]` 指向本地修改版：

```toml
[patch.crates-io]
webrtc-dtls = { path = "patches/webrtc-dtls" }
```

---

### BUG-002 · 本机 Chrome 无法播放（ICE candidate 竞态）

**状态**：✅ 已修复（commit `64de171`）

**现象**

服务端与浏览器在同一台机器上时，Chrome 打开页面后连接状态停在 `connecting`
或直接变为 `failed`，视频黑屏。从另一台机器访问则正常。

**根因**

`ws.onmessage` 声明为 `async`，但 WebSocket 不会等待前一个 handler 完成再触发
下一个，多条消息实际并发执行。

服务端发完 SDP offer 后几乎立刻发出 ICE candidates（本机延迟 ~0 ms）。此时
`sdp_offer` handler 的 `setRemoteDescription → createAnswer → setLocalDescription`
仍在异步执行，`ice_candidate` handler 已并发跑起来，在 remote description 尚未
设置时调用 `addIceCandidate()`，Chrome 抛出 `InvalidStateError`，被原来的
`catch (e) {}` 静默吞掉，server 的所有 ICE candidates 全部丢失。

远端机器因网络延迟（>10 ms），ICE candidates 到达时 offer 早已处理完，
所以没有触发该竞态。

**修复方案**

在 `web/index.html` 中：

1. **消息队列串行化**：将 `ws.onmessage` 改为 promise chain，保证消息按顺序
   串行处理，彻底消除并发竞态：

   ```js
   ws.onmessage = (event) => {
     msgQueue = msgQueue.then(() => handleMessage(event.data)).catch(...);
   };
   ```

2. **ICE candidate 暂存队列**：在 `setRemoteDescription` 完成前到达的
   candidate 先存入 `pendingCandidates[]`，offer 处理完毕后统一 flush，
   双重保障：

   ```js
   remoteDescReady = true;
   for (const c of pendingCandidates) {
     await pc.addIceCandidate(c);
   }
   pendingCandidates = [];
   ```

---

## 🔲 待修复

---

### BUG-003 · Firefox 浏览器无法播放

**状态**：🔲 待修复

**现象**

Firefox 打开页面后视频无法播放。

**已知信息**

- Firefox 对 WebRTC SDP 的格式要求与 Chrome/Edge 存在差异
- Firefox 不支持 `muted` 属性的某些默认行为，可能需要用户手势触发播放
- Firefox 的 ICE candidate 格式、DTLS 握手流程可能与 Chromium 系浏览器不同
- 待收集 Firefox 端具体错误日志后进一步分析

**排查方向**

- [ ] 收集 Firefox 浏览器控制台错误信息及服务端日志
- [ ] 检查 SDP offer 中的 codec/fmtp 格式是否符合 Firefox 要求
- [ ] 检查 Firefox 的 ICE candidate 类型及 mDNS 处理差异
- [ ] 检查 DTLS 证书/密码套件兼容性
- [ ] 检查 `autoplay` 策略，Firefox 对静音视频自动播放的策略与 Chrome 不同

