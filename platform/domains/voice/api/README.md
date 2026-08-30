# mox-voice-api · 语音领域 trait 契约层

MOX 平台语音领域的 API 契约 crate，定义 ASR（语音识别）、TTS（语音合成）、意图识别、DSP（数字信号处理）、语音会话管理等核心能力的 trait 接口与数据结构，为各服务实现提供统一抽象。

## 功能特性

- **ASR 语音识别**：支持批量识别与流式识别，返回文本、置信度、语言、分段信息及说话人标签
- **TTS 语音合成**：支持多音色、多语言、语速/音调调节，输出指定格式音频数据
- **语音意图识别**：支持从文本或直接从音频识别用户意图，返回意图、槽位与置信度
- **音频 DSP 处理**：重采样、滤波（低通/高通/带通/带阻）、归一化、降噪、VAD 端点检测
- **语音会话管理**：会话的创建、查询、结束与列举，支持租户隔离与会话元数据
- **统一错误类型**：`VoiceApiError` 涵盖 ASR/TTS/Intent/DSP/Internal 五类错误

## 架构定位

本 crate 属于 MOX 平台 **voice 领域 API 层**，位于：

```
platform/domains/voice/
├── api/                    ← 本 crate（trait 契约 / DTO）
├── core/                   ← 核心领域逻辑（DSP 等）
└── svc/                    ← 服务实现（ASR / Intent / Operator 等）
```

- 向上：供各 voice svc crate 实现对应 trait
- 向下：供上层应用（BallWidget、voice_proxy 等）依赖调用
- 横向：作为 voice 领域各子模块之间的解耦契约

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-voice-api = { path = "../api" }
```

### 基本用法

实现 `SpeechRecognizer` trait 示例：

```rust
use mox_voice_api::{SpeechRecognizer, AsrResult, VoiceApiResult, VoiceApiError};
use async_trait::async_trait;

struct MyAsrEngine;

#[async_trait]
impl SpeechRecognizer for MyAsrEngine {
    async fn recognize(&self, audio: &[u8], format: &str) -> VoiceApiResult<AsrResult> {
        // 调用具体 ASR 引擎...
        Err(VoiceApiError::Asr("not implemented".into()))
    }

    async fn recognize_stream(
        &self,
        audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> VoiceApiResult<AsrResult> {
        Err(VoiceApiError::Asr("not implemented".into()))
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["wav".into(), "pcm".into()]
    }
}
```

## 核心模块 / 类型

### 错误与结果
- `VoiceApiError` — 语音领域统一错误枚举（Asr / Tts / Intent / Dsp / Internal）
- `VoiceApiResult<T>` — 结果类型别名

### ASR 模块
- `AsrResult` — 语音识别结果（文本、置信度、语言、分段、时长）
- `AsrSegment` — 识别分段（起止时间、文本、说话人）
- `SpeechRecognizer` — 语音识别器 trait（recognize / recognize_stream / supported_formats）

### TTS 模块
- `TtsRequest` — 合成请求（文本、音色、语言、语速、音调、格式）
- `TtsResult` — 合成结果（音频数据、格式、时长、采样率）
- `SpeechSynthesizer` — 语音合成器 trait（synthesize / list_voices）

### 意图识别模块
- `VoiceIntentResult` — 意图识别结果（意图名、置信度、槽位、原始文本）
- `VoiceIntentRecognizer` — 意图识别器 trait（recognize / recognize_from_audio）

### DSP 模块
- `DspFilterType` — 滤波器类型枚举（LowPass / HighPass / BandPass / BandStop）
- `AudioProcessor` — 音频处理器 trait（resample / filter / normalize / noise_reduce / vad）

### 会话管理模块
- `VoiceSession` — 语音会话结构体（ID、租户、用户、状态、创建时间、元数据）
- `VoiceSessionManager` — 会话管理器 trait（create / get / end / list）

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>
