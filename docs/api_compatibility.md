# AI Provider API 兼容性分析报告

> 基于源码 `src-tauri/src/providers/` 的全面分析，涵盖 API 格式、认证方式、重试机制、超时处理、速率限制和签名逻辑。

---

## 一、总览对比表

| Provider | 模态 | API格式 | OpenAI兼容 | 认证方式 | 重试 | 超时 | 速率限制(429) | 特殊签名 |
|---|---|---|---|---|---|---|---|---|
| **DeepSeek** | Text | OpenAI Chat | ✅ 是 | Bearer Token | ✅ 2次, 线性退避 | 连接10s/读取90s/总180s | 识别但不重试 | 无 |
| **Qwen (通义千问)** | Text | OpenAI Chat (compatible-mode) | ✅ 是 | Bearer Token | ✅ 3次, 线性退避 | 连接10s/总120s | 识别但不重试 | 无 |
| **Zhipu (智谱)** | Text | OpenAI Chat (v4) | ✅ 是 | Bearer Token | ✅ 3次, 线性退避 | 连接10s/总120s | 识别但不重试 | 无 |
| **XfyunSpark (讯飞星火)** | Text | WebSocket 自定义 | ❌ 否 | HMAC-SHA256 鉴权URL | ❌ 无 | ❌ 无 | ❌ 无 | ✅ HMAC-SHA256 |
| **SiliconFlow (硅基流动)** | Image | OpenAI Image | ⚠️ 部分 | Bearer Token | ✅ 3次, 429指数退避 | 连接10s/总180s | ✅ 滑动窗口+指数退避 | 无 |
| **Kling (可灵)** | Video | REST 异步轮询 | ❌ 否 | AccessKey+HMAC签名 | ❌ 无 | 连接10s/总30s, 轮询600s | 识别但无重试 | ✅ HMAC-SHA256 |
| **Hailuo (海螺)** | Image | OpenAI Image (data字段) | ⚠️ 部分 | Bearer Token | ✅ 3次, 线性退避 | 连接10s/总180s | 识别但不重试 | 无 |
| **Jimeng (即梦)** | Image | OpenAI Image (data字段) | ⚠️ 部分 | Bearer Token | ✅ 3次, 线性退避 | 连接10s/总180s | 识别但不重试 | 无 |
| **Vidu** | Video | REST 异步轮询 | ❌ 否 | Bearer Token | ❌ 无 | 连接10s/总30s, 轮询600s | 识别但无重试 | 无 |
| **SkyMusic (天谱乐)** | Music | REST 异步轮询 | ❌ 否 | Bearer Token | ❌ 无 | 连接10s/总30s, 轮询300s | 识别但无重试 | 无 |
| **NeteaseMusic (网易天音)** | Music | REST 异步轮询 | ❌ 否 | Bearer Token | ❌ 无 | 连接10s/总30s, 轮询300s | 识别但无重试 | 无 |
| **VolcengineTTS (火山TTS)** | Voice | REST 自定义JSON | ❌ 否 | appid+token(请求体内) | ❌ 无 | 连接10s/总60s | 识别但无重试 | 无 |
| **EdgeTTS** | Voice | WebSocket SSML | ❌ 否 | 硬编码Token | ❌ 无 | ❌ 无 | ❌ 无 | 无 |
| **Builtin** | 全部 | 本地文件 | N/A | 无需认证 | N/A | N/A | N/A | N/A |

---

## 二、详细分析

### 1. DeepSeek

- **源文件**: `src-tauri/src/providers/deepseek.rs`
- **API 格式**: 完全 OpenAI 兼容，使用 `/v1/chat/completions` 端点
- **默认端点**: `https://api.deepseek.com/v1/chat/completions`
- **请求格式**: 标准 OpenAI `ChatRequest`（model, messages, max_tokens, temperature, stream）
- **响应格式**: 标准 OpenAI `ChatResponse`（choices[].message.content）
- **认证**: `Authorization: Bearer {api_key}` 请求头
- **重试机制**:
  - 最大重试 2 次（`MAX_RETRIES = 2`）
  - 退避策略：线性 `500ms * attempt`
  - 触发条件：`NetworkError`、`Timeout`、5xx 服务器错误
  - **不重试** 429 速率限制
- **超时**: connect=10s, read=90s, total=180s（最完善的三层超时）
- **429 处理**: 识别为 `QuotaExceeded` 错误但不重试
- **流式支持**: ✅ `chat_stream()` 方法返回原始 Response 供 SSE 处理

### 2. Qwen (通义千问)

