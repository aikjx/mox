// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_dsp_core::*;

// ─── LimiterOptions 默认值 ───

#[test]
fn limiter_options_default_values() {
    let opts = LimiterOptions::default();
    assert_eq!(opts.target_dbfs, -18.0);
    assert!(opts.enable_loudness);
}

#[test]
fn limiter_options_clone_and_debug() {
    let opts = LimiterOptions {
        target_dbfs: -20.0,
        enable_loudness: false,
    };
    let cloned = opts.clone();
    assert_eq!(cloned.target_dbfs, -20.0);
    assert!(!cloned.enable_loudness);
    let dbg = format!("{:?}", opts);
    assert!(dbg.contains("LimiterOptions"));
}

// ─── apply_limiter_and_loudness ───

#[test]
fn limiter_empty_input_returns_empty() {
    let out = apply_limiter_and_loudness(&[], &LimiterOptions::default());
    assert!(out.is_empty());
}

#[test]
fn limiter_preserves_length() {
    let sig = vec![0.5f32; 1024];
    let out = apply_limiter_and_loudness(&sig, &LimiterOptions::default());
    assert_eq!(out.len(), sig.len());
}

#[test]
fn limiter_clamps_peak_below_0995() {
    // 大信号 + 高增益应该触发软限幅
    let sig: Vec<f32> = (0..256).map(|i| {
        let t = i as f32 / 22050.0;
        (std::f32::consts::TAU * 440.0 * t).sin() * 2.0
    }).collect();
    let out = apply_limiter_and_loudness(
        &sig,
        &LimiterOptions { target_dbfs: -18.0, enable_loudness: false },
    );
    let peak = out.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(peak < 0.995, "peak {} 应 < 0.995", peak);
}

// ─── SolaOptions 默认值 ───

#[test]
fn sola_options_default_values() {
    let opts = SolaOptions::default();
    assert_eq!(opts.frame_ms, 20.0);
    assert_eq!(opts.overlap_ms, 10.0);
    assert_eq!(opts.sample_rate, 22050);
}

#[test]
fn sola_options_custom_construction() {
    let opts = SolaOptions {
        frame_ms: 30.0,
        overlap_ms: 15.0,
        sample_rate: 44100,
    };
    assert_eq!(opts.sample_rate, 44100);
    assert_eq!(opts.frame_ms, 30.0);
}

// ─── time_stretch_sola ───

#[test]
fn sola_empty_or_tiny_noop() {
    let out = time_stretch_sola(&[], 100, &SolaOptions::default());
    assert!(out.is_empty());

    let a = vec![1.0f32; 8];
    let b = time_stretch_sola(&a, 8, &SolaOptions::default());
    assert_eq!(b.len(), 8);
}

#[test]
fn sola_stretch_changes_length() {
    // 0.2 秒正弦信号，变速 1.5 倍 → 长度约为 2/3
    let sr = 22050u32;
    let dur = 0.2;
    let n = (sr as f32 * dur) as usize;
    let sig: Vec<f32> = (0..n)
        .map(|i| (std::f32::consts::TAU * 220.0 * i as f32 / sr as f32).sin() * 0.5)
        .collect();

    let speed = 1.5f32;
    let target_len = (n as f32 / speed) as usize;
    let out = time_stretch_sola(&sig, target_len, &SolaOptions { sample_rate: sr, ..Default::default() });
    assert_eq!(out.len(), target_len);
}

// ─── WavSpec 默认值 ───

#[test]
fn wav_spec_default_values() {
    let spec = WavSpec::default();
    assert_eq!(spec.sample_rate, 22050);
    assert_eq!(spec.channels, 1);
}

#[test]
fn wav_spec_copy_and_clone() {
    let spec = WavSpec { sample_rate: 44100, channels: 2 };
    let copied = spec; // Copy
    assert_eq!(copied.sample_rate, 44100);
    let cloned = spec.clone();
    assert_eq!(cloned.channels, 2);
    let dbg = format!("{:?}", spec);
    assert!(dbg.contains("WavSpec"));
}

// ─── encode_wav_pcm16 ───

#[test]
fn wav_header_is_44_bytes() {
    let out = encode_wav_pcm16(&[], &WavSpec::default());
    assert_eq!(out.len(), 44);
    assert_eq!(&out[0..4], b"RIFF");
    assert_eq!(&out[8..12], b"WAVE");
    assert_eq!(&out[12..16], b"fmt ");
    assert_eq!(&out[36..40], b"data");
}

#[test]
fn wav_mono_data_size() {
    let samples = vec![0.0f32, 0.5, -0.5, 1.0];
    let out = encode_wav_pcm16(&samples, &WavSpec::default());
    // 44 字节 header + 4 样本 * 2 字节 = 52
    assert_eq!(out.len(), 44 + 4 * 2);
}

#[test]
fn wav_stereo_data_size() {
    let samples = vec![0.0f32, 0.0, 0.5, 0.5]; // 2 帧 * 2 通道
    let out = encode_wav_pcm16(
        &samples,
        &WavSpec { sample_rate: 44100, channels: 2 },
    );
    assert_eq!(out.len(), 44 + 4 * 2);
}

#[test]
fn wav_clamps_out_of_range_samples() {
    let out = encode_wav_pcm16(&[10.0, -10.0], &WavSpec::default());
    // 第一个样本（正饱和）
    let a = i16::from_le_bytes([out[44], out[45]]);
    // 第二个样本（负饱和）
    let b = i16::from_le_bytes([out[46], out[47]]);
    assert_eq!(a, i16::MAX);
    assert_eq!(b, i16::MIN);
}

// ─── resample_linear ───

#[test]
fn resample_same_rate_returns_copy() {
    let a = vec![1.0f32, 2.0, 3.0, 4.0];
    let b = resample_linear(&a, 22050, 22050);
    assert_eq!(b, a);
}

#[test]
fn resample_empty_input() {
    assert!(resample_linear(&[], 16000, 22050).is_empty());
}

#[test]
fn resample_upsample_increases_length() {
    let a = vec![0.0f32, 1.0];
    let b = resample_linear(&a, 1, 2); // 1Hz -> 2Hz: 2 samples -> ~4 samples
    assert_eq!(b.len(), 4);
    // 首末点不变
    assert!((b[0] - 0.0).abs() < 1e-6);
    assert!((b[3] - 1.0).abs() < 1e-6);
}

#[test]
fn resample_downsample_decreases_length() {
    let a: Vec<f32> = (0..100).map(|i| i as f32).collect();
    let b = resample_linear(&a, 100, 50); // 2:1 降采样
    assert_eq!(b.len(), 50);
}

#[test]
fn resample_single_element_repeated() {
    let a = vec![42.0f32];
    let b = resample_linear(&a, 1, 10);
    assert_eq!(b.len(), 10);
    for &v in &b {
        assert_eq!(v, 42.0);
    }
}
