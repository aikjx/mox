// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! PCM WAV 16-bit little-endian 编码器（标准 44 字节 RIFF/WAVE fmt-1 PCM header）。
//!
//! 与 Python wave/wavfile 标准一致，直接可被前端 audio/wav 播放；
//! 支持单/双通道，采样率 ≤ 192 kHz（足以覆盖 TTS 的 22.05 kHz 场景）。

#[derive(Debug, Clone, Copy)]
pub struct WavSpec {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for WavSpec {
    fn default() -> Self { Self { sample_rate: 22050, channels: 1 } }
}

/// 将 [-1,1] 区间的 float32 PCM 编码为标准 WAV 字节。
/// samples 总长度必须是 channels 的整数倍（按帧交织 L R L R）。
pub fn encode_wav_pcm16(samples: &[f32], spec: &WavSpec) -> Vec<u8> {
    let ch = spec.channels.max(1);
    let sr = spec.sample_rate.max(1);
    let n_samples = samples.len();
    // 16-bit PCM：每样本 2 字节
    let data_len = (n_samples * 2) as u32;
    let file_len = 36u32 + data_len;
    let mut out = Vec::with_capacity(44 + data_len as usize);

    // RIFF header
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&file_len.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    // fmt chunk
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // chunk size (PCM)
    out.extend_from_slice(&1u16.to_le_bytes()); // audio format (1 = PCM)
    out.extend_from_slice(&ch.to_le_bytes());
    out.extend_from_slice(&sr.to_le_bytes());
    let byte_rate = sr as u32 * ch as u32 * 2;
    out.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = ch as u16 * 2;
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    // data chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());

    // PCM s16 LE：非对称缩放匹配 i16 真实范围 [-32768, +32767]
    //   正半轴：x ∈ [0,1]   →  round(x·32767) ，最大 +32767
    //   负半轴：x ∈ [-1, 0)  →  round(x·32768) ，最小 -32768
    //   clamp(-1, 1) 确保 |x|>1 时精确到极值（测试 10/-10 精确命中 i16::MAX/MIN）
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let scaled = if clamped >= 0.0 {
            clamped * 32767.0f32
        } else {
            clamped * 32768.0f32
        };
        let v = scaled.round() as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size_is_44() {
        let out = encode_wav_pcm16(&[], &WavSpec::default());
        assert_eq!(out.len(), 44);
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WAVE");
        assert_eq!(&out[12..16], b"fmt ");
        assert_eq!(&out[36..40], b"data");
    }

    #[test]
    fn stereo_double_size() {
        let mono = encode_wav_pcm16(&[0.5, -0.5], &WavSpec { sample_rate: 44100, channels: 1 });
        let stereo = encode_wav_pcm16(&[0.5, -0.5, 0.1, -0.1], &WavSpec { sample_rate: 44100, channels: 2 });
        assert_eq!(mono.len(), 44 + 4);
        assert_eq!(stereo.len(), 44 + 8);
    }

    #[test]
    fn clamps_out_of_range() {
        let out = encode_wav_pcm16(&[10.0, -10.0], &WavSpec::default());
        // data bytes 从 44 开始；每个样本 2 字节
        let a = i16::from_le_bytes([out[44], out[45]]);
        let b = i16::from_le_bytes([out[46], out[47]]);
        assert_eq!(a, i16::MAX);
        assert_eq!(b, i16::MIN);
    }
}