- **源文件**: `src-tauri/src/providers/qwen.rs`
- **API 格式**: OpenAI 兼容模式，使用阿里云 DashScope 的 compatible-mode 端点
- **默认端点**: `https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions`
- **认证**: `Authorization: Bearer {api_key}`
- **重试机制**: 最大 3 次，线性退避 500ms，同 DeepSeek 逻辑
- **超时**: connect=10s, total=120s（无 read_timeout）
- **429 处理**: 识别为 `QuotaExceeded` 但不重试
- **流式支持**: ✅ `chat_stream()` 方法

### 3. Zhipu (智谱)

- **源文件**: `src-tauri/src/providers/zhipu.rs`
- **API 格式**: OpenAI 兼容，使用智谱 PAAS v4 端点
- **默认端点**: `https://open.bigmodel.cn/api/paas/v4/chat/completions`
- **认证**: `Authorization: Bearer {api_key}`
- **重试机制**: 最大 3 次，线性退避 500ms
- **超时**: connect=10s, total=120s
- **429 处理**: 识别为 `QuotaExceeded` 但不重试
- **流式支持**: ✅ `chat_stream()` 方法

### 4. XfyunSpark (讯飞星火)

- **源文件**: `src-tauri/src/providers/xfyun_spark.rs`
- **API 格式**: **完全自定义**，使用 WebSocket 协议
- **默认端点**: `wss://spark-api.xf-yun.com/v1.1/chat`
- **认证**: **HMAC-SHA256 签名鉴权 URL**
  - 需要 3 个凭证：`appId`、`apiSecret`、`apiKeyReal`
  - 签名流程：
    1. 构造签名原文：`host: {host}\ndate: {date}\nGET {path} HTTP/1.1`
    2. 使用 `apiSecret` 对签名原文做 HMAC-SHA256
    3. Base64 编码签名结果
    4. 构造 authorization：`api_key="{apiKeyReal}", algorithm="hmac-sha256", headers="host date request-line", signature="{signature}"`
    5. Base64 编码整个 authorization
    6. 拼接到 WebSocket URL 参数：`?authorization={}&date={}&host={}`
  - 手动实现了 RFC1123 时间格式和 URL 编码（未使用 chrono/urlencoding 库）
- **请求格式**: 自定义 JSON（header/parameter/payload 三段式）
- **响应格式**: 自定义 JSON（header.code/header.status/payload.choices）
- **重试机制**: ❌ 无
- **超时**: ❌ 无（WebSocket 无超时控制）
- **429 处理**: ❌ 无
- **风险**: WebSocket 连接可能无限挂起；签名逻辑依赖手动时间格式化，可能有时区问题

### 5. SiliconFlow (硅基流动)

- **源文件**: `src-tauri/src/providers/siliconflow.rs`
- **API 格式**: OpenAI Image Generation 格式（部分兼容）
- **默认端点**: `https://api.siliconflow.cn/v1/images/generations`
- **认证**: `Authorization: Bearer {api_key}`
- **请求格式**: `{model, prompt, negative_prompt, image_size, num_inference_steps, seed}`
- **响应格式**: `{images: [{url: "..."}]}`（注意：与 OpenAI DALL-E 的 `data` 字段不同）
- **重试机制**: ✅ 最大 3 次
  - 普通错误：线性退避 500ms
  - 429 错误：**指数退避** `10s * 2^attempt`（最完善）
- **超时**: connect=10s, total=180s
- **速率限制**: ✅ **最完善**
  - 内置滑动窗口速率限制器（`RateLimiter`）
  - 默认 5 次/分钟
  - 请求前检查等待时间
  - 读取 `Retry-After` 响应头
  - 成功请求记录到滑动窗口
- **429 处理**: ✅ 完整处理（指数退避 + Retry-After 头）

### 6. Kling (可灵)

- **源文件**: `src-tauri/src/providers/kling.rs`
- **API 格式**: **完全自定义**，REST + 异步轮询
- **默认端点**: `https://api.klingai.com/v1/videos/generations`
- **认证**: **HMAC-SHA256 签名**
  - 需要 2 个凭证：`access_key`、`secret_key`
  - 签名流程：
    1. 构造签名原文：`{METHOD}{path}{timestamp}`
    2. 使用 `secret_key` 对签名原文做 HMAC-SHA256
    3. 输出 hex 编码（注意：与讯飞的 Base64 编码不同）
  - 通过请求头传递：`X-Access-Key`、`X-Signature`、`X-Timestamp`
