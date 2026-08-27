// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! SOLA-like 时域缩放（不改变音高）。与 Python `_time_stretch_sola` 等价。
//!
//! - 小幅变速（speed 0.8~1.3 是 90% 场景，±30%）
//! - frame=20ms / overlap=10ms：480Hz / 1000Hz 基频稳定
//! - overlap-add with hanning window；cross-fade 在 overlap 区搜索 ±search 互相关最大偏移

#[derive(Debug, Clone)]
pub struct SolaOptions {
    pub frame_ms: f32,
    pub overlap_ms: f32,
    pub sample_rate: u32,
}

impl Default for SolaOptions {
    fn default() -> Self { Self { frame_ms: 20.0, overlap_ms: 10.0, sample_rate: 22050 } }
}

/// 将 `input` 拉伸到 `target_len` 长度（不改变音高）。
pub fn time_stretch_sola(input: &[f32], target_len: usize, opts: &SolaOptions) -> Vec<f32> {
    let sr = if opts.sample_rate == 0 { 22050 } else { opts.sample_rate } as usize;
    let frame = (sr as f32 * (opts.frame_ms / 1000.0)) as usize;
    let frame = frame.max(32).next_power_of_two().min(8192).max(32);
    let overlap = (sr as f32 * (opts.overlap_ms / 1000.0)) as usize;
    let overlap = overlap.max(8).min(frame / 2).max(8);
    let hop_synthesis = (frame - overlap).max(1);
    let src_len = input.len();
    if src_len <= 1 || target_len <= 1 { return input.to_vec(); }
    if src_len.abs_diff(target_len) <= 1 { return input.to_vec(); }
    let ratio = target_len as f64 / src_len as f64;
    let hop_analysis = (hop_synthesis as f64 / ratio).round() as usize;
    let hop_analysis = hop_analysis.max(1);

    let mut out = vec![0.0f32; target_len + frame + 16];
    // 汉宁窗
    let win: Vec<f32> = (0..frame)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / (frame - 1).max(1) as f32;
            0.5 * (1.0 - t.cos())
        })
        .collect();

    let n_frames = (src_len.saturating_sub(frame)) / hop_analysis + 1;
    let mut write_pos = 0usize;

    for i in 0..n_frames {
        let s = i * hop_analysis;
        let mut chunk = if s + frame <= src_len {
            input[s..s + frame].to_vec()
        } else {
            let mut c = vec![0.0f32; frame];
            let take = src_len - s;
            c[..take].copy_from_slice(&input[s..]);
            c
        };
        // 加窗
        for (c, &w) in chunk.iter_mut().zip(win.iter()) { *c *= w; }
        if write_pos == 0 {
            out[..frame].copy_from_slice(&chunk);
            write_pos += hop_synthesis;
            continue;
        }
        // 互相关对齐搜索
        let search = (overlap / 2).min(write_pos).min(out.len().saturating_sub(frame));
        if search > 0 && overlap >= 4 && write_pos + frame <= out.len() {
            let tail_need = &chunk[0..overlap];
            let mut best_off = 0i32;
            let mut best_corr = f32::NEG_INFINITY;
            for off in -(search as i32)..=(search as i32) {
                let t_start = write_pos as i64 + off as i64;
                if t_start < 0 { continue; }
                let t_end = t_start + overlap as i64;
                if t_end > out.len() as i64 { continue; }
                let buf = &out[t_start as usize..t_end as usize];
                let mut c = 0.0f32;
                for (a, b) in buf.iter().zip(tail_need.iter()) { c += a * b; }
                if c > best_corr { best_corr = c; best_off = off; }
            }
            let np = write_pos as i64 + best_off as i64;
            write_pos = if np < 0 { 0 } else { np as usize };
        }
        // write_pos 已在循环中通过 np<0 分支钳制为 0；此处无需重复 usize 无意义比较
        if write_pos + frame > out.len() {
            let grow = write_pos + frame - out.len() + 16;
            out.extend(std::iter::repeat(0.0f32).take(grow));
        }
        for k in 0..frame { out[write_pos + k] += chunk[k]; }
        write_pos += hop_synthesis;
    }

    if write_pos < target_len {
        if out.len() < target_len {
            out.extend(std::iter::repeat(0.0f32).take(target_len - out.len()));
        }
        out.truncate(target_len);
    } else {
        out.truncate(target_len);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_energy_basic() {
        // 构造一段 220Hz 正弦
        let sr = 22050u32;
        let dur_s = 0.2;
        let n = (sr as f32 * dur_s) as usize;
        let mut sig = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            sig[i] = (std::f32::consts::TAU * 220.0 * t).sin() * 0.5;
        }
        // speed 1.0 伸缩到相同长度：信号近似一致
        let opts = SolaOptions { sample_rate: sr, ..Default::default() };
        let out = time_stretch_sola(&sig, n, &opts);
        assert_eq!(out.len(), n);
        // 前 4096 点误差 < 0.05（SOLA 每帧 20ms 有轻微相位，要求不能太严）
        let err: f32 = sig[..4096.min(n)].iter().zip(out.iter()).map(|(a, b)| (a - b).abs()).sum::<f32>() / 4096.0 as f32;
        assert!(err < 0.05, "mean abs err = {err}");
    }

    #[test]
    fn tiny_length_is_noop() {
        let a = vec![1.0f32; 16];
        let b = time_stretch_sola(&a, 16, &SolaOptions::default());
        assert_eq!(b.len(), 16);
    }
}
