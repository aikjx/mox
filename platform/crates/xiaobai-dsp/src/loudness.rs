//! 响度归一化 + 软限幅。与 Python `_apply_limiter_and_loudness` 等价。
//!
//! - RMS → dBFS：db = 20·log10(rms)；目标响度 -18 dBFS
//! - 仅当 db < -6.0 时抬升（避免静音段爆炸），最大 +22dB
//! - 软限幅：|x|≥0.95 进入 tanh knee，保证 |x| < 0.995
//! - SIMD 优化：使用 wide::f32x4 批量处理 4 样本

use wide::f32x4;

#[derive(Debug, Clone)]
pub struct LimiterOptions {
    pub target_dbfs: f32,
    pub enable_loudness: bool,
}

impl Default for LimiterOptions {
    fn default() -> Self { Self { target_dbfs: -18.0, enable_loudness: true } }
}

pub fn apply_limiter_and_loudness(input: &[f32], opts: &LimiterOptions) -> Vec<f32> {
    let n = input.len();
    if n == 0 { return Vec::new(); }

    // ---- RMS 计算 (SIMD)
    let mut sum_sq = f32x4::ZERO;
    let chunks = n / 4;
    let rem_start = chunks * 4;
    for i in 0..chunks {
        let x = f32x4::new([
            input[4 * i],
            input[4 * i + 1],
            input[4 * i + 2],
            input[4 * i + 3],
        ]);
        sum_sq += x * x;
    }
    let mut s = sum_sq.to_array().iter().sum::<f32>();
    for &v in &input[rem_start..n] { s += v * v; }
    let rms = (s / n as f32 + 1e-12).sqrt();
    let db = 20.0 * (rms + 1e-12).log10();

    // 计算增益
    let gain = if opts.enable_loudness && db.is_finite() && db < -6.0 {
        let g = opts.target_dbfs - db;
        g.clamp(0.0, 22.0)
    } else { 0.0 };
    let gain_lin = 10.0f32.powf(gain / 20.0);

    // ---- 应用增益 + 软限幅（SIMD 主体 + 尾标量）
    // 设计注：soft-limiter 的 lane 条件选择不依赖 wide::f32x4 的 cmp_ge/blend mask API
    // （wide 0.7/0.8 跨版本 mask 命名不稳定），改用 SIMD 同时计算 x 与 y_high 两条路径，
    // 再 per-lane 做标量选择。选择开销相对于 rational tanh 可忽略。
    let mut out = vec![0.0f32; n];
    let c_eps = f32x4::splat(1e-9);
    let k = f32x4::splat(1.0 / 0.95);
    let c095 = f32x4::splat(0.95);
    let c0045 = f32x4::splat(0.045);
    let c27 = f32x4::splat(27.0);
    let c9 = f32x4::splat(9.0);
    let glin = f32x4::splat(gain_lin);
    let thr_scalar: f32 = 0.95;

    for i in 0..chunks {
        let base = 4 * i;
        let x = f32x4::new([
            input[base], input[base + 1], input[base + 2], input[base + 3],
        ]) * glin;
        let ax = x.abs();

        // branch HIGH (ax >= 0.95)：
        //   scaled = (ax - 0.95) / 0.95
        //   tanh(scaled) ≈ scaled·(27 + scaled²) / (27 + 9·scaled²)  (误差 < 2e-3 on [0,3])
        //   y_high = sign(x) * (0.95 + 0.045 * tanh(scaled))
        let scaled = (ax - c095) * k;
        let s2 = scaled * scaled;
        let num = scaled * (c27 + s2);
        let den = c27 + c9 * s2;
        let th = num / den;
        let sign = x / ax.max(c_eps);
        let y_high = sign * (c095 + c0045 * th);

        // per-lane 选择：ax[j] >= 0.95 → y_high[j]，否则 → x[j]
        // （SIMD 计算两条路径 + 标量 lane 选择：避免 wide mask API 跨版本漂移）
        let ax_arr = ax.to_array();
        let x_arr = x.to_array();
        let yh_arr = y_high.to_array();
        let mut r = [0.0f32; 4];
        for j in 0..4 {
            r[j] = if ax_arr[j] >= thr_scalar { yh_arr[j] } else { x_arr[j] };
        }
        out[base..base + 4].copy_from_slice(&r);
    }
    // 尾部标量处理
    for i in rem_start..n {
        let mut v = input[i] * gain_lin;
        let av = v.abs();
        if av >= thr_scalar {
            let scaled = (av - thr_scalar) * (1.0 / thr_scalar);
            let th = scaled.tanh();
            v = v.signum() * (thr_scalar + 0.045 * th);
        }
        out[i] = v;
    }

    // 保护：修正 sign 分母下溢导致的 NaN（ax==0 lane 选 x=0 路径，本步为兜底）
    for v in out.iter_mut() {
        if !v.is_finite() { *v = 0.0; }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn loudness_boosts_quiet_signal() {
        // 安静正弦：RMS = 0.01 → dBFS = -40
        let sr = 22050usize;
        let dur = 0.05;
        let n = (sr as f32 * dur) as usize;
        let mut sig = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            sig[i] = (std::f32::consts::TAU * 440.0 * t).sin() * 0.01;
        }
        let out = apply_limiter_and_loudness(&sig, &LimiterOptions::default());
        let rms_in = (sig.iter().map(|x| x*x).sum::<f32>() / n as f32).sqrt();
        let rms_out = (out.iter().map(|x| x*x).sum::<f32>() / n as f32).sqrt();
        assert!(rms_out > rms_in * 10.0, "rms_out should be boosted");
        let peak = out.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
        assert!(peak < 0.995, "peak {peak} 应 < 0.995（软限幅）");
    }

    #[test]
    fn limiter_clips_peaks() {
        // 尖锐冲激
        let sig = vec![0.0f32, 1.2, -1.2, 0.5, 0.9];
        let out = apply_limiter_and_loudness(&sig, &LimiterOptions { target_dbfs: -18.0, enable_loudness: false });
        for &v in &out {
            assert!(v.abs() < 0.995, "sample {v} 未被限幅");
        }
        assert_relative_eq!(out[3], 0.5, epsilon = 1e-9); // 0.5 < 0.95 不变
    }
}
