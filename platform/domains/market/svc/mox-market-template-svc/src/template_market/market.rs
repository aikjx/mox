// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 模板市场的持久化与检索中枢

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::types::{Domain, MarketError, SystemTemplate};

/// 模板市场的持久化与检索中枢
#[derive(Debug, Clone)]
pub struct TemplateMarket {
    root: PathBuf,
}

impl TemplateMarket {
    /// 以目录初始化市场（默认 `templates/`）
    pub fn open<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).context("创建模板目录失败")?;
        Ok(Self { root })
    }

    pub(crate) fn path_of(&self, id: &str) -> PathBuf {
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
    pub fn list(
        &self,
        domain: Option<&Domain>,
        keyword: Option<&str>,
    ) -> Result<Vec<SystemTemplate>, MarketError> {
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
        out.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
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
        tpl.rating = if tpl.rating == 0.0 {
            s
        } else {
            tpl.rating * 0.8 + s * 0.2
        };
        self.publish(&tpl)?;
        Ok(())
    }

    /// 检索排序：评分 × 复用次数的综合热度（指导"通用/优质模板"浮现）
    pub fn ranked(&self, domain: Option<&Domain>) -> Result<Vec<SystemTemplate>> {
        let mut list = self.list(domain, None)?;
        list.sort_by(|a, b| {
            let heat = |t: &SystemTemplate| (t.rating as f64) * (1.0 + t.reuse_count as f64);
            heat(b)
                .partial_cmp(&heat(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(list)
    }
}
