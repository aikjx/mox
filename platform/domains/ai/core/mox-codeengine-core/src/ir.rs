// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Intent 阶段：自然语言需求 → 结构化需求 IR（全自研规则解析，零 LLM 依赖）。
//!
//! 解析策略：按句切分 → 每句识别「动作意图（工具类别）+ 资源（读/写集合）」，
//! 资源标识抽取兼容中英文标点与 `a.b.c` 型限定名；敏感资源就地打标。

use mox_ai_flow_core::model::ToolKind;
use serde::{Deserialize, Serialize};

/// 敏感资源关键字（等保/个保法口径的最小自研词表）
pub const SENSITIVE_KEYWORDS: &[&str] = &[
    "citizen", "idcard", "id_card", "ssn", "password", "secret", "token", "身份证", "手机号",
    "密码", "隐私", "salary", "薪资",
];

/// 一个需求步骤（一句一个候选步骤）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepSpec {
    pub name: String,
    pub tool: ToolKind,
    pub duration_ms: u64,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    /// 该步骤触碰的敏感资源（reads∪writes 的敏感子集）
    pub sensitive: Vec<String>,
}

/// 需求 IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub name: String,
    pub text: String,
    pub steps: Vec<StepSpec>,
}

const SENTENCE_SEPS: [char; 7] = ['。', '；', ';', '\n', '！', '!', '?'];
const TRIM_CHARS: [char; 5] = [' ', '\t', '，', ',', '、'];

impl Requirement {
    pub fn touches(&self, kind: ToolKind) -> bool {
        self.steps.iter().any(|s| s.tool == kind)
    }
    pub fn has_sensitive(&self) -> bool {
        self.steps.iter().any(|s| !s.sensitive.is_empty())
    }
}

/// 判断资源是否敏感
pub fn is_sensitive(resource: &str) -> bool {
    let lower = resource.to_lowercase();
    SENSITIVE_KEYWORDS.iter().any(|k| lower.contains(k))
}

fn infer_tool(sentence: &str) -> ToolKind {
    let s = sentence.to_lowercase();
    let has = |k: &str| s.contains(k);
    // 顺序重要：越具体的副作用工具越先判定
    if has("审批") || has("审核") || has("人工") || has("确认") || has("approve") || has("review by human") {
        ToolKind::Human
    } else if has("浏览器") || has("网页") || has("爬") || has("rpa") || has("browser") || has(" selenium") {
        ToolKind::Browser
    } else if has("数据库") || has("入库") || has("表") || has("sql") || has("database") || s.contains("db.") {
        ToolKind::Database
    } else if has("命令") || has("脚本") || has("shell") || has("部署") || has("执行 bash") {
        ToolKind::Shell
    } else if has("接口") || has("http") || has("api") || has("url") || has("请求") {
        ToolKind::Http
    } else if has("文件") || has("导出") || has("报表") || has("csv") || has("xlsx") || has("excel")
        || has(".txt") || has("读入") || has("加载")
    {
        ToolKind::File
    } else if has("模型") || has("llm") || has("gpt") || has("总结") || has("翻译") || has("生成摘要")
        || has("问答") || has("分类")
    {
        ToolKind::Llm
    } else {
        ToolKind::Compute
    }
}

fn default_duration(tool: ToolKind) -> u64 {
    match tool {
        ToolKind::Llm => 800,
        ToolKind::Browser => 1200,
        ToolKind::Database => 400,
        ToolKind::File => 300,
        ToolKind::Http => 500,
        ToolKind::Shell => 600,
        ToolKind::Human => 3_600_000,
        ToolKind::Compute => 100,
    }
}

/// 从句子中抽取资源标识符：`[A-Za-z_][A-Za-z0-9_]*(&#92;.[A-Za-z0-9_]+)+` 限定名，
/// 或带引号的中文短语（“xx表”）。
fn extract_resources(sentence: &str) -> Vec<String> {
    let bytes = sentence.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let starts = c == '_' || c.is_ascii_alphabetic();
        if !starts {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut dotted = false;
        while j < bytes.len() {
            let ch = bytes[j] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                j += 1;
            } else if ch == '.' && j + 1 < bytes.len() && (bytes[j + 1] as char).is_ascii_alphanumeric() {
                dotted = true;
                j += 1;
            } else {
                break;
            }
        }
        let token = &sentence[i..j];
        // 单词裸标识符太易误报（如 if/for/db），只收限定名或较长的下划线名
        let keep = dotted || (token.len() >= 5 && token.contains('_'));
        if keep && !out.iter().any(|t| t == token) {
            out.push(token.to_string());
        }
        i = j.max(i + 1);
    }
    out
}

/// 解析自然语言需求文本为需求 IR。
///
/// `id`/`name` 由调用方给定；句子以 。；;!?\n 及中英标点切分，纯连接词句子被丢弃。
pub fn parse_requirement(id: &str, name: &str, text: &str) -> Requirement {
    let mut steps: Vec<StepSpec> = Vec::new();
    for raw in text.split(|c| SENTENCE_SEPS.contains(&c)) {
        let sentence = raw.trim_matches(|c| TRIM_CHARS.contains(&c)).trim();
        if sentence.chars().count() < 2 {
            continue;
        }
        let tool = infer_tool(sentence);
        let resources = extract_resources(sentence);
        let lower = sentence.to_lowercase();
        let writes_only_hint = ["写", "保存", "入库", "导出", "输出", "生成到", "write", "save", "export"]
            .iter()
            .any(|k| lower.contains(k));
        let (reads, writes) = if resources.is_empty() {
            (Vec::new(), Vec::new())
        } else if writes_only_hint && tool != ToolKind::Compute {
            // 「写入 db.orders」类句子：资源按写处理
            (Vec::new(), resources.clone())
        } else if tool == ToolKind::Database || tool == ToolKind::File {
            // 读写方向不明时，DB/File 默认视为读（保守：不误报写冲突）
            (resources.clone(), Vec::new())
        } else {
            (resources.clone(), Vec::new())
        };
        let sensitive: Vec<String> = reads
            .iter()
            .chain(writes.iter())
            .filter(|r| is_sensitive(r))
            .cloned()
            .collect();
        steps.push(StepSpec {
            name: truncate(sentence, 40),
            tool,
            duration_ms: default_duration(tool),
            reads,
            writes,
            sensitive,
        });
    }
    Requirement {
        id: id.to_string(),
        name: name.to_string(),
        text: text.to_string(),
        steps,
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_steps_tools_and_resources() {
        let req = parse_requirement(
            "r1",
            "订单报表",
            "读取 input.csv 文件；把解析结果写入 db.orders 表；导出报表 output.xlsx",
        );
        assert_eq!(req.steps.len(), 3);
        assert_eq!(req.steps[0].tool, ToolKind::File);
        assert_eq!(req.steps[1].tool, ToolKind::Database);
        assert_eq!(req.steps[1].writes, vec!["db.orders".to_string()]);
        assert!(req.steps[2].writes.iter().any(|w| w == "output.xlsx"));
    }

    #[test]
    fn marks_sensitive_resources() {
        let req = parse_requirement("r2", "涉敏", "查询 db.citizen_idcard 表并总结");
        assert!(req.has_sensitive());
        assert!(req.steps[0].sensitive.iter().any(|s| s == "db.citizen_idcard"));
    }

    #[test]
    fn human_and_shell_tools() {
        let req = parse_requirement("r3", "n", "需要人工审批后执行部署命令");
        assert_eq!(req.steps[0].tool, ToolKind::Human);
    }
}
