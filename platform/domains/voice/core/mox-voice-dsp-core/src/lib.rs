// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # xiaobai-dsp · TTS 语音 DSP 权威单源
//!
//! 1. 线性插值重采样（O(n)，与 Python cosyvoice2._resample_linear 逐位等价）
//! 2. SOLA 时域变速（frame=20ms / overlap=10ms，±30% 小幅度场景）
//! 3. 响度归一化 + 软限幅（目标 -18dBFS，tanh knee ≥ 0.95 进入压缩）
//! 4. PCM WAV 编码（16-bit little-endian，44 字节标准 RIFF/WAVE header）
//!
//! SIMD 加速：`wide::f32x4` 用于响度/软限幅的逐样本循环（4×吞吐）。
//! rayon 并行：批量音频处理或大音频分块可按 CHUNK 并行处理。

pub mod resample;
pub mod sola;
pub mod loudness;
pub mod wav;

pub use loudness::{apply_limiter_and_loudness, LimiterOptions};
pub use resample::resample_linear;
pub use sola::{time_stretch_sola, SolaOptions};
pub use wav::{encode_wav_pcm16, WavSpec};
