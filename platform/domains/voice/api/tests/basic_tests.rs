// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_voice_api::*;

// ─── VoiceApiError / VoiceApiResult ───

#[test]
fn error_variants_construct_and_display() {
    let err = VoiceApiError::Asr("model not loaded".into());
    let msg = format!("{}", err);
    assert!(msg.contains("asr error"));
    assert!(msg.contains("model not loaded"));

    let err = VoiceApiError::Tts("voice not found".into());
    assert!(format!("{}", err).contains("tts error"));

    let err = VoiceApiError::Intent("no match".into());
    assert!(format!("{}", err).contains("intent error"));

    let err = VoiceApiError::Dsp("invalid format".into());
    assert!(format!("{}", err).contains("dsp error"));

    let err = VoiceApiError::Internal("oops".into());
    assert!(format!("{}", err).contains("internal"));
}

#[test]
fn result_type_alias_works() {
    let ok: VoiceApiResult<i32> = Ok(42);
    assert_eq!(ok.unwrap(), 42);
    let err: VoiceApiResult<i32> = Err(VoiceApiError::Internal("test".into()));
    assert!(err.is_err());
}

// ─── AsrResult / AsrSegment ───

#[test]
fn asr_result_default_construction() {
    let result = AsrResult {
        text: "你好世界".into(),
        confidence: 0.95,
        language: "zh".into(),
        segments: vec![],
        duration_ms: 1500,
    };
    assert_eq!(result.text, "你好世界");
    assert!((result.confidence - 0.95).abs() < f64::EPSILON);
    assert_eq!(result.language, "zh");
    assert!(result.segments.is_empty());
    assert_eq!(result.duration_ms, 1500);
}

#[test]
fn asr_segment_with_speaker() {
    let seg = AsrSegment {
        start_ms: 0,
        end_ms: 1000,
        text: "hello".into(),
        speaker: Some("speaker_1".into()),
    };
    assert_eq!(seg.start_ms, 0);
    assert_eq!(seg.end_ms, 1000);
    assert_eq!(seg.text, "hello");
    assert_eq!(seg.speaker.as_deref(), Some("speaker_1"));

    let seg2 = AsrSegment {
        start_ms: 0,
        end_ms: 500,
        text: "world".into(),
        speaker: None,
    };
    assert!(seg2.speaker.is_none());
}

#[test]
fn asr_result_clone_and_debug() {
    let result = AsrResult {
        text: "test".into(),
        confidence: 0.8,
        language: "en".into(),
        segments: vec![AsrSegment {
            start_ms: 0,
            end_ms: 100,
            text: "test".into(),
            speaker: None,
        }],
        duration_ms: 100,
    };
    let cloned = result.clone();
    assert_eq!(cloned.text, result.text);
    assert_eq!(cloned.segments.len(), 1);
    let dbg = format!("{:?}", result);
    assert!(dbg.contains("AsrResult"));
}

// ─── TtsRequest / TtsResult ───

#[test]
fn tts_request_construction() {
    let req = TtsRequest {
        text: "你好".into(),
        voice: "default".into(),
        language: "zh".into(),
        speed: 1.0,
        pitch: 1.0,
        format: "wav".into(),
    };
    assert_eq!(req.text, "你好");
    assert_eq!(req.speed, 1.0);
    assert_eq!(req.format, "wav");
}

#[test]
fn tts_result_construction() {
    let result = TtsResult {
        audio: vec![0, 1, 2, 3],
        format: "pcm".into(),
        duration_ms: 2000,
        sample_rate: 22050,
    };
    assert_eq!(result.audio.len(), 4);
    assert_eq!(result.sample_rate, 22050);
    assert_eq!(result.duration_ms, 2000);
}

// ─── VoiceIntentResult ───

#[test]
fn voice_intent_result_construction() {
    let mut slots = std::collections::HashMap::new();
    slots.insert("app".into(), "chrome".into());
    let result = VoiceIntentResult {
        intent: "open_app".into(),
        confidence: 0.92,
        slots,
        raw_text: "打开浏览器".into(),
    };
    assert_eq!(result.intent, "open_app");
    assert_eq!(result.slots.get("app").unwrap(), "chrome");
    assert_eq!(result.raw_text, "打开浏览器");
}

// ─── DspFilterType ───

#[test]
fn dsp_filter_type_variants() {
    use DspFilterType::*;
    let types = vec![LowPass, HighPass, BandPass, BandStop];
    assert_eq!(types.len(), 4);
    // Copy + PartialEq
    let a = LowPass;
    let b = a;
    assert_eq!(a, b);
    assert_ne!(LowPass, HighPass);
}

#[test]
fn dsp_filter_type_debug() {
    let s = format!("{:?}", DspFilterType::BandPass);
    assert!(s.contains("BandPass"));
}

// ─── VoiceSession ───

#[test]
fn voice_session_construction() {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("device".into(), "desktop".into());
    let session = VoiceSession {
        id: "sess_123".into(),
        tenant_id: "tenant_001".into(),
        user_id: "user_001".into(),
        status: "active".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        metadata,
    };
    assert_eq!(session.id, "sess_123");
    assert_eq!(session.status, "active");
    assert_eq!(session.metadata.get("device").unwrap(), "desktop");
}
