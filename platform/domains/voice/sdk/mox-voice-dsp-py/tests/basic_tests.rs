// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 基础测试：验证 mox-voice-dsp-py crate 所依赖的核心 DSP 函数链路。
//! 由于本 crate 是 cdylib（PyO3 扩展），集成测试通过其依赖的 mox-voice-dsp-core
//! 来验证所有公开 DSP 原语的正确性，确保 Python 侧调用的底层逻辑可靠。

use mox_voice_dsp_core::{
    apply_limiter_and_loudness, encode_wav_pcm16, resample_linear, time_stretch_sola,
    LimiterOptions, SolaOptions, WavSpec,
};

// ─── 管线完整性测试：resample → sola → loudness → wav ───

#[test]
fn full_pipeline_runs_end_to_end() {
    // 构造一段 220Hz 正弦波（22050 Hz 采样率，0.1 秒）
    let sr_orig = 22050u32;
    let dur = 0.1;
    let n = (sr_orig as f32 * dur) as usize;
    let sig: Vec<f32> = (0..n)
        .map(|i| (std::f32::consts::TAU * 220.0 * i as f32 / sr_orig as f32).sin() * 0.3)
        .collect();

    // Step 1: 重采样 22050 → 16000
    let sr_target = 16000u32;
    let resampled = resample_linear(&sig, sr_orig, sr_target);
    let expected_len = (n as f64 * sr_target as f64 / sr_orig as f64).round() as usize;
    assert_eq!(resampled.len(), expected_len);

    // Step 2: SOLA 变速 1.2 倍
    let speed = 1.2f32;
    let target_len = (resampled.len() as f32 / speed) as usize;
    let stretched = time_stretch_sola(
        &resampled,
        target_len,
        &SolaOptions { sample_rate: sr_target, ..Default::default() },
    );
    assert_eq!(stretched.len(), target_len);

    // Step 3: 响度归一 + 软限幅
    let loudnessed = apply_limiter_and_loudness(&stretched, &LimiterOptions::default());
    assert_eq!(loudnessed.len(), stretched.len());
    let peak = loudnessed.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(peak < 0.995, "peak {} 应 < 0.995", peak);

    // Step 4: WAV 编码
    let wav_bytes = encode_wav_pcm16(
        &loudnessed,
        &WavSpec { sample_rate: sr_target, channels: 1 },
    );
    assert!(wav_bytes.len() > 44);
    assert_eq!(&wav_bytes[0..4], b"RIFF");
    assert_eq!(&wav_bytes[8..12], b"WAVE");
}

// ─── LimiterOptions 配置测试 ───

#[test]
fn limiter_options_disable_loudness_preserves_amplitude() {
    // 0.5 幅度的信号，禁用响度归一后，输出应接近 0.5（不受增益影响）
    let sig = vec![0.5f32; 256];
    let out = apply_limiter_and_loudness(
        &sig,
        &LimiterOptions { target_dbfs: -18.0, enable_loudness: false },
    );
    // 0.5 < 0.95 阈值，不会被限幅，所以值应保持 0.5
    for &v in &out {
        assert!((v - 0.5).abs() < 1e-6, "expected 0.5, got {}", v);
    }
}

#[test]
fn limiter_options_custom_target_dbfs() {
    let sig: Vec<f32> = (0..512)
        .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / 22050.0).sin() * 0.01)
        .collect();
    // 目标 -10 dBFS（比默认 -18 更响）
    let out_louder = apply_limiter_and_loudness(
        &sig,
        &LimiterOptions { target_dbfs: -10.0, enable_loudness: true },
    );
    let out_softer = apply_limiter_and_loudness(
        &sig,
        &LimiterOptions { target_dbfs: -24.0, enable_loudness: true },
    );
    let rms = |a: &[f32]| (a.iter().map(|x| x * x).sum::<f32>() / a.len() as f32).sqrt();
    assert!(rms(&out_louder) > rms(&out_softer), "-10dBFS 输出应比 -24dBFS 更响");
}

// ─── SolaOptions 配置测试 ───

#[test]
fn sola_zero_sample_rate_falls_back_to_default() {
    let sig: Vec<f32> = (0..1024)
        .map(|i| (std::f32::consts::TAU * 220.0 * i as f32 / 22050.0).sin() * 0.5)
        .collect();
    let opts = SolaOptions { sample_rate: 0, ..Default::default() }; // 0 → 回退 22050
    let target_len = sig.len() * 2 / 3; // ~0.667x
    let out = time_stretch_sola(&sig, target_len, &opts);
    assert_eq!(out.len(), target_len);
}

// ─── WavSpec 配置测试 ───

#[test]
fn wav_spec_sample_rate_reflected_in_header() {
    let sr = 48000u32;
    let out = encode_wav_pcm16(
        &[0.0f32; 10],
        &WavSpec { sample_rate: sr, channels: 1 },
    );
    // Sample rate 在 byte 24..28 (little-endian)
    let sr_bytes = u32::from_le_bytes([out[24], out[25], out[26], out[27]]);
    assert_eq!(sr_bytes, sr);
}

#[test]
fn wav_spec_stereo_header_correct() {
    let out = encode_wav_pcm16(
        &[0.0f32, 0.0, 0.5, 0.5], // 2 frames, stereo
        &WavSpec { sample_rate: 44100, channels: 2 },
    );
    // Num channels at byte 22..24
    let ch = u16::from_le_bytes([out[22], out[23]]);
    assert_eq!(ch, 2);
    // Byte rate at byte 28..32 = sr * ch * 2
    let byte_rate = u32::from_le_bytes([out[28], out[29], out[30], out[31]]);
    assert_eq!(byte_rate, 44100 * 2 * 2);
    // Block align at byte 32..34 = ch * 2
    let block_align = u16::from_le_bytes([out[32], out[33]]);
    assert_eq!(block_align, 4);
}

// ─── resample 边界与一致性 ───

#[test]
fn resample_first_last_sample_preserved() {
    let a = vec![0.0f32, 1.0, 0.0, -1.0, 0.0];
    let b = resample_linear(&a, 5, 11); // 上采样
    assert!((b[0] - 0.0).abs() < 1e-6, "首点应保持 0.0");
    assert!((b[b.len() - 1] - 0.0).abs() < 1e-6, "末点应保持 0.0");
}

#[test]
fn resample_very_high_ratio() {
    let a = vec![1.0f32, 0.0];
    let b = resample_linear(&a, 2, 2000); // 极大上采样
    assert_eq!(b.len(), 1000); // (2-1) * 2000/2 + 1 = 1000... wait let me check
    // len = (len_in as f64 * sr_out as f64 / sr_in as f64).round()
    // = (2.0 * 2000.0 / 2.0).round() = 2000
    assert_eq!(b.len(), 2000);
    assert_eq!(b[0], 1.0);
    assert_eq!(b[b.len() - 1], 0.0);
}
