//! # 草莓多平台 · 系统模板市场（Template Market）
//!
//! 这是"对话驱动全栈生成式开发平台"的**资产中枢**：
//!
//! 1. **模板 = 一个完整可复用系统的蓝图**：包含
//!    - `graph`：由对话/专家联盟生成的 FlowGraph（业务功能 + 关联关系 + 流程图）
//!    - `artifacts`：由 codegen 生成的代码包（后端/DB/前端），可留空待生成
//!    - `tags`：业务域标签（商城 / 小说 / 论文 / 产品设计 / 影视 …），支持通用模块归类
//!    - `derived_from`：引用链（"引用下载"他人模板后二开）
//! 2. **四类核心操作**（对应你的诉求）：
//!    - `publish`   —— 上传/发布一个系统模板（也可从对话实时生成后落盘）
//!    - `list`      —— 浏览所有模板（按标签/关键词检索，支持"通用模块"复用）
//!    - `load`      —— 下载/加载模板到本地工程
//!    - `fork`      —— 引用他人模板生成派生模板（"引用下载后快速开发"）
//! 3. **持续学习**：`record_feedback` 把"某模板被复用/评分"的反馈沉淀，供后续生成优化。
//!
//! 所有模板以 JSON 持久化到 `templates/` 目录，幂等、可版本化、可走 Git 协作。

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// 模板业务域标签（可无限扩展 → "所有模块都是通用的"）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Domain {
    Mall,        // 商城
    Novel,       // 小说
    Thesis,      // 论文
    Book,        // 图书/出版
    VideoDesign, // 影视设计
    ProductDesign, // 产品设计
    SystemDesign, // 系统设计
    Other(String),
}

impl Domain {
    pub fn as_str(&self) -> String {
        match self {
            Domain::Mall => "mall".into(),
            Domain::Novel => "novel".into(),
            Domain::Thesis => "thesis".into(),
            Domain::Book => "book".into(),
            Domain::VideoDesign => "video_design".into(),
            Domain::ProductDesign => "product_design".into(),
            Domain::SystemDesign => "system_design".into(),
            Domain::Other(s) => s.clone(),
        }
    }
    pub fn parse(s: &str) -> Domain {
        match s {
            "mall" => Domain::Mall,
            "novel" => Domain::Novel,
            "thesis" => Domain::Thesis,
            "book" => Domain::Book,
            "video_design" => Domain::VideoDesign,
            "product_design" => Domain::ProductDesign,
            "system_design" => Domain::SystemDesign,
            other => Domain::Other(other.to_string()),
        }
    }
}

/// 单个系统模板（草莓多平台的核心资产）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domains: Vec<Domain>,
    /// 由对话/专家联盟生成的流程图（业务功能 + 关联关系）
    pub graph_json: serde_json::Value,
    /// 由 codegen 生成的代码包（后端/DB/前端）；可留空待后续生成
    pub artifacts: BTreeMap<String, String>,
    /// 引用链：本模板派生自哪个模板（"引用下载"二开）
    pub derived_from: Option<String>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 复用次数（持续学习信号）
    pub reuse_count: u64,
    /// 平均评分 0..5（持续学习信号）
    pub rating: f32,
}

impl SystemTemplate {
    pub fn new(name: &str, description: &str, domains: Vec<Domain>, graph_json: serde_json::Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            domains,
            graph_json,
            artifacts: BTreeMap::new(),
            derived_from: None,
            version: 1,
            created_at: now,
            updated_at: now,
            reuse_count: 0,
            rating: 0.0,
        }
    }

    /// 绑定生成的代码包（后端/DB/前端）
    pub fn with_artifacts(mut self, artifacts: BTreeMap<String, String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// "引用下载"：派生一个子模板，继承父模板的图与代码，重置评分/复用
    pub fn fork(&self, new_name: &str, new_description: &str) -> SystemTemplate {
        let mut child = SystemTemplate::new(
            new_name,
            new_description,
            self.domains.clone(),
            self.graph_json.clone(),
        );
        child.artifacts = self.artifacts.clone();
        child.derived_from = Some(self.id.clone());
        child.version = 1;
        child
    }
}

/// 模板市场的持久化与检索中枢
#[derive(Debug, Clone)]
pub struct TemplateMarket {
    root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum MarketError {
    #[error("模板不存在: {0}")]
    NotFound(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("通用错误: {0}")]
    Anyhow(#[from] anyhow::Error),
}

impl TemplateMarket {
    /// 以目录初始化市场（默认 `templates/`）
    pub fn open<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).context("创建模板目录失败")?;
        Ok(Self { root })
    }

    fn path_of(&self, id: &str) -> PathBuf {
        self.root.join(format!("{}.json", id))
    }

    /// 发布/上传一个系统模板（幂等：同 id 覆盖更新版本）
    pub fn publish(&self, tpl: &SystemTemplate) -> Result<(), MarketError> {
        let path = self.path_of(&tpl.id);
        let json = serde_json::to_string_pretty(tpl)?;
        std::fs::write(&path, json)?;
        tracing::info!(target: "template_market", "发布模板 {} ({})", tpl.name, tpl.id);
        Ok(())
    }

