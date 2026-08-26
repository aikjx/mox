//! PyO3 扩展：xiaobai-dsp → Python `mox_voice_dsp_py`
//!
//! 目标：被 xiaobai_voice TTS 生产链路直接 import 使用，
//!       替换 Python 侧纯 _apply_limiter_and_loudness / resample_linear / time_stretch_sola / wav.encode。
//!
//! 输入兼容：
//!   - numpy 1D f32 数组（优先，0-copy via PyO3 numpy 边界）
//!   - Python list[float]（fallback）
//! 输出：numpy 1D f32 数组（用于 pipeline 串联）或 bytes（WAV 编码）。

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use mox_voice_dsp_core::{
    apply_limiter_and_loudness, encode_wav_pcm16, resample_linear, time_stretch_sola,
    LimiterOptions, SolaOptions, WavSpec,
};

fn to_vec_f32<'py>(obj: &Bound<'py, PyAny>) -> PyResult<Vec<f32>> {
    // 1) Try numpy first via numpy crate
    if let Ok(npy) = obj.extract::<numpy::PyReadonlyArray1<f32>>() {
        return Ok(npy.as_slice().unwrap_or(&[]).to_vec());
    }
    // 2) Fallback to list[float]
    if let Ok(lst) = obj.downcast::<PyList>() {
        let mut out = Vec::with_capacity(lst.len());
        for item in lst.iter() {
            let v: f32 = item.extract()?;
            out.push(v);
        }
        return Ok(out);
    }
    Err(PyValueError::new_err(
        "signal must be numpy 1D f32 array or list[float]",
    ))
}

fn to_numpy_or_list(py: Python<'_>, data: Vec<f32>) -> PyResult<Py<PyAny>> {
    // numpy 优先（若可用）；否则退化为 Python list[float]
    let np_mod = py.import_bound("numpy").ok();
    if let Some(np) = &np_mod {
        if let Ok(arr) = np.call_method1("array", (data.clone(),)) {
            return Ok(arr.unbind());
        }
    }
    let items: Vec<PyObject> = data.into_iter().map(|x| x.into_py(py)).collect();
    let lst = PyList::new_bound(py, items.iter());
    Ok(lst.unbind().into_any())
}

