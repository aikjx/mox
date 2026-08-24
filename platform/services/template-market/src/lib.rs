//! # 草莓多平台 · 系统模板市场（Template Market）
//!
//! 这是"对话驱动全栈生成式开发平台"的**资产中枢**：
//!
//! 1. **模板 = 一个完整可复用系统的蓝图**：包含
//!    - `graph`：由对话/璇玑生成的 FlowGraph（业务功能 + 关联关系 + 流程图）
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

pub const CRATE_ID: &str = "4d2e50c1-9d64-525d-86cf-2d7d610a27b9";
pub const ENGINE_NAME: &str = "xuanji::template_market";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// 模板业务域标签（可无限扩展 → "所有模块都是通用的"）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Domain {
    Mall,          // 商城
    Novel,         // 小说
    Thesis,        // 论文
    Book,          // 图书/出版
    VideoDesign,   // 影视设计
    ProductDesign, // 产品设计
    SystemDesign,  // 系统设计
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
    /// 由对话/璇玑生成的流程图（业务功能 + 关联关系）
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
    pub fn new(
        name: &str,
        description: &str,
        domains: Vec<Domain>,
        graph_json: serde_json::Value,
    ) -> Self {
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

    /// 预置「商城行业」种子模板：一份完整可复用的电商系统蓝图 +
    /// 由 codegen 风格的 DDL / 前端骨架组成的 artifacts。
    /// 用于在空白市场上提供可立即引用/二开的"通用商城模块"。
    pub fn seed_mall_templates(&self) -> Result<Vec<SystemTemplate>, MarketError> {
        let mut seeded = Vec::new();

        // ---- 种子1：标准电商商城（商品 / 购物车 / 下单 / 支付）----
        let mall_graph = serde_json::json!({
            "name": "标准电商商城",
            "nodes": [
                {"id":"start","type":"start","name":"开始"},
                {"id":"f1","type":"data_input","name":"浏览商品","action":"浏览","entities":["商品"],"depends_on":["start"]},
                {"id":"f2","type":"operator","name":"加入购物车","action":"加购","entities":["购物车","商品"],"depends_on":["f1"]},
                {"id":"f3","type":"operator","name":"提交订单","action":"下单","entities":["订单","购物车"],"depends_on":["f2"]},
                {"id":"f4","type":"operator","name":"支付订单","action":"支付","entities":["订单","支付"],"depends_on":["f3"]},
                {"id":"f5","type":"data_output","name":"通知发货","action":"通知","entities":["订单"],"depends_on":["f4"]},
                {"id":"end","type":"end","name":"结束"}
            ]
        });
        let mut mall_artifacts = BTreeMap::new();
        mall_artifacts.insert("generated/schema.sql".into(), mall_schema_sql());
        mall_artifacts.insert("generated/App.vue".into(), mall_app_vue());
        let mut mall = SystemTemplate::new(
            "标准电商商城",
            "通用商城系统：商品浏览、购物车、下单、支付、发货通知。可直接引用二开。",
            vec![Domain::Mall],
            mall_graph,
        )
        .with_artifacts(mall_artifacts);
        mall.reuse_count = 128; // 预置为热门模板（持续学习信号）
        mall.rating = 4.8;
        self.publish(&mall)?;
        seeded.push(mall);

        // ---- 种子2：会员制商城（在种子1基础上扩展会员/积分）----
        let member_graph = serde_json::json!({
            "name": "会员制商城",
            "nodes": [
                {"id":"start","type":"start","name":"开始"},
                {"id":"f1","type":"operator","name":"会员登录","action":"登录","entities":["会员"],"depends_on":["start"]},
                {"id":"f2","type":"data_input","name":"浏览商品","action":"浏览","entities":["商品"],"depends_on":["f1"]},
                {"id":"f3","type":"operator","name":"下单并积分","action":"下单","entities":["订单","会员","积分"],"depends_on":["f2"]},
                {"id":"f4","type":"operator","name":"支付","action":"支付","entities":["订单"],"depends_on":["f3"]},
                {"id":"end","type":"end","name":"结束"}
            ]
        });
        let mut member_artifacts = BTreeMap::new();
        member_artifacts.insert("generated/schema.sql".into(), member_schema_sql());
        let mut member = SystemTemplate::new(
            "会员制商城",
            "在通用商城基础上增加会员登录与积分体系，适合订阅/复购场景。",
            vec![Domain::Mall, Domain::ProductDesign],
            member_graph,
        )
        .with_artifacts(member_artifacts);
        member.reuse_count = 64;
        member.rating = 4.6;
        self.publish(&member)?;
        seeded.push(member);

        Ok(seeded)
    }

    /// 幂等播种：仅当市场为空（无任何模板）时写入种子。
    /// 适合在程序首次启动时调用，避免覆盖用户已发布的模板。
    pub fn ensure_seeded(&self) -> Result<usize, MarketError> {
        let existing = self.list(None, None)?;
        if !existing.is_empty() {
            return Ok(0);
        }
        let seeded = self.seed_mall_templates()?;
        Ok(seeded.len())
    }
}