- **请求格式**: 自定义 `{model, prompt, negative_prompt, duration, aspect_ratio, seed}`
- **响应格式**: 自定义 `{code, message, data: {task_id, task_status, task_result}}`
- **异步模式**: 提交任务 → 轮询状态（10s 间隔，最长 600s）
- **重试机制**: ❌ 无
- **超时**: connect=10s, total=30s（API请求），轮询最长 600s，下载 120s
- **429 处理**: 识别为 `QuotaExceeded` 但不重试
- **风险**: 无重试机制，网络波动直接失败

### 7. Hailuo (海螺)

- **源文件**: `src-tauri/src/providers/hailuo.rs`
- **API 格式**: 类 OpenAI Image 格式，但响应使用 `data` 字段
- **默认端点**: `https://api.hailuo.ai/v1/images/generations`
- **认证**: `Authorization: Bearer {api_key}`
- **请求格式**: `{model, prompt, negative_prompt, image_size}`
- **响应格式**: `{data: [{url: "..."}]}`（与 OpenAI DALL-E 一致）
- **重试机制**: ✅ 最大 3 次，线性退避 500ms
- **超时**: connect=10s, total=180s
- **429 处理**: 识别为 `QuotaExceeded` 但不重试

### 8. Jimeng (即梦)

- **源文件**: `src-tauri/src/providers/jimeng.rs`
- **API 格式**: 与 Hailuo 完全一致的 Image 格式
- **默认端点**: `https://api.jimeng.ai/v1/images/generations`
- **认证**: `Authorization: Bearer {api_key}`
- **重试机制**: ✅ 最大 3 次，线性退避 500ms
- **超时**: connect=10s, total=180s
- **429 处理**: 识别为 `QuotaExceeded` 但不重试
- **注意**: 代码结构与 Hailuo 几乎完全相同，存在大量重复

### 9. Vidu

- **源文件**: `src-tauri/src/providers/vidu.rs`
- **API 格式**: REST + 异步轮询（与 Kling 类似但更简单）
- **默认端点**: `https://api.vidu.ai/v1/videos/generations`
- **认证**: `Authorization: Bearer {api_key}`
- **请求格式**: `{model, prompt, negative_prompt, duration, aspect_ratio}`
- **响应格式**: `{task_id, status, url}`
- **异步模式**: 提交任务 → 轮询状态（10s 间隔，最长 600s）
- **重试机制**: ❌ 无
- **超时**: connect=10s, total=30s，轮询最长 600s，下载 120s
- **429 处理**: 识别为 `QuotaExceeded` 但不重试

### 10. SkyMusic (天谱乐)

- **源文件**: `src-tauri/src/providers/skymusic.rs`
- **API 格式**: REST + 异步轮询
- **默认端点**: `https://api.tiangong.cn/v1/music/generations`
- **认证**: `Authorization: Bearer {api_key}`
- **请求格式**: `{model, prompt, duration, style}`
- **异步模式**: 提交任务 → 轮询状态（5s 间隔，最长 300s）
- **重试机制**: ❌ 无
- **超时**: connect=10s, total=30s
- **429 处理**: 识别但无重试
- **连通性检测**: 使用 GET 请求到端点（404 也视为可达）

### 11. NeteaseMusic (网易天音)

- **源文件**: `src-tauri/src/providers/netease_music.rs`
- **API 格式**: REST + 异步轮询（与 SkyMusic 几乎完全一致）
- **默认端点**: `https://music.163.com/api/music/generate`
- **认证**: `Authorization: Bearer {api_key}`
- **重试机制**: ❌ 无
- **超时**: connect=10s, total=30s
- **429 处理**: 识别但无重试
- **注意**: 代码结构与 SkyMusic 几乎完全相同，存在大量重复

### 12. VolcengineTTS (火山TTS)

- **源文件**: `src-tauri/src/providers/volcengine_tts.rs`
- **API 格式**: **完全自定义** REST JSON
- **默认端点**: `https://openspeech.bytedance.com/api/v1/tts`
- **认证**: **请求体内嵌认证**（非 HTTP 头）
  - 需要 2 个凭证：`appid`、`access_token`
  - 认证信息放在请求体 `app` 字段：`{appid, token}`
- **请求格式**: 自定义四段式 `{app, user, audio, request}`
  - `app`: `{appid, token}`
  - `user`: `{uid}`
  - `audio`: `{voice_type, encoding, speed_ratio}`
  - `request`: `{reqid(uuid), text, text_type, operation}`
