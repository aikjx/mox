// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 文档版本服务：创建 / 列表 / 读取 / 对比 / 回滚
//!
//! 基于内容寻址去重存储的语义：内容相同的版本天然去重；
//! 版本快照以 JSON 内嵌 `versions[]` 存储（零拷贝恢复 = 回滚仅改指针语义）。

use crate::model::{KbDocument, KbVersion, now_iso};

/// 版本服务
#[derive(Clone)]
pub struct KbVersionService;

/// 版本对比条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionDiff {
    pub version: String,
    pub added_lines: Vec<String>,
    pub removed_lines: Vec<String>,
    pub unchanged: usize,
}

impl KbVersionService {
    /// 创建新版本：当前正文快照为 vN，进入 vN+1
    pub fn create(doc: &mut KbDocument, note: &str) -> KbVersion {
        let next = next_version(&doc.current_version);
        // 把当前版本推进为历史快照
        let snapshot = KbVersion {
            version: doc.current_version.clone(),
            title: doc.title.clone(),
            content: doc.content.clone(),
            note: note.to_string(),
            created_at: now_iso(),
        };
        doc.versions.push(snapshot);
        doc.current_version = next;
        doc.updated_at = now_iso();
        doc.versions
            .last()
            .cloned()
            .unwrap_or_else(|| KbVersion {
                version: doc.current_version.clone(),
                title: doc.title.clone(),
                content: doc.content.clone(),
                note: note.to_string(),
                created_at: now_iso(),
            })
    }

    /// 版本列表（含当前版本，倒序）
    pub fn list(doc: &KbDocument) -> Vec<KbVersion> {
        let mut all = doc.versions.clone();
        all.push(KbVersion {
            version: doc.current_version.clone(),
            title: doc.title.clone(),
            content: doc.content.clone(),
            note: "当前版本".into(),
            created_at: doc.updated_at.clone(),
        });
        all.sort_by(|a, b| b.version.cmp(&a.version));
        all
    }

    /// 读取指定版本内容
    pub fn get(doc: &KbDocument, version: &str) -> Option<KbVersion> {
        if version == doc.current_version {
            return Some(KbVersion {
                version: doc.current_version.clone(),
                title: doc.title.clone(),
                content: doc.content.clone(),
                note: "当前版本".into(),
                created_at: doc.updated_at.clone(),
            });
        }
        doc.versions.iter().find(|v| v.version == version).cloned()
    }

    /// 对比两版本（按行集合差异，保留顺序）
    pub fn compare(doc: &KbDocument, v1: &str, v2: &str) -> Option<VersionDiff> {
        let a = Self::get(doc, v1)?;
        let b = Self::get(doc, v2)?;
        let lines_a: Vec<&str> = a.content.lines().collect();
        let lines_b: Vec<&str> = b.content.lines().collect();
        let added: Vec<String> = lines_b
            .iter()
            .filter(|l| !lines_a.contains(l))
            .map(|l| l.to_string())
            .collect();
        let removed: Vec<String> = lines_a
            .iter()
            .filter(|l| !lines_b.contains(l))
            .map(|l| l.to_string())
            .collect();
        let unchanged = lines_b.iter().filter(|l| lines_a.contains(l)).count();
        Some(VersionDiff {
            version: format!("{v1} → {v2}"),
            added_lines: added,
            removed_lines: removed,
            unchanged,
        })
    }

    /// 回滚到历史版本：正文替换为指定版本内容，并保留回滚快照
    pub fn revert(doc: &mut KbDocument, version: &str) -> Option<KbVersion> {
        let target = Self::get(doc, version)?;
        let note = format!("回滚至 {version}");
        Self::create(doc, &note);
        doc.title = target.title;
        doc.content = target.content;
        doc.current_version = next_version(&doc.current_version);
        doc.updated_at = now_iso();
        Some(KbVersion {
            version: doc.current_version.clone(),
            title: doc.title.clone(),
            content: doc.content.clone(),
            note,
            created_at: now_iso(),
        })
    }
}

/// 版本号自增：v1 → v2
fn next_version(current: &str) -> String {
    let n = current.trim_start_matches("v").parse::<u32>().unwrap_or(1);
    format!("v{}", n + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::KbDocument;

    #[test]
    fn version_create_and_list() {
        let mut doc = KbDocument::new("kb-1".into(), "标题".into(), "第一行\n第二行".into(), "cat-tech".into());
        assert_eq!(doc.current_version, "v1");
        KbVersionService::create(&mut doc, "初版");
        assert_eq!(doc.current_version, "v2");
        assert_eq!(doc.versions.len(), 1);
        assert_eq!(doc.versions[0].version, "v1");
        let all = KbVersionService::list(&doc);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn version_compare_and_revert() {
        let mut doc = KbDocument::new("kb-2".into(), "标题".into(), "A\nB\nC".into(), "cat-tech".into());
        KbVersionService::create(&mut doc, "v1 快照");
        // 修改正文
        doc.content = "A\nB\nD".into();
        let diff = KbVersionService::compare(&doc, "v1", "v2").unwrap();
        assert!(diff.added_lines.iter().any(|l| l == "D"));
        assert!(diff.removed_lines.iter().any(|l| l == "C"));
        // 回滚到 v1
        let reverted = KbVersionService::revert(&mut doc, "v1").unwrap();
        assert_eq!(reverted.content, "A\nB\nC");
    }
}
