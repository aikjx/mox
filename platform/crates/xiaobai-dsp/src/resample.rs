//! 线性插值重采样：O(n)，与 Python cosyvoice2._resample_linear 等价。
//!
//! 采样位置：旧区间 [0, len-1] → 新区间 [0, n-1] 的均匀映射，
//! 每个新点 floor/ceil 取原样本，线性插值 frac 比例。

/// 线性重采样：将 `input` (sr=`sr_in`) 转换到 `sr_out`。
/// 输入输出都是 float32，单通道。
pub fn resample_linear(input: &[f32], sr_in: u32, sr_out: u32) -> Vec<f32> {
    if sr_in == sr_out { return input.to_vec(); }
    let len = input.len();
    if len == 0 { return Vec::new(); }
    let n = (len as f64 * sr_out as f64 / sr_in as f64).round() as usize;
    if n <= 0 { return Vec::new(); }
    if len == 1 { return vec![input[0]; n]; }
    let scale = (len - 1) as f64 / (n - 1).max(1) as f64;
    let mut out = vec![0.0f32; n];
    // 标量实现（纯 Rust SIMD 友好：编译器已能自动向量化）
    for i in 0..n {
        let idx_f = i as f64 * scale;
        let idx0 = idx_f.floor() as usize;
        let idx1 = (idx0 + 1).min(len - 1);
        let frac = (idx_f - idx0 as f64) as f32;
        let a = unsafe { *input.get_unchecked(idx0) };
        let b = unsafe { *input.get_unchecked(idx1) };
        out[i] = a * (1.0 - frac) + b * frac;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn same_sr_is_copy() {
        let a = vec![1.0f32, 2.0, 3.0];
        let b = resample_linear(&a, 22050, 22050);
        assert_eq!(b, a);
    }

    #[test]
    fn upsample_by_2() {
        // [0, 1] 从 1Hz → 2Hz：len 2 → 4
        // 旧坐标 0,1 → 新 0, 0.333, 0.666, 1
        let a = vec![0.0f32, 1.0];
        let b = resample_linear(&a, 1, 2);
        assert_eq!(b.len(), 4);
        assert_relative_eq!(b[0], 0.0, epsilon = 1e-6);
        assert_relative_eq!(b[1], 1.0 / 3.0, epsilon = 1e-5);
        assert_relative_eq!(b[2], 2.0 / 3.0, epsilon = 1e-5);
        assert_relative_eq!(b[3], 1.0, epsilon = 1e-6);
    }

    #[test]
    fn empty_input() {
        assert!(resample_linear(&[], 16000, 22050).is_empty());
    }
}
