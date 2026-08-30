# mox-voice-dsp-core · 语音 DSP 核心引擎

TTS 语音数字信号处理（DSP）权威单源实现，提供重采样、时域变速、响度归一化与 PCM WAV 编码等核心能力，与 Python `cosyvoice2._resample_linear` 逐位等价。

## 功能特性

- **线性插值重采样**：O(n) 时间复杂度，与 Python cosyvoice2 参考实现逐位等价
- **SOLA 时域变速**：基于同步重叠相加算法（frame=20ms / overlap=10ms），支持 ±30% 小幅度变速场景
- **响度归一化 + 软限幅**：目标 -18dBFS，tanh knee ≥ 0.95 进入压缩，避免削波失真
- **PCM WAV 编码**：16-bit little-endian，44 字节标准 RIFF/WAVE header
- **SIMD 加速**：使用 `wide::f32x4` 加速响度/软限幅的逐样本循环，提升 4 倍吞吐
- **rayon 并行**：批量音频处理或大音频分块可按 CHUNK 并行处理

## 架构定位

本 crate 属于 MOX 平台 **voice 领域核心层**，位于：

```
platform/domains/voice/
├── api/                    ← trait 契约层
├── core/
│   └── mox-voice-dsp-core/ ← 本 crate（DSP 核心算法）
└── svc/                    ← 服务实现层
```

- 向上：被 voice svc 层（如 TTS 服务、ASR 前置处理）调用
- 向下：纯算法实现，无 I/O 依赖，可独立测试与 benchmark
- 定位：语音领域 DSP 算法的权威 Rust 实现，替代 Python 版本以获得性能提升

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-voice-dsp-core = { path = "../core/mox-voice-dsp-core" }
```

### 基本用法

```rust
use mox_voice_dsp_core::{
    resample_linear,
    time_stretch_sola,
    apply_limiter_and_loudness,
    encode_wav_pcm16,
    SolaOptions,
    LimiterOptions,
    WavSpec,
};

// 1. 重采样：从 22050Hz 到 16000Hz
let audio: Vec<f32> = /* 输入音频 */;
let resampled = resample_linear(&audio, 22050, 16000);

// 2. 时域变速：减速到 0.9 倍
let stretched = time_stretch_sola(
    &resampled,
    16000,
    0.9,
    SolaOptions::default(),
);

// 3. 响度归一化 + 软限幅
let normalized = apply_limiter_and_loudness(
    &stretched,
    LimiterOptions::default(),
);

// 4. 编码为 16-bit PCM WAV
let wav_bytes = encode_wav_pcm16(
    &normalized,
    WavSpec { sample_rate: 16000, channels: 1 },
);
```

## 核心模块 / 类型

### `resample` 模块
- `resample_linear(input, from_rate, to_rate) -> Vec<f32>` — 线性插值重采样

### `sola` 模块
- `time_stretch_sola(input, sample_rate, speed, options) -> Vec<f32>` — SOLA 时域变速
- `SolaOptions` — SOLA 算法配置（帧长、重叠长度等）

### `loudness` 模块
- `apply_limiter_and_loudness(input, options) -> Vec<f32>` — 响度归一化 + 软限幅
- `LimiterOptions` — 限幅器配置（目标响度、knee 阈值等）

### `wav` 模块
- `encode_wav_pcm16(samples, spec) -> Vec<u8>` — 16-bit PCM WAV 编码
- `WavSpec` — WAV 规格（采样率、声道数）

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>
