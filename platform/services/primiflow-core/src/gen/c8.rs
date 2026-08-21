//! 代码骨架 · 由关联图谱自动生成（primiflow_core::assoc::primiflow_seed）
//! 溯源链路: R1 → F4 → B1 → A1 → T0 → C8
//! 数据设计: S2(Conversation)
//! 说明: 语音转写客户端（离线 Mock 实现，可替换为真实 ASR 服务）。
//! 规格: primiflow/SPEC.md（§6 模块 / §10 DoD）

use std::path::Path;

/// 转写结果
#[derive(Debug, Clone, PartialEq)]
pub struct AsrResult {
    /// 识别出的文本
    pub text: String,
    /// 置信度 0..1
    pub confidence: f32,
    /// 耗时估计（毫秒），用于调度代价估算
    pub duration_ms: u64,
}

/// 转写器统一接口：真实环境下可替换为云端 ASR / vLLM 本地模型。
/// 主链路只依赖此 trait，与具体实现解耦。
pub trait Transcriber {
    /// 从音频文件路径转写为文本
    fn transcribe(&self, audio_path: &str) -> anyhow::Result<AsrResult>;
}

/// 受支持的音频格式（域白名单，超域拒绝以收敛幻觉面）
const SUPPORTED_EXT: &[&str] = &["wav", "mp3", "pcm", "m4a", "flac"];

/// 本地离线 Mock 转写器：不依赖网络与模型，按文件名规范化产出确定性文本。
/// 用于「先跑通主链路（用 Mock，离线可跑）」的 P0 切片。
#[derive(Debug, Default)]
pub struct LocalMockAsr {
    /// 命中文件名关键词 → 直接返回预置业务语句（模拟识别结果）
    pub vocabulary: Vec<(String, String)>,
}

impl LocalMockAsr {
    pub fn new() -> Self {
        // 预置几条业务短语，覆盖常见需求输入，方便端到端演示
        let vocabulary = vec![
            ("报表".into(), "请帮我做一个电商月度经营分析报告，包含销售数据抓取、清洗对账和图表生成。".into()),
            ("审批".into(), "做一个报销审批流程，需要人工审批节点和入库节点。".into()),
            ("爬虫".into(), "抓取公开网页新闻并存储到数据库，生成摘要。".into()),
        ];
        Self { vocabulary }
    }
}

impl Transcriber for LocalMockAsr {
    fn transcribe(&self, audio_path: &str) -> anyhow::Result<AsrResult> {
        let path = Path::new(audio_path);
        // 1) 格式白名单校验（超域拒绝）
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .ok_or_else(|| anyhow::anyhow!("无法识别音频扩展名: {}", audio_path))?;
        if !SUPPORTED_EXT.contains(&ext.as_str()) {
            anyhow::bail!("不支持的音频格式 .{}（仅支持 {:?}）", ext, SUPPORTED_EXT);
        }
        // 2) 文件名（去扩展名）作为「语义种子」做确定性转写
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let (text, confidence) = self
            .vocabulary
            .iter()
            .find(|(kw, _)| stem.contains(kw))
            .map(|(_, t)| (t.clone(), 0.92_f32))
            .unwrap_or_else(|| {
                // 无命中：回退为确定性占位文本（保留文件名痕迹，便于追溯）
                (
                    format!("（语音输入）{} 的需求，请自动拆解为任务并生成拓扑。", stem),
                    0.6_f32,
                )
            });
        // 3) 文本规范化：去除首尾空白、合并多余空白、统一全角标点
        let text = normalize(&text);
        let duration_ms = (stem.len() as u64 + 8) * 350;
        Ok(AsrResult { text, confidence, duration_ms })
    }
}

/// 文本规范化：折叠空白、统一中文标点、移除标点前的多余空格
fn normalize(s: &str) -> String {
    let s = s.trim();
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        let is_ws = c.is_whitespace();
        if is_ws {
            if !prev_ws {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            // 统一常见全角/半角标点
            let c = match c {
                '，' => ',',
                '。' => '.',
                '：' => ':',
                '；' => ';',
                _ => c,
            };
            // 标点前不应有空格（中文排版）
            if matches!(c, ',' | '.' | ':' | ';') {
                while out.ends_with(' ') {
                    out.pop();
                }
            }
            out.push(c);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}

/// 语音转写客户端：对 `Transcriber` 的轻量门面，主链路统一入口。
#[derive(Debug, Default)]
pub struct AsrClient {
    pub engine: LocalMockAsr,
}

impl AsrClient {
    pub fn new() -> Self {
        Self { engine: LocalMockAsr::new() }
    }
    /// 便捷封装：用内置 LocalMockAsr 转写
    pub fn asr_transcribe(&self, audio_path: &str) -> anyhow::Result<AsrResult> {
        self.engine.transcribe(audio_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_format() {
        let client = AsrClient::new();
        let r = client.asr_transcribe("meeting.txt");
        assert!(r.is_err(), "非音频格式应被拒绝");
    }

    #[test]
    fn transcribes_vocabulary_hit() {
        let client = AsrClient::new();
        let r = client.asr_transcribe("报销审批录音.mp3").unwrap();
        assert!(r.text.contains("审批"));
        assert!(r.confidence > 0.8);
    }

    #[test]
    fn transcribes_unknown_with_fallback() {
        let client = AsrClient::new();
        let r = client.asr_transcribe("项目alpha.wav").unwrap();
        assert!(r.text.contains("项目alpha"));
        assert!(r.confidence < 0.8);
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize("  你好  ，  世界  。 "), "你好, 世界.");
    }
}
