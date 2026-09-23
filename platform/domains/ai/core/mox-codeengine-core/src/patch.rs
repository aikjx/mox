// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Diff-first 增量编辑（参考 2026 code agent 主流编辑格式，如 Aider SEARCH/REPLACE）：
//! 交付时若目标文件已存在（调用方提供基线），产出**最小 SEARCH/REPLACE 块**而非全量覆写，
//! 应用侧以确定性匹配（精确 → 全文唯一）落盘，歧义一律拒绝，保证可回滚可审计。

use mox_ai_flow_core::codegen::CodeBundle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const SEARCH: &str = "<<<<<<< SEARCH";
const DIVIDER: &str = "=======";
const REPLACE: &str = ">>>>>>> REPLACE";

/// 一个编辑块：path 相对代码包根目录
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditBlock {
    pub path: String,
    pub search: String,
    pub replace: String,
}

impl EditBlock {
    /// 渲染为标准 SEARCH/REPLACE 文本
    pub fn render(&self) -> String {
        format!(
            "{SEARCH} path={}\n{}{DIVIDER}\n{}{REPLACE}",
            self.path, self.search, self.replace
        )
    }
}

/// 解析 SEARCH/REPLACE 文本（容忍 CRLF 与块间空行）
pub fn parse_blocks(text: &str) -> Result<Vec<EditBlock>, String> {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if line == SEARCH || line.starts_with(&format!("{SEARCH} ")) {
            let path = line
                .strip_prefix(&format!("{SEARCH} "))
                .and_then(|rest| rest.strip_prefix("path="))
                .unwrap_or_default()
                .to_string();
            if path.is_empty() {
                return Err(format!("第 {} 行 SEARCH 块缺少 path=", i + 1));
            }
            i += 1;
            let mut search = String::new();
            while i < lines.len() && lines[i].trim_end() != DIVIDER {
                search.push_str(lines[i]);
                search.push('\n');
                i += 1;
            }
            if i >= lines.len() {
                return Err("SEARCH 块缺少 ======= 分隔线".into());
            }
            i += 1;
            let mut replace = String::new();
            while i < lines.len() && lines[i].trim_end() != REPLACE {
                replace.push_str(lines[i]);
                replace.push('\n');
                i += 1;
            }
            if i >= lines.len() {
                return Err("SEARCH 块缺少 >>>>>>> REPLACE 结束线".into());
            }
            out.push(EditBlock {
                path,
                search,
                replace,
            });
        }
        i += 1;
    }
    if out.is_empty() {
        return Err("未解析到任何 SEARCH/REPLACE 块".into());
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyStatus {
    /// 精确子串命中（唯一）
    Exact,
    /// search 与整个文件（忽略首尾空白）一致 → 全文件替换
    WholeFile,
    /// 空块（基线与目标无差异），无需变更即成功
    NoChange,
    /// 未命中
    NotFound,
    /// 命中多处，安全起见拒绝
    Ambiguous,
    /// 基线里没有该文件
    MissingFile,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub path: String,
    pub status: ApplyStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchReport {
    pub results: Vec<ApplyResult>,
    pub applied: usize,
    pub failed: usize,
}

impl PatchReport {
    pub fn ok(&self) -> bool {
        self.failed == 0
    }
}

/// 将编辑块应用到基线文件集；返回新文件集与逐块结果。
pub fn apply_blocks(
    base: &BTreeMap<String, String>,
    blocks: &[EditBlock],
) -> (BTreeMap<String, String>, PatchReport) {
    let mut files = base.clone();
    let mut results = Vec::new();
    let mut applied = 0;
    let mut failed = 0;
    for b in blocks {
        let Some(content) = files.get(&b.path).cloned() else {
            results.push(ApplyResult {
                path: b.path.clone(),
                status: ApplyStatus::MissingFile,
            });
            failed += 1;
            continue;
        };
        if b.search.trim().is_empty() && b.replace.trim().is_empty() {
            results.push(ApplyResult {
                path: b.path.clone(),
                status: ApplyStatus::NoChange,
            });
            applied += 1;
            continue;
        }
        if content.trim_end() == b.search.trim_end() && !b.search.trim().is_empty() {
            files.insert(b.path.clone(), b.replace.clone());
            results.push(ApplyResult {
                path: b.path.clone(),
                status: ApplyStatus::WholeFile,
            });
            applied += 1;
            continue;
        }
        let hits = occurrence_positions(&content, &b.search);
        let status = match hits.len() {
            0 => {
                failed += 1;
                ApplyStatus::NotFound
            }
            1 => {
                let at = hits[0];
                let mut next = String::with_capacity(content.len());
                next.push_str(&content[..at]);
                next.push_str(&b.replace);
                next.push_str(&content[at + b.search.len()..]);
                files.insert(b.path.clone(), next);
                applied += 1;
                ApplyStatus::Exact
            }
            _ => {
                failed += 1;
                ApplyStatus::Ambiguous
            }
        };
        results.push(ApplyResult {
            path: b.path.clone(),
            status,
        });
    }
    (
        files,
        PatchReport {
            results,
            applied,
            failed,
        },
    )
}

fn occurrence_positions(hay: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let hb = hay.as_bytes();
    let nb = needle.as_bytes();
    let mut pos = Vec::new();
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        if &hb[i..i + nb.len()] == nb {
            pos.push(i);
            i += nb.len(); // 不重叠计数
        } else {
            i += 1;
        }
    }
    pos
}

/// old→new 生成压缩编辑块：剥掉公共前后缀行，只保留最小差异区。
pub fn make_block(path: &str, old: &str, new: &str) -> EditBlock {
    let o: Vec<&str> = old.lines().collect();
    let n: Vec<&str> = new.lines().collect();
    let mut pre = 0;
    while pre < o.len() && pre < n.len() && o[pre] == n[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < o.len() - pre && suf < n.len() - pre && o[o.len() - 1 - suf] == n[n.len() - 1 - suf] {
        suf += 1;
    }
    let join = |v: &[&str]| -> String {
        if v.is_empty() {
            String::new()
        } else {
            let mut s = v.join("\n");
            s.push('\n');
            s
        }
    };
    EditBlock {
        path: path.to_string(),
        search: join(&o[pre..o.len() - suf]),
        replace: join(&n[pre..n.len() - suf]),
    }
}

/// 交付增强：对代码包中与基线同名的文件产出压缩编辑块清单
/// （diff-first：已有文件给补丁，新文件保持全量）。
pub fn diff_against(bundle: &CodeBundle, base: &BTreeMap<String, String>) -> Vec<EditBlock> {
    bundle
        .files
        .iter()
        .filter(|f| base.contains_key(&f.path))
        .map(|f| make_block(&f.path, &base[&f.path], &f.content))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(
            "generated/tasks.py".to_string(),
            "import os\n\ndef op_s0():\n    return 1\n\ndef op_s1():\n    return 2\n".to_string(),
        );
        m
    }

    #[test]
    fn roundtrip_render_parse() {
        let b = EditBlock {
            path: "generated/tasks.py".into(),
            search: "    return 1\n".into(),
            replace: "    return 100\n".into(),
        };
        let parsed = parse_blocks(&b.render()).expect("应解析出 1 块");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], b);
    }

    #[test]
    fn applies_unique_and_rejects_ambiguous() {
        let blocks = vec![
            EditBlock {
                path: "generated/tasks.py".into(),
                search: "    return 1\n".into(),
                replace: "    return 100\n".into(),
            },
            EditBlock {
                path: "generated/tasks.py".into(),
                search: "import os\n".into(),
                replace: "import sys\nimport os\n".into(),
            },
            EditBlock {
                path: "nope.py".into(),
                search: "x\n".into(),
                replace: "y\n".into(),
            },
        ];
        let (files, report) = apply_blocks(&base(), &blocks);
        assert!(files["generated/tasks.py"].contains("return 100"));
        assert!(files["generated/tasks.py"].contains("import sys"));
        assert_eq!(report.applied, 2);
        assert_eq!(report.failed, 1);
        assert_eq!(report.results[2].status, ApplyStatus::MissingFile);
    }

    #[test]
    fn ambiguous_hits_are_refused_for_safety() {
        let b = base();
        let dup = "generated/dup.py".to_string();
        let mut m = b.clone();
        m.insert(dup.clone(), "a\nb\na\n".to_string());
        let (_, report) = apply_blocks(
            &m,
            &[EditBlock {
                path: dup,
                search: "a\n".into(),
                replace: "z\n".into(),
            }],
        );
        assert_eq!(report.results[0].status, ApplyStatus::Ambiguous);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn empty_block_is_noop_success() {
        // 无差异压缩后的空块不得判失败（make_block 对 identical 内容产出空块）
        let b = base();
        let path = "generated/tasks.py".to_string();
        let blk = make_block(&path, &b[&path], &b[&path]);
        assert!(blk.search.is_empty() && blk.replace.is_empty());
        let (files, report) = apply_blocks(&b, std::slice::from_ref(&blk));
        assert!(report.ok());
        assert_eq!(report.results[0].status, ApplyStatus::NoChange);
        assert_eq!(files[&path], b[&path]);
    }

    #[test]
    fn make_block_compacts_common_context() {
        let old = "head\nmid1\ntail\n";
        let new = "head\nmid2\ntail\n";
        let blk = make_block("f.py", old, new);
        assert_eq!(blk.search, "mid1\n");
        assert_eq!(blk.replace, "mid2\n");
        // 应用块应还原 new
        let mut m = BTreeMap::new();
        m.insert("f.py".to_string(), old.to_string());
        let (files, report) = apply_blocks(&m, &[blk]);
        assert_eq!(report.results[0].status, ApplyStatus::Exact);
        assert_eq!(files["f.py"], new);
    }
}