#[pymodule]
fn mox_voice_dsp_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // ----- 1. 线性插值重采样 -----
    #[pyfn(m)]
    #[pyo3(name = "resample_linear")]
    fn py_resample_linear<'py>(
        py: Python<'py>,
        signal: &Bound<'py, PyAny>,
        orig_sr: u32,
        target_sr: u32,
    ) -> PyResult<Py<PyAny>> {
        let sig = to_vec_f32(signal)?;
        if orig_sr == 0 || target_sr == 0 {
            return Err(PyValueError::new_err("sample_rate must be > 0"));
        }
        let out = resample_linear(&sig, orig_sr, target_sr);
        to_numpy_or_list(py, out)
    }

    // ----- 2. SOLA 时域变速（不变调） -----
    #[pyfn(m)]
    #[pyo3(name = "time_stretch_sola", signature = (signal, *, sample_rate = 22050u32, speed = 1.0f32))]
    fn py_time_stretch_sola<'py>(
        py: Python<'py>,
        signal: &Bound<'py, PyAny>,
        sample_rate: u32,
        speed: f32,
    ) -> PyResult<Py<PyAny>> {
        let sig = to_vec_f32(signal)?;
        if sample_rate == 0 {
            return Err(PyValueError::new_err("sample_rate must be > 0"));
        }
        if !(0.25..=4.0).contains(&speed) {
            return Err(PyValueError::new_err(
                "speed must be within [0.25, 4.0] (SOLA stable range)",
            ));
        }
        let target_len = (sig.len() as f32 / speed.max(1e-6)) as usize;
        let opts = SolaOptions { sample_rate, ..Default::default() };
        let out = time_stretch_sola(&sig, target_len, &opts);
        to_numpy_or_list(py, out)
    }

    // ----- 3. 响度归一 + 软限幅 -----
    #[pyfn(m)]
    #[pyo3(name = "apply_limiter_and_loudness", signature = (signal, *, target_dbfs = -18.0f32, enable_loudness = true))]
    fn py_apply_limiter<'py>(
        py: Python<'py>,
        signal: &Bound<'py, PyAny>,
        target_dbfs: f32,
        enable_loudness: bool,
    ) -> PyResult<Py<PyAny>> {
        let sig = to_vec_f32(signal)?;
        let opts = LimiterOptions { target_dbfs, enable_loudness };
        let out = apply_limiter_and_loudness(&sig, &opts);
        to_numpy_or_list(py, out)
    }

    // ----- 4. PCM WAV 16bit LE 编码（返回 bytes） -----
    #[pyfn(m)]
    #[pyo3(name = "encode_wav_pcm16", signature = (signal, *, sample_rate = 22050u32, channels = 1u16))]
    fn py_encode_wav<'py>(
        _py: Python<'py>,
        signal: &Bound<'py, PyAny>,
        sample_rate: u32,
        channels: u16,
    ) -> PyResult<Py<PyBytes>> {
        let sig = to_vec_f32(signal)?;
        if sample_rate == 0 || sample_rate > 192_000 {
            return Err(PyValueError::new_err(
                "sample_rate must be within (0, 192000]",
            ));
        }
        if channels == 0 || channels > 2 {
            return Err(PyValueError::new_err("channels must be 1 (mono) or 2 (stereo)"));
        }
        if sig.len() % channels as usize != 0 {
            return Err(PyValueError::new_err(format!(
                "signal length ({}) 必须是 channels({}) 的整数倍（交织帧）",
                sig.len(),
                channels
            )));
        }
        let spec = WavSpec { sample_rate, channels };
        let bytes = encode_wav_pcm16(&sig, &spec);
        Ok(PyBytes::new_bound(_py, &bytes).unbind())
    }

    // ----- 5. 全链路组合：resample → SOLA → loudness → (wav bytes) -----
    /// 从 PyAny 中按 key 取值（支持 dict、Mapping、属性对象）。
    fn get_opt<'py, T: for<'a> FromPyObject<'a>>(obj: &Bound<'py, PyAny>, key: &str) -> PyResult<Option<T>> {
        if let Ok(dict) = obj.downcast::<pyo3::types::PyDict>() {
            if let Some(v) = dict.get_item(key)? {
                return v.extract::<T>().map(Some);
            }
            return Ok(None);
        }
        // Mapping（如 TypedDict/MappingProxy）：obj[key]
        if let Ok(getitem) = obj.get_item(key) {
            if !getitem.is_none() {
                return getitem.extract::<T>().map(Some);
            }
        }
        // 对象（带 attr 如 NamedTuple/dataclass）
        if let Ok(attr) = obj.getattr(key) {
            if !attr.is_none() {
                return attr.extract::<T>().map(Some);
            }
        }
        Ok(None)
    }

    #[pyfn(m)]
    #[pyo3(name = "apply_dsp_pipeline", signature = (signal, opts = None, *, orig_sr = None, target_sr = None, speed = None, target_dbfs = None, enable_loudness = None, encode_wav = None, channels = None))]
    #[allow(clippy::too_many_arguments)]
    fn py_pipeline<'py>(
        py: Python<'py>,
        signal: &Bound<'py, PyAny>,
        opts: Option<&Bound<'py, PyAny>>,
        orig_sr: Option<u32>,
        target_sr: Option<u32>,
        speed: Option<f32>,
        target_dbfs: Option<f32>,
        enable_loudness: Option<bool>,
        encode_wav: Option<bool>,
        channels: Option<u16>,
    ) -> PyResult<Py<PyAny>> {
        // opts dict/mapping 合并优先级：显式关键字 > opts 对象
        let mut o_orig_sr = orig_sr;
        let mut o_target_sr = target_sr;
        let mut o_speed = speed;
        let mut o_target_dbfs = target_dbfs;
        let mut o_enable_loudness = enable_loudness;
        let mut o_encode_wav = encode_wav;
        let mut o_channels = channels;
        if let Some(o) = opts {
            if o_orig_sr.is_none() { o_orig_sr = get_opt::<u32>(o, "orig_sr")?; }
            if o_target_sr.is_none() { o_target_sr = get_opt::<u32>(o, "target_sr")?; }
            if o_speed.is_none() { o_speed = get_opt::<f32>(o, "speed")?; }
            if o_target_dbfs.is_none() { o_target_dbfs = get_opt::<f32>(o, "target_dbfs")?; }
            if o_enable_loudness.is_none() { o_enable_loudness = get_opt::<bool>(o, "enable_loudness")?; }
            if o_encode_wav.is_none() { o_encode_wav = get_opt::<bool>(o, "encode_wav")?; }
            if o_channels.is_none() { o_channels = get_opt::<u16>(o, "channels")?; }
        }
        let mut sig = to_vec_f32(signal)?;
        let orig_sr_v = o_orig_sr.unwrap_or(22050);
        let target_sr_v = o_target_sr.unwrap_or(orig_sr_v);
        if orig_sr_v == 0 || target_sr_v == 0 {
            return Err(PyValueError::new_err("sample_rate must be > 0"));
        }
        if orig_sr_v != target_sr_v {
            sig = resample_linear(&sig, orig_sr_v, target_sr_v);
        }
        let speed_v = o_speed.unwrap_or(1.0);
        if (speed_v - 1.0).abs() > 1e-6 {
            if !(0.25..=4.0).contains(&speed_v) {
                return Err(PyValueError::new_err("speed must be within [0.25, 4.0]"));
            }
            let target_len = (sig.len() as f32 / speed_v.max(1e-6)) as usize;
            let sopt = SolaOptions { sample_rate: target_sr_v, ..Default::default() };
            sig = time_stretch_sola(&sig, target_len, &sopt);
        }
        let lopts = LimiterOptions {
            target_dbfs: o_target_dbfs.unwrap_or(-18.0),
            enable_loudness: o_enable_loudness.unwrap_or(true),
        };
        sig = apply_limiter_and_loudness(&sig, &lopts);

        if o_encode_wav.unwrap_or(false) {
            let ch = o_channels.unwrap_or(1);
            if sig.len() % ch as usize != 0 {
                return Err(PyValueError::new_err(format!(
                    "channels={ch} but samples={} not divisible",
                    sig.len()
                )));
            }
            let spec = WavSpec {
                sample_rate: target_sr_v,
                channels: ch,
            };
            let b = encode_wav_pcm16(&sig, &spec);
            return Ok(PyBytes::new_bound(py, &b).unbind().into_any());
        }
        to_numpy_or_list(py, sig)
    }

    Ok(())
}
