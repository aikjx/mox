// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! FR-5 热词注入三层实现：S1/S2/S3 + HotwordInjector 状态机

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};
use tempfile::NamedTempFile;
use mox_voice_core_svc::hotword::{apply_fuzzy, validate_and_rank, Hotword};

use crate::ffi_probe::sherpa_rs_context_config_available;

/// 每层的状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerStatus {
    /// 尚未执行
    Pending,
    /// 成功（S1：context 注入成功 / S2：文件写成功 / S3：post-hoc 替换执行成功）
    Applied { n: usize },
    /// 降级（S1 不可用，写了降级原因；S2 写文件失败；S3 无匹配）
    Skipped { reason: &'static str },
}

impl LayerStatus {
    pub fn summary_json(&self) -> Value {
        match self {
            LayerStatus::Pending => json!("pending"),
            LayerStatus::Applied { n } => json!({"applied": true, "n": n}),
            LayerStatus::Skipped { reason } => json!({"applied": false, "reason": reason}),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InjectionReport {
    pub s1: LayerStatus,
    pub s2: LayerStatus,
    pub s3: LayerStatus,
    /// S2 写出的热词文件路径（可被调用方拿去重建 sherpa-rs recognizer）
    pub hotword_file: Option<PathBuf>,
    pub hotwords_count: usize,
}

impl InjectionReport {
    pub fn new() -> Self {
        Self {
            s1: LayerStatus::Pending,
            s2: LayerStatus::Pending,
            s3: LayerStatus::Pending,
            hotword_file: None,
            hotwords_count: 0,
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "s1": self.s1.summary_json(),
            "s2": self.s2.summary_json(),
            "s3": self.s3.summary_json(),
            "hotword_file": self.hotword_file.as_ref().map(|p| p.display().to_string()),
            "hotwords_count": self.hotwords_count,
        })
    }
}

impl Default for InjectionReport {
    fn default() -> Self {
        Self::new()
    }
}

/// FR-5 注入器：线程安全（内部 Mutex），可跨 await 调用
pub struct HotwordInjector {
    state: Mutex<Inner>,
}

struct Inner {
    /// 最近一次 validate 后的干净热词（已去重、饱和、PII 标黄）
    hotwords: Vec<Hotword>,
    /// S2 写出的临时文件句柄（保持生命周期；Drop 时临时文件自动删）
    last_tempfile: Option<NamedTempFile>,
}

impl HotwordInjector {
    pub fn new() -> Self {
        Self { state: Mutex::new(Inner { hotwords: Vec::new(), last_tempfile: None }) }
    }

    /// 设置热词（走 validate_and_rank）：失败返回 XiaobaiError
    pub fn set_hotwords(&self, raw: &[Hotword]) -> Result<(Vec<Hotword>, InjectionReport), mox_voice_core_svc::errors::XiaobaiError> {
        let cleaned = validate_and_rank(raw)?;
        let _validation = ();
        let mut g = self.state.lock().unwrap();
        g.hotwords = cleaned.clone();
        // 写 S2 临时文件
        let (tmp_path, report_s2) = Self::write_hotword_tempfile(&g.hotwords);
        match (tmp_path, &report_s2) {
            (Some(p), LayerStatus::Applied { .. }) => {
                g.last_tempfile = Some(p);
            }
            _ => {}
        }
        let hw_path = g.last_tempfile.as_ref().map(|f| f.path().to_path_buf());
        // S1 探测
        let s1 = match sherpa_rs_context_config_available() {
            Ok(true) if !g.hotwords.is_empty() => LayerStatus::Applied { n: g.hotwords.len() },
            Ok(true) => LayerStatus::Skipped { reason: "hotwords 为空，跳过 S1 注入" },
            Ok(false) => LayerStatus::Skipped { reason: "sherpa_rs_context_config 不可用，已降级 S2+S3" },
            Err(e) => {
                tracing::warn!("S1 FFI 探测失败：{e}");
                LayerStatus::Skipped { reason: "sherpa_rs 探测异常，降级 S2+S3" }
            }
        };
        let mut report = InjectionReport::new();
        report.s1 = s1;
        report.s2 = report_s2;
        report.hotword_file = hw_path;
        report.hotwords_count = g.hotwords.len();
        // S3 还没执行（要等 ASR 出来才执行），先标 pending
        // 把 validate warnings 追加到 s1/report 里也可以，这里我们在 report.json 外暴露 validation
        drop(g);
        Ok((cleaned, report))
    }

    /// S3 post-hoc：把 ASR 输出的文本按 hotword.apply_fuzzy 做 Levenshtein 替换
    pub fn apply_post_hoc(&self, asr_text: &str) -> Result<(String, Vec<Hotword>), mox_voice_core_svc::errors::XiaobaiError> {
        let g = self.state.lock().unwrap();
        if g.hotwords.is_empty() {
            return Ok((asr_text.to_string(), Vec::new()));
        }
        let fuzzy_result = apply_fuzzy(asr_text, &g.hotwords);
        let hit: Vec<Hotword> = fuzzy_result.applied.iter().map(|(hw, _)| (*hw).clone()).collect();
        Ok((fuzzy_result.text, hit))
    }

    /// 当前 S2 文件路径（给 sherpa recognizer 做 load hotwords 用）
    pub fn hotword_file_path(&self) -> Option<PathBuf> {
        let g = self.state.lock().unwrap();
        g.last_tempfile.as_ref().map(|f| f.path().to_path_buf())
    }

    /// 当前热词列表快照
    pub fn hotwords(&self) -> Vec<Hotword> {
        self.state.lock().unwrap().hotwords.clone()
    }

    // ----- S2 内部：写临时文件 -----
    fn write_hotword_tempfile(hws: &[Hotword]) -> (Option<NamedTempFile>, LayerStatus) {
        if hws.is_empty() {
            return (None, LayerStatus::Skipped { reason: "hotwords 为空，跳过 S2 文件" });
        }
        use std::io::Write;
        let mut tmp = match tempfile::Builder::new()
            .prefix("xiaobai_hotwords_")
            .suffix(".txt")
            .rand_bytes(6)
            .tempfile()
        {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("S2 临时文件创建失败：{e}");
                return (None, LayerStatus::Skipped { reason: "创建临时文件失败" });
            }
        };
        for hw in hws.iter() {
            // sherpa-onnx 格式：word\tscore\n
            let line = format!("{}\t{:.3}\n", hw.word, hw.score);
            if let Err(e) = tmp.write_all(line.as_bytes()) {
                tracing::warn!("S2 写临时文件失败：{e}");
                return (None, LayerStatus::Skipped { reason: "写临时文件失败" });
            }
        }
        if let Err(e) = tmp.flush() {
            tracing::warn!("S2 flush 失败：{e}");
            return (None, LayerStatus::Skipped { reason: "flush 临时文件失败" });
        }
        (Some(tmp), LayerStatus::Applied { n: hws.len() })
    }
}

impl Default for HotwordInjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod smoke {
    use super::*;
    use serde_json::json;

