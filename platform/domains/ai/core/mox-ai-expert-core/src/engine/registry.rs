// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/mox

//! 内存专家注册表：实现 proto::ExpertRegistry trait
//!
//! 设计要点：
//! - 基于 HashMap + RwLock，读写并发安全
//! - 启动时预填 14 位内置专家
//! - 支持按 domain / 维度过滤
//! - 实现 DIP：下游通过 `Arc<dyn ExpertRegistry>` 访问

use crate::experts::all_experts;
use anyhow::Result;
use async_trait::async_trait;
use mox_ai_expert_proto::{Dimension, ExpertMeta, ExpertRegistry};
use std::collections::HashMap;
use std::sync::RwLock;

/// 内存专家注册表（实现 proto::ExpertRegistry trait）
pub struct InMemoryExpertRegistry {
    inner: RwLock<HashMap<String, ExpertMeta>>,
}

impl Default for InMemoryExpertRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryExpertRegistry {
    /// 创建空注册表
    pub fn empty() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// 创建并预填 14 位内置专家
    pub fn new() -> Self {
        let s = Self::empty();
        // 预填内置专家：把 all_experts() 中的 trait 对象映射为 ExpertMeta
        let experts = all_experts();
        for e in experts {
            let id = e.id();
            let dim = e.dimension();
            let meta = ExpertMeta {
                id: id.clone(),
                name: dim.display_name().to_string(),
                domain: "*".into(),
                capabilities: dimension_capabilities(dim),
                description: format!("内置璇玑专家 · 维度={:?}", dim),
                dimension: Some(format!("{:?}", dim)),
            };
            let _ = s.inner.write().unwrap().insert(meta.id.clone(), meta);
        }
        s
    }

    /// 注册一位专家（同步便捷方法）
    pub fn register_sync(&self, expert: ExpertMeta) {
        self.inner
            .write()
            .unwrap()
            .insert(expert.id.clone(), expert);
    }

    /// 同步查找
    pub fn find_sync(&self, id: &str) -> Option<ExpertMeta> {
        self.inner.read().unwrap().get(id).cloned()
    }

    /// 同步列出
    pub fn list_sync(&self, domain: Option<&str>) -> Vec<ExpertMeta> {
        let guard = self.inner.read().unwrap();
        let iter = guard.values().cloned();
        match domain {
            None | Some("*") => iter.collect(),
            Some(d) => iter.filter(|m| m.domain == d).collect(),
        }
    }

    /// 按维度列出专家
    pub fn list_by_dimension(&self, dim: Dimension) -> Vec<ExpertMeta> {
        let guard = self.inner.read().unwrap();
        guard
            .values()
            .filter(|m| {
                m.dimension
                    .as_ref()
                    .map(|s| s == &format!("{:?}", dim))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// 获取专家总数
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }
}

fn dimension_capabilities(dim: Dimension) -> Vec<String> {
    match dim {
        Dimension::Business => vec!["business".into(), "process".into(), "workflow".into()],
        Dimension::Algorithm => vec!["algorithm".into(), "optimization".into(), "llm".into()],
        Dimension::Permission => vec!["permission".into(), "authz".into(), "rbac".into()],
        Dimension::Resource => vec!["resource".into(), "quota".into(), "pool".into()],
        Dimension::Security => vec!["security".into(), "pii".into(), "data-leak".into()],
        Dimension::Data => vec!["data".into(), "privacy".into(), "lineage".into()],
        Dimension::Observability => {
            vec!["observability".into(), "monitoring".into(), "tracing".into()]
        }
        Dimension::Architecture => {
            vec!["architecture".into(), "design".into(), "pattern".into()]
        }
        Dimension::SecurityCode => {
            vec!["security-code".into(), "sast".into(), "vulnerability".into()]
        }
        Dimension::CodeQuality => {
            vec!["code-quality".into(), "lint".into(), "complexity".into()]
        }
        Dimension::Performance => {
            vec!["performance".into(), "profiling".into(), "bottleneck".into()]
        }
        Dimension::Testing => vec!["testing".into(), "coverage".into(), "qa".into()],
        Dimension::Documentation => {
            vec!["documentation".into(), "docs".into(), "readme".into()]
        }
        Dimension::Maintainability => {
            vec![
                "maintainability".into(),
                "technical-debt".into(),
                "refactor".into(),
            ]
        }
    }
}

#[async_trait]
impl ExpertRegistry for InMemoryExpertRegistry {
    async fn register(&self, expert: &ExpertMeta) -> Result<()> {
        self.inner
            .write()
            .map_err(|e| anyhow::anyhow!("Registry lock poisoned: {}", e))?
            .insert(expert.id.clone(), expert.clone());
        Ok(())
    }

    async fn list(&self, domain: Option<&str>) -> Result<Vec<ExpertMeta>> {
        let guard = self
            .inner
            .read()
            .map_err(|e| anyhow::anyhow!("Registry lock poisoned: {}", e))?;
        let iter = guard.values().cloned();
        let out: Vec<ExpertMeta> = match domain {
            None => iter.collect(),
            Some("*") => iter.collect(),
            Some(d) => iter.filter(|m| m.domain == d).collect(),
        };
        Ok(out)
    }

    async fn find(&self, id: &str) -> Result<Option<ExpertMeta>> {
        let guard = self
            .inner
            .read()
            .map_err(|e| anyhow::anyhow!("Registry lock poisoned: {}", e))?;
        Ok(guard.get(id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn registry_has_fourteen_experts() {
        let reg = InMemoryExpertRegistry::new();
        let experts = reg.list(None).await.unwrap();
        assert_eq!(experts.len(), 14, "注册表应预填 14 位专家");
    }

    #[tokio::test]
    async fn registry_find_by_id() {
        let reg = InMemoryExpertRegistry::new();
        let found = reg.find("security").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "security");
    }

    #[tokio::test]
    async fn registry_find_missing_returns_none() {
        let reg = InMemoryExpertRegistry::new();
        let found = reg.find("nonexistent").await.unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn sync_methods_work() {
        let reg = InMemoryExpertRegistry::new();
        assert_eq!(reg.len(), 14);
        assert!(!reg.is_empty());

        let found = reg.find_sync("business");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "business");

        let all = reg.list_sync(None);
        assert_eq!(all.len(), 14);
    }

    #[test]
    fn list_by_dimension_filters() {
        let reg = InMemoryExpertRegistry::new();
        let sec = reg.list_by_dimension(Dimension::Security);
        assert_eq!(sec.len(), 1);
        assert_eq!(sec[0].id, "security");
    }

    #[tokio::test]
    async fn register_adds_new_expert() {
        let reg = InMemoryExpertRegistry::empty();
        assert!(reg.is_empty());

        let meta = ExpertMeta::new("custom", "Custom Expert", "test");
        reg.register(&meta).await.unwrap();

        assert_eq!(reg.len(), 1);
        let found = reg.find("custom").await.unwrap().unwrap();
        assert_eq!(found.name, "Custom Expert");
    }

    #[test]
    fn empty_registry_is_empty() {
        let reg = InMemoryExpertRegistry::empty();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }
}