- **响应格式**: `{code, message, data(base64音频)}`
- **重试机制**: ❌ 无
- **超时**: connect=10s, total=60s
- **429 处理**: 识别但无重试
- **风险**: 认证信息在请求体内，可能被日志泄露

### 13. EdgeTTS

- **源文件**: `src-tauri/src/providers/edge_tts.rs`
- **API 格式**: **WebSocket + SSML**（微软 Edge 语音合成逆向接口）
- **端点**: `wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken=...&ConnectionId=...`
- **认证**: **硬编码 Token**（`6A5AA1D4EAFF4E9FB37E23D68491D6F4`）
- **协议流程**:
  1. WebSocket 连接（带硬编码 Token + 随机 ConnectionId）
  2. 发送配置消息（JSON 格式，指定音频输出格式）
  3. 发送 SSML 消息（XML 格式，包含语音和文本）
  4. 接收二进制音频数据（header + `\r\n\r\n` + 音频数据）
  5. 检测 `Path:turn.end` 结束标记
- **重试机制**: ❌ 无
- **超时**: ❌ 无
- **429 处理**: ❌ 无
- **风险**:
  - 硬编码 Token 可能随时失效
  - 逆向接口，无官方保障
  - WebSocket 无超时控制

### 14. Builtin

- **源文件**: `src-tauri/src/providers/builtin.rs`
- **类型**: 本地资源提供者，不调用任何 API
- **认证**: 无需
- **重试/超时/速率限制**: 不适用
- **功能**: 从内置资源库和用户导入资源中查找匹配文件

---

## 三、分类总结

### 3.1 真正 OpenAI 兼容的 Provider

| Provider | 兼容程度 | 说明 |
|---|---|---|
| DeepSeek | ✅ 完全 | 标准 `/v1/chat/completions`，请求/响应格式完全一致 |
| Qwen | ✅ 完全 | 使用 DashScope compatible-mode，完全兼容 |
| Zhipu | ✅ 完全 | 使用 PAAS v4 端点，完全兼容 |

### 3.2 部分 OpenAI 兼容的 Provider（Image Generation 变体）

| Provider | 差异点 |
|---|---|
| SiliconFlow | 响应用 `images` 字段而非 OpenAI 的 `data` 字段 |
| Hailuo | 响应用 `data` 字段，与 DALL-E 一致 |
| Jimeng | 同 Hailuo |

### 3.3 完全自定义 API 的 Provider

| Provider | 协议 | 特殊之处 |
|---|---|---|
| XfyunSpark | WebSocket | HMAC-SHA256 鉴权 URL，自定义三段式请求 |
| Kling | REST + 轮询 | HMAC-SHA256 签名头，hex 编码 |
| Vidu | REST + 轮询 | Bearer Token，简单异步轮询 |
| SkyMusic | REST + 轮询 | Bearer Token，简单异步轮询 |
| NeteaseMusic | REST + 轮询 | Bearer Token，简单异步轮询 |
| VolcengineTTS | REST | 请求体内嵌认证，base64 音频响应 |
| EdgeTTS | WebSocket | 硬编码 Token，SSML 协议 |

---

## 四、缺失功能分析

### 4.1 缺少重试机制的 Provider

| Provider | 风险等级 | 说明 |
|---|---|---|
| XfyunSpark | 🔴 高 | WebSocket 无重试，连接断开直接失败 |
| Kling | 🔴 高 | 视频生成耗时长，网络波动概率大 |
| Vidu | 🟡 中 | 同 Kling，但无签名依赖 |
| SkyMusic | 🟡 中 | 音乐生成需要轮询，网络中断导致任务丢失 |
| NeteaseMusic | 🟡 中 | 同 SkyMusic |
| VolcengineTTS | 🟡 中 | TTS 请求较短，风险相对较低 |
| EdgeTTS | 🟡 中 | WebSocket 无重试 |

### 4.2 缺少超时控制的 Provider

| Provider | 风险等级 | 说明 |
|---|---|---|
| XfyunSpark | 🔴 高 | WebSocket 无任何超时，可能无限挂起 |
| EdgeTTS | 🔴 高 | WebSocket 无任何超时，可能无限挂起 |

### 4.3 429 速率限制处理不足的 Provider