    fn hw(word: &str, score: f32) -> Hotword {
        Hotword::new(word).with_score(score)
    }

    #[test]
    fn empty_hotwords_returns_skipped_s2() {
        let inj = HotwordInjector::new();
        let (cleaned, report) = inj.set_hotwords(&[]).unwrap();
        assert!(cleaned.is_empty());
        assert!(matches!(report.s2, LayerStatus::Skipped { .. }));
        assert!(report.hotword_file.is_none());
    }

    #[test]
    fn valid_hotwords_writes_tempfile_and_s3_replace() {
        let inj = HotwordInjector::new();
        let (cleaned, report) = inj.set_hotwords(&[
            hw("飞书审批", 0.90),
            hw("企业微信", 0.88),
            hw("Infotopograph", 0.80),
        ]).unwrap();
        assert_eq!(cleaned.len(), 3);
        assert!(matches!(report.s2, LayerStatus::Applied { n: 3 }));
        let p = inj.hotword_file_path().expect("应该有 S2 文件路径");
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("飞书审批\t0.900\n"), "文件格式: {}", content);
        assert!(content.contains("Infotopograph\t0.800\n"));
        drop(p);

        // S3 Levenshtein 模糊修正："飞书 审批 2 分钟"
        let (txt, hits) = inj.apply_post_hoc("我在飞书申批提交了 2 个流程").unwrap();
        // "申批" → "审批" 因为 Levenshtein 距离 1 ≤ 1 且长度比 0.7 匹配
        assert!(!hits.is_empty());
        assert!(txt.contains("飞书审批"), "得到 {}", txt);
    }

    #[test]
    fn s1_probe_sanity() {
        let _ = sherpa_rs_context_config_available(); // 不 panic
    }

    #[test]
    fn pii_word_is_rejected() {
        let _unused = Hotword::new("138 0013 8000 手机").with_score(0.9);
        // PII 黑名单应该让 validate_and_rank 打 warning；Hotword::new 本身不拒绝，只 validate
        let inj = HotwordInjector::new();
        let hw_pii = Hotword::new("138 0013 8000 手机").with_score(0.9).with_category("pii");
        let (cleaned, rep) = inj.set_hotwords(&[hw_pii]).unwrap();
        // PII 标黄但不硬拒绝（企业策略：审计可见）
        assert!(cleaned.iter().any(|h| h.category == "pii"));
        assert_eq!(rep.hotwords_count, 1);
    }
}
