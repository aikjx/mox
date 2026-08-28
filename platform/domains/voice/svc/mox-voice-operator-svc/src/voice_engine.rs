// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! P2 语音闭环引擎：真录音（cpal）+ Paraformer ASR（sherpa-onnx）+ Kokoro TTS（sherpa-onnx）+ 播放（rodio）。
//!
//! 全本地离线、无网络依赖；模型放在 `models/voice/` 下：
//! - `asr-paraformer-streaming/` : tokens.txt + encoder.int8.onnx + decoder.int8.onnx
//! - `tts-kokoro/`               : model.onnx + tokens.txt + voices.bin + espeak-ng-data/
//!
//! 链路：录音 16k mono i16 → 重采样 → Paraformer 流式识别 → 文本
//!       → dispatch_text（外部）→ 回答文本 → Kokoro 合成 → rodio 播放

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// ================================================================ 录音 ========

/// 一次录音会话：`start()` 打开输入流，`stop()` 停止并返回 16k mono i16 样本。
/// 注意：cpal::Stream 非 Send，Recorder 必须始终在创建它的线程内使用（GUI 主线程）。
pub struct Recorder {
    stream: cpal::Stream,
    running: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl Recorder {
    /// 开始录音（使用设备默认配置）。停止时用 `stop(16000)` 重采样到目标采样率。
    pub fn start() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("没有找到默认录音设备（麦克风）")?;
        let default_cfg = device.default_input_config()?;
        let dev_rate = default_cfg.sample_rate().0;
        let channels = default_cfg.channels();
        let sample_format = default_cfg.sample_format();

        let running = Arc::new(AtomicBool::new(true));
        let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
        let running_cb = running.clone();
        let samples_cb = samples.clone();

        let err_fn = |e: cpal::StreamError| {
            tracing::error!(target: "xiaobai_voice", "录音流错误: {e}");
        };
        let channels_cb = channels as usize;

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let cb = samples_cb.clone();
                device.build_input_stream(
                    &default_cfg.into(),
                    move |data: &[f32], _| {
                        if !running_cb.load(Ordering::Relaxed) { return; }
                        let mut g = cb.lock().unwrap();
                        // 多声道 → 取第一声道（或平均），保证 mono
                        if channels_cb == 1 {
                            g.extend_from_slice(data);
                        } else {
                            for chunk in data.chunks(channels_cb) {
                                let m: f32 = chunk.iter().sum::<f32>() / chunk.len() as f32;
                                g.push(m);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let cb = samples_cb.clone();
                device.build_input_stream(
                    &default_cfg.into(),
                    move |data: &[i16], _| {
                        if !running_cb.load(Ordering::Relaxed) { return; }
                        let mut g = cb.lock().unwrap();
                        if channels_cb == 1 {
                            g.extend(data.iter().map(|v| *v as f32 / 32768.0));
                        } else {
                            for chunk in data.chunks(channels_cb) {
                                let m: f32 = chunk.iter().map(|v| *v as f32 / 32768.0).sum::<f32>() / chunk.len() as f32;
                                g.push(m);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::U16 => {
                let cb = samples_cb.clone();
                device.build_input_stream(
                    &default_cfg.into(),
                    move |data: &[u16], _| {
                        if !running_cb.load(Ordering::Relaxed) { return; }
                        let mut g = cb.lock().unwrap();
                        if channels_cb == 1 {
                            g.extend(data.iter().map(|v| (*v as f32 / 32768.0) - 1.0));
                        } else {
                            for chunk in data.chunks(channels_cb) {
                                let m: f32 = chunk.iter().map(|v| (*v as f32 / 32768.0) - 1.0).sum::<f32>() / chunk.len() as f32;
                                g.push(m);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            other => bail!("不支持的采样格式: {other:?}"),
        };
        stream.play().context("录音流无法启动")?;

        Ok(Self {
            stream,
            running,
            samples,
            sample_rate: dev_rate,
        })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// 停止录音并返回 16k mono i16 样本（自动重采样）。
    /// 消费 self：返回时内部 cpal 流 drop 即停止采集。
    pub fn stop(self, target_rate: u32) -> Vec<i16> {
        self.running.store(false, Ordering::Relaxed);
        let raw: Vec<f32> = std::mem::take(&mut *self.samples.lock().unwrap());
        let pcm = resample_to_i16(&raw, self.sample_rate, target_rate);
        // self 在这里 drop → cpal 输入流自动停止
        pcm
    }
}

/// 线性插值重采样到 16k，并转为 i16。
fn resample_to_i16(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if input.is_empty() {
        return Vec::new();
    }
    let n_out = ((input.len() as f64) * (to_rate as f64) / (from_rate as f64)).round() as usize;
    if n_out == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(n_out);
    let last = (input.len() - 1) as f64;
    for i in 0..n_out {
        let pos = (i as f64) * (input.len() as f64 - 1.0) / (n_out as f64 - 1.0);
        let i0 = pos.floor() as usize;
        let i1 = (i0 + 1).min(input.len() - 1);
        let frac = (pos - i0 as f64) as f32;
        let v = input[i0] * (1.0 - frac) + input[i1] * frac;
        out.push((v * 32767.0).clamp(-32768.0, 32767.0) as i16);
    }
    let _ = last;
    out
}

// ================================================================ 语音引擎 ====

/// 语音引擎：加载 Paraformer ASR + Kokoro TTS，提供识别 / 合成 / 播放。
pub struct VoiceEngine {
    recognizer: sherpa_onnx::OnlineRecognizer,
    tts: sherpa_onnx::OfflineTts,
    tts_sid: i32,
    /// 模型根目录
    pub model_root: PathBuf,
}

impl VoiceEngine {
    /// 从 `models/voice/` 加载引擎。
    /// 可选模型：asr 缺省跳过（recognition 不可用）；tts 缺省跳过（synthesize 不可用）。
    pub fn new(models_dir: &Path) -> Result<Self> {
        let asr_dir = models_dir.join("asr-paraformer-streaming");
        let tts_dir = models_dir.join("tts-kokoro");

        // ---- ASR: Paraformer streaming (int8) ----
        let recognizer = {
            let tokens = asr_dir.join("tokens.txt");
            let encoder = asr_dir.join("encoder.int8.onnx");
            let decoder = asr_dir.join("decoder.int8.onnx");
            if tokens.is_file() && encoder.is_file() && decoder.is_file() {
                let cfg = sherpa_onnx::OnlineRecognizerConfig {
                    model_config: sherpa_onnx::OnlineModelConfig {
                        paraformer: sherpa_onnx::OnlineParaformerModelConfig {
                            encoder: Some(encoder.to_string_lossy().into_owned()),
                            decoder: Some(decoder.to_string_lossy().into_owned()),
                        },
                        tokens: Some(tokens.to_string_lossy().into_owned()),
                        num_threads: 4,
                        provider: Some("cpu".into()),
                        ..Default::default()
                    },
                    enable_endpoint: true,
                    rule1_min_trailing_silence: 2.4,
                    rule2_min_trailing_silence: 1.2,
                    rule3_min_utterance_length: 0.0,
                    ..Default::default()
                };
                sherpa_onnx::OnlineRecognizer::create(&cfg)
                    .with_context(|| format!("Paraformer recognizer 创建失败（模型路径: {asr_dir:?}）"))?
            } else {
                bail!("ASR 模型缺失：{asr_dir:?} 下需要 tokens.txt + encoder.int8.onnx + decoder.int8.onnx")
            }
        };

        // ---- TTS: Kokoro multi-lang v1.0 ----
        let tts = {
            let model = tts_dir.join("model.onnx");
            let tokens = tts_dir.join("tokens.txt");
            let voices = tts_dir.join("voices.bin");
            let data_dir = tts_dir.join("espeak-ng-data");
            let lexicon_zh = tts_dir.join("lexicon-zh.txt");
            let lexicon_en = tts_dir.join("lexicon-us-en.txt");
            if !model.is_file() || !tokens.is_file() {
                bail!("TTS 模型缺失：{tts_dir:?} 下需要 model.onnx + tokens.txt")
            }
            // 多语言 kokoro 需要 lexicon（逗号分隔，C++ 约定同 Python）
            let lexicon = if lexicon_en.is_file() && lexicon_zh.is_file() {
                Some(format!(
                    "{},{}",
                    lexicon_en.to_string_lossy(),
                    lexicon_zh.to_string_lossy()
                ))
            } else {
                None
            };
            let cfg = sherpa_onnx::OfflineTtsConfig {
                model: sherpa_onnx::OfflineTtsModelConfig {
                    kokoro: sherpa_onnx::OfflineTtsKokoroModelConfig {
                        model: Some(model.to_string_lossy().into_owned()),
                        tokens: Some(tokens.to_string_lossy().into_owned()),
                        voices: if voices.is_file() {
                            Some(voices.to_string_lossy().into_owned())
                        } else {
                            None
                        },
                        data_dir: if data_dir.is_dir() {
                            Some(data_dir.to_string_lossy().into_owned())
                        } else {
                            None
                        },
                        lexicon,
                        lang: Some("zh".into()),
                        ..Default::default()
                    },
                    num_threads: 2,
                    provider: Some("cpu".into()),
                    ..Default::default()
                },
                ..Default::default()
            };
            sherpa_onnx::OfflineTts::create(&cfg)
                .with_context(|| format!("Kokoro TTS 创建失败（模型路径: {tts_dir:?}）"))?
        };

        // 中文女声 sid：zf_xiaobei=45 / xiaoni=46 / xiaoxiao=47 / xiaoyi=48（kokoro-multi-lang 官方映射）。
        // 默认 45（小北，温柔女声）；可用环境变量 XIAOBAI_TTS_SID 覆盖。
        let tts_sid: i32 = std::env::var("XIAOBAI_TTS_SID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(45);

        Ok(Self {
            recognizer,
            tts,
            tts_sid,
            model_root: models_dir.to_path_buf(),
        })
    }

    /// 读 WAV 文件并识别（自动重采样到 16k）。用于无麦克风的确定性 ASR 验证。
    pub fn recognize_wav_file(&self, path: &Path) -> Result<String> {
        let wave = sherpa_onnx::Wave::read(&path.to_string_lossy())
            .with_context(|| format!("无法读取 WAV: {path:?}"))?;
        let sr = wave.sample_rate();
        let samples_f32 = wave.samples();
        // 转 i16（sherpa Wave 已是 f32 mono）
        let pcm: Vec<i16> = samples_f32
            .iter()
            .map(|v| (v * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
        let pcm16 = if sr == 16000 {
            pcm
        } else {
            // 重采样：先转回 f32 比例
            let f: Vec<f32> = pcm.iter().map(|v| *v as f32 / 32768.0).collect();
            let rs = resample_to_i16(&f, sr as u32, 16000);
            rs
        };
        Ok(self.recognize(&pcm16))
    }

    /// 识别一段 16k mono 音频（i16），返回识别文本。
    pub fn recognize(&self, pcm: &[i16]) -> String {
        if pcm.is_empty() {
            return String::new();
        }
        let samples: Vec<f32> = pcm.iter().map(|v| *v as f32 / 32768.0).collect();
        let stream = self.recognizer.create_stream();
        // 分块喂（960 样本 = 60ms @16k）
        let chunk = 960usize;
        for c in samples.chunks(chunk) {
            stream.accept_waveform(16000, c);
            while self.recognizer.is_ready(&stream) {
                self.recognizer.decode(&stream);
            }
        }
        stream.input_finished();
        while self.recognizer.is_ready(&stream) {
            self.recognizer.decode(&stream);
        }
        let text = self
            .recognizer
            .get_result(&stream)
            .map(|r| r.text)
            .unwrap_or_default();
        self.recognizer.reset(&stream);
        text.trim().to_string()
    }

    /// 用 Kokoro 合成文本 → f32 samples（返回 sample_rate）。
    pub fn synthesize(&self, text: &str) -> Result<(Vec<f32>, i32)> {
        let text = text.trim();
        if text.is_empty() {
            bail!("合成文本为空");
        }
        let cfg = sherpa_onnx::GenerationConfig {
            sid: self.tts_sid,
            speed: 1.0,
            ..Default::default()
        };
        let audio = self
            .tts
            .generate_with_config(text, &cfg, None::<fn(&[f32], f32) -> bool>)
            .context("Kokoro 合成失败")?;
        let samples = audio.samples().to_vec();
        let sr = audio.sample_rate();
        if samples.is_empty() {
            bail!("Kokoro 合成结果为空");
        }
        Ok((samples, sr))
    }

    /// 合成并保存为 WAV（sherpa-onnx 自带 wav writer）。
    pub fn synthesize_to_wav(&self, text: &str, out_path: &Path) -> Result<i32> {
        let (samples, sr) = self.synthesize(text)?;
        let ok = sherpa_onnx::write(
            &out_path.to_string_lossy(),
            &samples,
            sr,
        );
        if !ok {
            bail!("WAV 写入失败: {out_path:?}");
        }
        Ok(sr)
    }

    /// 播放一段音频（阻塞直到播完）。
    pub fn play(&self, samples: &[f32], sample_rate: i32) -> Result<()> {
        let (_stream, handle) = rodio::OutputStream::try_default()
            .context("无法打开音频输出设备")?;
        let sink = rodio::Sink::try_new(&handle)
            .context("无法创建播放 sink")?;
        let buf = rodio::buffer::SamplesBuffer::new(1, sample_rate as u32, samples.to_vec());
        sink.append(buf);
        sink.sleep_until_end();
        Ok(())
    }

    /// 合成并播放（阻塞）。
    pub fn speak(&self, text: &str) -> Result<()> {
        let (samples, sr) = self.synthesize(text)?;
        self.play(&samples, sr)
    }

    /// 列出可用说话人数（调试用）。
    pub fn speaker_count(&self) -> i32 {
        self.tts.num_speakers()
    }
}

// ================================================================ 工具 ========

/// 多路径探测模型根目录（`models/voice/`，含 asr-paraformer-streaming + tts-kokoro）。
/// 顺序：环境变量 > 当前目录 > exe 相对 > 仓库路径 > 用户目录。
pub fn locate_models_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(env) = std::env::var("XIAOBAI_MODELS_DIR") {
        if !env.trim().is_empty() {
            candidates.push(PathBuf::from(env.trim()));
        }
    }

    // 仓库内语音模型根（dev 工作目录）
    candidates.push(PathBuf::from("projects/xiaobai_voice/models/voice"));
    candidates.push(PathBuf::from("models/voice"));
    candidates.push(PathBuf::from("models"));

    // exe 相对路径（release 包形态）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("models/voice"));
            candidates.push(dir.join("models"));
        }
    }

    // 用户目录
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        candidates.push(PathBuf::from(&home).join(".mox/models/voice"));
        candidates.push(PathBuf::from(&home).join(".mox/models"));
    }

    for c in candidates {
        if c.join("asr-paraformer-streaming").is_dir() || c.join("tts-kokoro").is_dir() {
            return Some(c);
        }
    }
    None
}

/// 简单工具：静音/能量检测。返回最大振幅（0..1）。
pub fn peak_level(pcm: &[i16]) -> f32 {
    pcm.iter()
        .map(|v| (v.unsigned_abs() as f32) / 32768.0)
        .fold(0.0f32, f32::max)
}

/// 判断录音是否基本为静音（可用来丢弃无效录音）。
pub fn is_silent(pcm: &[i16], threshold: f32) -> bool {
    peak_level(pcm) < threshold
}

// ================================================================ 错误 ========

/// 语音引擎错误类型（统一包装）。
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("模型缺失: {0}")]
    ModelMissing(String),
    #[error("引擎创建失败: {0}")]
    EngineInit(String),
    #[error("设备错误: {0}")]
    Device(String),
    #[error("合成失败: {0}")]
    Synthesis(String),
}

pub type VoiceResult<T> = std::result::Result<T, VoiceError>;

impl From<anyhow::Error> for VoiceError {
    fn from(e: anyhow::Error) -> Self {
        VoiceError::EngineInit(e.to_string())
    }
}