/// 标准电商商城 DDL（PostgreSQL 风格）
fn mall_schema_sql() -> String {
    r#"-- 标准电商商城 · 自动生成 DDL（PostgreSQL）
CREATE TABLE IF NOT EXISTS product (
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    price       NUMERIC(12,2) NOT NULL DEFAULT 0,
    stock       INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cart (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL,
    product_id  BIGINT NOT NULL REFERENCES product(id),
    qty         INT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS orders (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL,
    total       NUMERIC(12,2) NOT NULL DEFAULT 0,
    status      VARCHAR(32) NOT NULL DEFAULT 'pending',
    paid_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS payment (
    id          BIGSERIAL PRIMARY KEY,
    order_id    BIGINT NOT NULL REFERENCES orders(id),
    amount      NUMERIC(12,2) NOT NULL,
    channel     VARCHAR(32) NOT NULL,
    paid_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#
    .to_string()
}

/// 会员制商城 DDL（在商城基础上增加会员/积分）
fn member_schema_sql() -> String {
    let mut s = mall_schema_sql();
    s.push_str(
        r#"
-- 会员 / 积分扩展
CREATE TABLE IF NOT EXISTS member (
    id          BIGSERIAL PRIMARY KEY,
    username    VARCHAR(64) NOT NULL UNIQUE,
    level       INT NOT NULL DEFAULT 1,
    points      INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS point_log (
    id          BIGSERIAL PRIMARY KEY,
    member_id   BIGINT NOT NULL REFERENCES member(id),
    delta       INT NOT NULL,
    reason      VARCHAR(64),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#,
    );
    s
}

/// 标准电商商城前端骨架（Vue3）
fn mall_app_vue() -> String {
    r#"<template>
  <div class="mall">
    <h1>商城</h1>
    <ul>
      <li v-for="p in products" :key="p.id">
        {{ p.name }} —— ¥{{ p.price }}
        <button @click="addToCart(p)">加入购物车</button>
      </li>
    </ul>
    <button @click="checkout">去结算</button>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
const products = ref([])
async function load() {
  const r = await fetch('/api/products')
  products.value = await r.json()
}
function addToCart(p) { /* 调用 /api/cart */ }
function checkout() { /* 调用 /api/order */ }
onMounted(load)
</script>
"#
    .to_string()
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
        m.publish(&SystemTemplate::new(
            "小说平台",
            "网文创作",
            vec![Domain::Novel],
            serde_json::json!({}),
        ))
        .unwrap();
        m.publish(&SystemTemplate::new(
            "商城A",
            "电商",
            vec![Domain::Mall],
            serde_json::json!({}),
        ))
        .unwrap();
        m.publish(&SystemTemplate::new(
            "商城B",
            "零售电商",
            vec![Domain::Mall],
            serde_json::json!({}),
        ))
        .unwrap();

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

    #[test]
    fn seed_mall_templates_produces_reusable_mall_seeds() {
        let m = market();
        let seeded = m.seed_mall_templates().unwrap();
        assert_eq!(seeded.len(), 2);
        assert!(seeded.iter().all(|t| t.domains.contains(&Domain::Mall)));

        // 商城种子携带 DDL 与前端骨架
        let mall = m.list(Some(&Domain::Mall), Some("标准电商")).unwrap();
        assert_eq!(mall.len(), 1);
        assert!(mall[0].artifacts.contains_key("generated/schema.sql"));
        assert!(mall[0].artifacts.contains_key("generated/App.vue"));

        // 会员种子包含会员/积分扩展表
        let member = m.list(Some(&Domain::Mall), Some("会员")).unwrap();
        assert_eq!(member.len(), 1);
        let ddl = member[0].artifacts.get("generated/schema.sql").unwrap();
        // DDL 使用 `CREATE TABLE IF NOT EXISTS` 风格，断言需匹配完整建表语句
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS member"));
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS point_log"));

        // 种子按热度排序应在前面
        let ranked = m.ranked(Some(&Domain::Mall)).unwrap();
        assert!(!ranked.is_empty());
        assert!(ranked[0].reuse_count >= 64);
    }

    #[test]
    fn ensure_seeded_is_idempotent() {
        let m = market();
        assert_eq!(m.ensure_seeded().unwrap(), 2);
        // 第二次调用不应重复写入（市场非空）
        assert_eq!(m.ensure_seeded().unwrap(), 0);
    }
}