    /// 列出全部模板（可按域/关键词过滤）
    pub fn list(&self, domain: Option<&Domain>, keyword: Option<&str>) -> Result<Vec<SystemTemplate>, MarketError> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let content = std::fs::read_to_string(entry.path())?;
            let tpl: SystemTemplate = serde_json::from_str(&content)?;
            if let Some(d) = domain {
                if !tpl.domains.iter().any(|x| x == d) {
                    continue;
                }
            }
            if let Some(kw) = keyword {
                let hay = format!("{} {}", tpl.name, tpl.description).to_lowercase();
                if !hay.contains(&kw.to_lowercase()) {
                    continue;
                }
            }
            out.push(tpl);
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(out)
    }

    /// 加载（下载）一个模板
    pub fn load(&self, id: &str) -> Result<SystemTemplate, MarketError> {
        let path = self.path_of(id);
        if !path.exists() {
            return Err(MarketError::NotFound(id.to_string()));
        }
        let content = std::fs::read_to_string(&path)?;
        let mut tpl: SystemTemplate = serde_json::from_str(&content)?;
        tpl.reuse_count += 1;
        // 复用即沉淀学习信号
        let json = serde_json::to_string_pretty(&tpl)?;
        std::fs::write(&path, json)?;
        Ok(tpl)
    }

    /// 删除模板
    pub fn remove(&self, id: &str) -> Result<(), MarketError> {
        let path = self.path_of(id);
        if !path.exists() {
            return Err(MarketError::NotFound(id.to_string()));
        }
        std::fs::remove_file(&path)?;
        Ok(())
    }

    /// 记录用户评分（持续学习闭环：高分模板在检索时优先）
    pub fn rate(&self, id: &str, score: f32) -> Result<(), MarketError> {
        let mut tpl = self.load(id)?;
        // 简单指数滑动平均
        let s = score.clamp(0.0, 5.0);
        tpl.rating = if tpl.rating == 0.0 { s } else { tpl.rating * 0.8 + s * 0.2 };
        self.publish(&tpl)?;
        Ok(())
    }

    /// 检索排序：评分 × 复用次数的综合热度（指导"通用/优质模板"浮现）
    pub fn ranked(&self, domain: Option<&Domain>) -> Result<Vec<SystemTemplate>> {
        let mut list = self.list(domain, None)?;
        list.sort_by(|a, b| {
            let heat = |t: &SystemTemplate| (t.rating as f64) * (1.0 + t.reuse_count as f64);
            heat(b).partial_cmp(&heat(a)).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market() -> TemplateMarket {
        let dir = std::env::temp_dir().join(format!("caomei_market_{}", Uuid::new_v4()));
        TemplateMarket::open(&dir).unwrap()
    }

    #[test]
    fn publish_then_load_roundtrip() {
        let m = market();
        let tpl = SystemTemplate::new(
            "商城模板",
            "标准电商系统",
            vec![Domain::Mall],
            serde_json::json!({"name":"mall","nodes":["home","cart","pay"]}),
        );
        m.publish(&tpl).unwrap();
        let loaded = m.load(&tpl.id).unwrap();
        assert_eq!(loaded.name, "商城模板");
        assert_eq!(loaded.reuse_count, 1);
    }

    #[test]
    fn fork_keeps_parent_link_and_artifacts() {
        let m = market();
        let parent = SystemTemplate::new(
            "父模板",
            "基础系统",
            vec![Domain::SystemDesign],
            serde_json::json!({"nodes":["a","b"]}),
        )
        .with_artifacts({
            let mut a = BTreeMap::new();
            a.insert("schema.sql".into(), "CREATE TABLE t();".into());
            a
        });
        m.publish(&parent).unwrap();

        let child = parent.fork("子模板-二开", "在父模板基础上扩展");
        assert_eq!(child.derived_from.as_deref(), Some(parent.id.as_str()));
        assert!(child.artifacts.contains_key("schema.sql"));
        m.publish(&child).unwrap();

        let list = m.list(Some(&Domain::SystemDesign), None).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn list_filter_by_domain_and_keyword() {
        let m = market();
        m.publish(&SystemTemplate::new("小说平台", "网文创作", vec![Domain::Novel], serde_json::json!({}))).unwrap();
        m.publish(&SystemTemplate::new("商城A", "电商", vec![Domain::Mall], serde_json::json!({}))).unwrap();
        m.publish(&SystemTemplate::new("商城B", "零售电商", vec![Domain::Mall], serde_json::json!({}))).unwrap();

        let novels = m.list(Some(&Domain::Novel), None).unwrap();
        assert_eq!(novels.len(), 1);
        let malls = m.list(Some(&Domain::Mall), None).unwrap();
        assert_eq!(malls.len(), 2);
        let kw = m.list(None, Some("零售")).unwrap();
        assert_eq!(kw.len(), 1);
    }

    #[test]
    fn ranking_reflects_reuse_and_rating() {
        let m = market();
        let a = SystemTemplate::new("A", "a", vec![Domain::Mall], serde_json::json!({}));
        let b = SystemTemplate::new("B", "b", vec![Domain::Mall], serde_json::json!({}));
        m.publish(&a).unwrap();
        m.publish(&b).unwrap();
        let _ = m.load(&a.id); // A 复用 +1
        m.rate(&a.id, 5.0).unwrap();
        let ranked = m.ranked(Some(&Domain::Mall)).unwrap();
        assert_eq!(ranked[0].id, a.id);
    }

    #[test]
    fn remove_works() {
        let m = market();
        let t = SystemTemplate::new("临时", "x", vec![Domain::Book], serde_json::json!({}));
        m.publish(&t).unwrap();
        m.remove(&t.id).unwrap();
        assert!(m.load(&t.id).is_err());
    }
}
