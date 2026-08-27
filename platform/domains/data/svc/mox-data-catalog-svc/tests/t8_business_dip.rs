// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Business-catalog DIP 验收：`optimize_with` / `register_business_experts`
//! 只依赖 Arc<dyn ExpertConsultant> / Arc<dyn ExpertRegistry> trait 对象。
//!
//! tr_b8_01_optimize_with_mock_passes：Business.optimize_with(MockAlwaysApproved)
//!   → 返回 Mock 分数，不调用真实引擎。
//! tr_b8_02_error_fallback_to_veto：consultant 抛 Err → 内置 fallback 返回
//!   vetoed=true，不 panic（生产降级保证）。
//! tr_b8_03_register_uses_mock_registry：MockRegistry 收到 register 调用，
//!   证明 register_business_experts 仅依赖 Arc<dyn ExpertRegistry>，不出现 concrete。
//! tr_b8_04_all_business_ids_covered_by_registry：每条业务注册后可在 registry
//!   中按 id 查到（mock 内存实现）。
//! tr_b8_05_no_mox_concrete_import：静态扫描 src，仅允许 use expert_traits/types/domain。

use async_trait::async_trait;
use mox_data_catalog_svc::{all_businesses, register_business_experts, Business};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use mox_ai_expert_svc::expert_traits::{ExpertConsultant, ExpertRegistry};
use mox_ai_expert_svc::types::{ConsultQuery, ConsultReport, ExpertMeta, Result as ExpertResult};

// ---- Mock Always Approved ----
struct MockApproved;
#[async_trait]
impl ExpertConsultant for MockApproved {
    async fn consult(&self, _q: &ConsultQuery) -> ExpertResult<ConsultReport> {
        unreachable!()
    }
    fn consult_blocking(&self, q: &ConsultQuery) -> ExpertResult<ConsultReport> {
        Ok(ConsultReport {
            report_id: q.id.clone(),
            steps: vec!["[MockApproved] business DIP".into()],
            score: 0.88,
            vetoed: false,
            reason: None,
        })
    }
}

// ---- Mock Error Consultant ----
struct MockError;
#[async_trait]
impl ExpertConsultant for MockError {
    async fn consult(&self, _q: &ConsultQuery) -> ExpertResult<ConsultReport> {
        unreachable!()
    }
    fn consult_blocking(&self, _q: &ConsultQuery) -> ExpertResult<ConsultReport> {
        let e = std::io::Error::other("模拟专家引擎不可用");
        Err(e.into())
    }
}

// ---- Mock Registry (线程内记录) ----
struct MockRegistry {
    inner: Mutex<Vec<ExpertMeta>>,
}
impl MockRegistry {
    fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }
    fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
    fn contains(&self, id: &str) -> bool {
        self.inner.lock().unwrap().iter().any(|x| x.id == id)
    }
}

#[async_trait]
impl ExpertRegistry for MockRegistry {
    async fn register(&self, expert: &ExpertMeta) -> ExpertResult<()> {
        self.inner.lock().unwrap().push(expert.clone());
        Ok(())
    }
    async fn list(&self, domain: Option<&str>) -> ExpertResult<Vec<ExpertMeta>> {
        let g = self.inner.lock().unwrap();
        let res: Vec<_> = g
            .iter()
            .filter(|e| domain.is_none_or(|d| e.domain == d))
            .cloned()
            .collect();
        Ok(res)
    }
    async fn find(&self, id: &str) -> ExpertResult<Option<ExpertMeta>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.id == id)
            .cloned())
    }
}

#[test]
fn tr_b8_01_optimize_with_mock_approved() {
    let biz: &Business = &all_businesses()[0]; // gov-pii
    let rep = biz.optimize_with(Arc::new(MockApproved));
    assert_eq!(rep.report_id, biz.id);
    assert!((rep.score - 0.88).abs() < 1e-9, "score={}", rep.score);
    assert!(!rep.vetoed);
    assert!(rep.steps.iter().any(|s| s.contains("business DIP")));
}

#[test]
fn tr_b8_02_error_fallback_to_veto_guarantees_no_panic() {
    let biz: &Business = &all_businesses()[1]; // finance
    let rep = biz.optimize_with(Arc::new(MockError));
    // 生产降级保证： consultant 出错不应 panic，而是返回 vetoed=true 带 error reason
    assert!(rep.vetoed, "fallback 应返回 vetoed=true，实际 {:?}", rep);
    assert!(rep.reason.as_ref().is_some_and(|r| r.contains("error")));
    assert_eq!(rep.report_id, biz.id);
    assert_eq!(rep.score, 0.0);
}

#[tokio::test]
async fn tr_b8_03_register_uses_mock_registry_trait_object() {
    let reg = Arc::new(MockRegistry::new());
    register_business_experts(reg.clone())
        .await
        .expect("register 应成功");
    // 至少应注册 N 条业务（全量业务 > 5）
    assert!(reg.len() >= 5, "应至少注册 5 条，实际 {}", reg.len());
    assert!(reg.contains("biz-gov-pii"), "缺少政务专家注册");
}

#[tokio::test]
async fn tr_b8_04_all_businesses_can_be_found_via_trait_list() {
    let reg = Arc::new(MockRegistry::new());
    register_business_experts(reg.clone()).await.unwrap();

    // 通过 Arc<dyn ExpertRegistry>.list() 走 trait 抽象，不用 concrete struct
    let all: Vec<ExpertMeta> = reg.list(None).await.unwrap();
    assert!(!all.is_empty());

    // 按 domain 过滤 gov 至少 1 条
    let gov = reg.list(Some("gov")).await.unwrap();
    assert!(!gov.is_empty(), "gov 领域应有专家注册（来自 gov-pii 业务）");

    // find 按 id
    let bot = reg.find("biz-bot").await.unwrap();
    assert!(bot.is_some(), "应存在 biz-bot 专家（对应 bot 业务）");
    assert_eq!(bot.unwrap().id, "biz-bot");
}

#[test]
fn tr_b8_05_no_mox_concrete_import() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest.join("src");
    assert!(src_dir.is_dir());
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out)
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&src_dir, &mut files);
    assert!(!files.is_empty());
    let allowed = ["expert_traits", "types", "domain"];

    for f in &files {
        let content = std::fs::read_to_string(f).unwrap();
        for (idx, line) in content.lines().enumerate() {
            let t = line.trim_start();
            if !(t.starts_with("use ") || t.starts_with("pub use ")) {
                continue;
            }
            let Some(rest_p) = t.find("mox_ai_expert_svc::") else {
                continue;
            };
            let after = &t[rest_p + "mox_ai_expert_svc::".len()..];
            let end = after
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(after.len());
            let mod_name = &after[..end];
            if !allowed.contains(&mod_name) {
                panic!(
                    "tr_b8_05 FAIL: {}:{} use mox_ai_expert_svc::{}（仅允许 expert_traits/types/domain）;\n原行: {}",
                    f.display(), idx + 1, mod_name, line.trim()
                );
            }
        }
    }
}