| Provider | 现状 | 建议 |
|---|---|---|
| DeepSeek | 识别但不重试 | 应增加 429 重试（指数退避） |
| Qwen | 识别但不重试 | 同上 |
| Zhipu | 识别但不重试 | 同上 |
| Hailuo | 识别但不重试 | 同上 |
| Jimeng | 识别但不重试 | 同上 |
| Kling | 识别但不重试 | 视频生成成本高，429 重试更关键 |
| Vidu | 识别但不重试 | 同上 |
| SkyMusic | 识别但不重试 | 同上 |
| NeteaseMusic | 识别但不重试 | 同上 |
| VolcengineTTS | 识别但不重试 | 同上 |
| **SiliconFlow** | ✅ 完整处理 | 唯一正确实现 429 重试的 Provider |

### 4.4 特殊认证可能出问题的 Provider

| Provider | 风险 | 说明 |
|---|---|---|
| XfyunSpark | 🔴 高 | HMAC 签名依赖手动 RFC1123 时间格式化，未使用 chrono 库，可能有时区/格式问题 |
| Kling | 🟡 中 | HMAC 签名使用 hex 编码，URL 路径提取逻辑较脆弱 |
| VolcengineTTS | 🟡 中 | 认证信息在请求体内，日志可能泄露 token |
| EdgeTTS | 🔴 高 | 硬编码 Token，微软可能随时更换或封禁 |

---

## 五、标准化建议

### 5.1 提取公共 HTTP Client 基础设施

当前每个 Provider 都独立创建 `reqwest::Client` 和实现错误处理。建议：

```
trait BaseHttpClient {
    fn create_client(config) -> Client;  // 统一超时配置
    fn handle_error_status(status, body) -> ProviderError;  // 统一错误映射
    fn retry_request(fn, max_retries, backoff) -> Result;  // 统一重试逻辑
}
```

### 5.2 统一重试策略

- **所有 Provider 应支持重试**，至少对 `NetworkError`、`Timeout`、5xx 错误
- **429 应自动重试**，采用指数退避（参考 SiliconFlow 实现）
- 建议默认 `MAX_RETRIES = 3`，退避策略 `base_delay * 2^attempt`（如 1s, 2s, 4s）

### 5.3 统一超时配置

| 场景 | 建议超时 |
|---|---|
| HTTP 连接 | 10s（当前统一） |
| 文本生成 | 120s |
| 图片生成 | 180s |
| 视频/音乐 API 请求 | 30s |
| 异步轮询总时长 | 600s（视频）/ 300s（音乐） |
| WebSocket | 需要添加读超时（建议 120s） |

### 5.4 统一速率限制处理

建议为所有 Provider 添加类似 SiliconFlow 的 `RateLimiter`：
- 滑动窗口速率限制器
- 读取 `Retry-After` 响应头
- 429 自动指数退避重试

### 5.5 消除代码重复

以下 Provider 对存在大量代码重复，应提取公共模块：

| 重复组 | Provider | 重复内容 |
|---|---|---|
| Text Chat | DeepSeek, Qwen, Zhipu | ChatRequest/ChatResponse/重试/错误处理/连通性检测 |
| Image Gen | Hailuo, Jimeng | ImageGenerationRequest/Response/下载/错误处理 |
| Async Poll | Kling, Vidu | 轮询逻辑/下载逻辑/错误处理 |
| Music Gen | SkyMusic, NeteaseMusic | 几乎完全相同的代码结构 |

### 5.6 修复高风险问题

1. **XfyunSpark**: 添加 WebSocket 读超时（120s）；使用 `chrono` 库替代手动时间格式化
2. **EdgeTTS**: 添加 WebSocket 读超时；Token 应可配置而非硬编码
3. **Kling**: 添加重试机制（视频生成成本高，网络波动不应直接失败）
4. **VolcengineTTS**: 将认证信息从请求体移到 HTTP 头（如需保持兼容，至少确保日志不打印 token）

### 5.7 Provider Trait 增强

当前 `IAssetProvider` trait 较为简单，建议增加：

```rust
trait IAssetProvider {
    // 现有方法...

    /// 获取推荐的超时配置
    fn timeout_config(&self) -> TimeoutConfig;

    /// 获取速率限制配置
    fn rate_limit_config(&self) -> RateLimitConfig;

    /// 是否支持重试
    fn supports_retry(&self) -> bool { true }
}
```

---

## 六、OpenAI 兼容性矩阵

| 特性 | DeepSeek | Qwen | Zhipu | SiliconFlow | Hailuo | Jimeng |
|---|---|---|---|---|---|---|
| `/v1/chat/completions` | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| `/v1/images/generations` | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ |
| Bearer Token 认证 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 标准 ChatRequest | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| 标准 ChatResponse | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| SSE 流式 | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| 标准 Error 格式 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

*报告生成时间: 2026-06-05*
*分析基于源码版本: src-tauri/src/providers/*
