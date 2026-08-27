// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX Voice Domain API — trait contracts for ASR, TTS, Intent, DSP, Operator.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceApiError {
    #[error("asr error: {0}")]
    Asr(String),
    #[error("tts error: {0}")]
    Tts(String),
    #[error("intent error: {0}")]
    Intent(String),
    #[error("dsp error: {0}")]
    Dsp(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type VoiceApiResult<T> = Result<T, VoiceApiError>;

// ─── ASR ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrResult {
    pub text: String,
    pub confidence: f64,
    pub language: String,
    pub segments: Vec<AsrSegment>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub speaker: Option<String>,
}

#[async_trait]
pub trait SpeechRecognizer: Send + Sync {
    async fn recognize(&self, audio: &[u8], format: &str) -> VoiceApiResult<AsrResult>;
    async fn recognize_stream(&self, audio_stream: tokio::sync::mpsc::Receiver<Vec<u8>>) -> VoiceApiResult<AsrResult>;
    fn supported_formats(&self) -> Vec<String>;
}

// ─── TTS ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: String,
    pub language: String,
    pub speed: f32,
    pub pitch: f32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub audio: Vec<u8>,
    pub format: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
}

#[async_trait]
pub trait SpeechSynthesizer: Send + Sync {
    async fn synthesize(&self, request: TtsRequest) -> VoiceApiResult<TtsResult>;
    async fn list_voices(&self, language: Option<&str>) -> VoiceApiResult<Vec<String>>;
}

// ─── Voice Intent ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceIntentResult {
    pub intent: String,
    pub confidence: f64,
    pub slots: std::collections::HashMap<String, String>,
    pub raw_text: String,
}

#[async_trait]
pub trait VoiceIntentRecognizer: Send + Sync {
    async fn recognize(&self, text: &str) -> VoiceApiResult<VoiceIntentResult>;
    async fn recognize_from_audio(&self, audio: &[u8]) -> VoiceApiResult<VoiceIntentResult>;
}

// ─── DSP ───

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DspFilterType { LowPass, HighPass, BandPass, BandStop }

pub trait AudioProcessor: Send + Sync {
    fn resample(&self, audio: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32>;
    fn filter(&self, audio: &[f32], filter_type: DspFilterType, cutoff: f32, sample_rate: u32) -> Vec<f32>;
    fn normalize(&self, audio: &[f32], target_peak: f32) -> Vec<f32>;
    fn noise_reduce(&self, audio: &[f32], noise_profile: &[f32]) -> Vec<f32>;
    fn vad(&self, audio: &[f32], sample_rate: u32) -> Vec<(usize, usize)>;
}

// ─── Voice Operator / Session ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSession {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub status: String,
    pub created_at: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[async_trait]
pub trait VoiceSessionManager: Send + Sync {
    async fn create_session(&self, tenant_id: &str, user_id: &str) -> VoiceApiResult<VoiceSession>;
    async fn get_session(&self, id: &str) -> VoiceApiResult<Option<VoiceSession>>;
    async fn end_session(&self, id: &str) -> VoiceApiResult<bool>;
    async fn list_sessions(&self, tenant_id: &str) -> VoiceApiResult<Vec<VoiceSession>>;
}
