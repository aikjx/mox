// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use dashmap::DashMap;

use crate::pipeline::{PipelineCtx, StepResult};
use mox_platform_datastore_core::MetaRepository;

pub trait BizModule: Send + Sync {
    fn industry_code(&self) -> &'static str;

    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        let _ = ctx;
        StepResult::Continue
    }

    fn hook_after(&self, ctx: &mut PipelineCtx) -> StepResult {
        let _ = ctx;
        StepResult::Continue
    }
}

pub struct ModuleRegistry {
    pub mods: DashMap<String, Box<dyn BizModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let r = Self {
            mods: DashMap::new(),
        };
        r.register(Box::new(CommonModule));
        r.register(Box::new(FinanceModule));
        r.register(Box::new(MedicalModule));
        r.register(Box::new(ManufacturingModule));
        r.register(Box::new(GovernmentModule));
        r.register(Box::new(EducationModule));
        r.register(Box::new(RetailModule));
        r
    }

    pub fn register(&self, m: Box<dyn BizModule>) {
        self.mods.insert(m.industry_code().to_string(), m);
    }

    pub fn find_by_entity<R: MetaRepository>(
        &self,
        _entity_code: &str,
        _meta_repo: &R,
        _tenant_id: &str,
    ) -> Option<String> {
        None
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommonModule;
impl BizModule for CommonModule {
    fn industry_code(&self) -> &'static str {
        "common"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
                if title.trim().is_empty() {
                    return StepResult::Stop(anyhow::anyhow!("common: title 不能为空字符串"));
                }
            }
        }
        StepResult::Continue
    }
}

pub struct FinanceModule;
impl BizModule for FinanceModule {
    fn industry_code(&self) -> &'static str {
        "finance"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if let Some(amount) = data.get("amount").and_then(|v| v.as_f64()) {
                if amount <= 0.0 {
                    return StepResult::Stop(anyhow::anyhow!("finance: amount 必须 > 0"));
                }
            }
        }
        StepResult::Continue
    }
}

pub struct MedicalModule;
impl BizModule for MedicalModule {
    fn industry_code(&self) -> &'static str {
        "medical"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if data.contains_key("patient_id") {
                if let Some(pid) = data.get("patient_id").and_then(|v| v.as_str()) {
                    if pid.len() < 2 {
                        return StepResult::Stop(anyhow::anyhow!("medical: patient_id 格式无效"));
                    }
                }
            }
        }
        StepResult::Continue
    }
}

pub struct ManufacturingModule;
impl BizModule for ManufacturingModule {
    fn industry_code(&self) -> &'static str {
        "manufacturing"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if let Some(qty) = data.get("quantity").and_then(|v| v.as_i64()) {
                if qty < 0 {
                    return StepResult::Stop(anyhow::anyhow!("manufacturing: quantity 不能为负"));
                }
            }
        }
        StepResult::Continue
    }
}

pub struct GovernmentModule;
impl BizModule for GovernmentModule {
    fn industry_code(&self) -> &'static str {
        "government"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if data.contains_key("classification_level") {
                if let Some(v) = data.get("classification_level").and_then(|x| x.as_str()) {
                    let allowed = ["internal", "secret", "top-secret"];
                    if !allowed.contains(&v) {
                        return StepResult::Stop(anyhow::anyhow!("government: 密级无效"));
                    }
                }
            }
        }
        StepResult::Continue
    }
}

pub struct EducationModule;
impl BizModule for EducationModule {
    fn industry_code(&self) -> &'static str {
        "education"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if let Some(score) = data.get("score").and_then(|v| v.as_f64()) {
                if !(0.0..=100.0).contains(&score) {
                    return StepResult::Stop(anyhow::anyhow!("education: score 范围 0-100"));
                }
            }
        }
        StepResult::Continue
    }
}

pub struct RetailModule;
impl BizModule for RetailModule {
    fn industry_code(&self) -> &'static str {
        "retail"
    }
    fn hook_before(&self, ctx: &mut PipelineCtx) -> StepResult {
        if let Some(data) = &ctx.request_data {
            if let Some(sku) = data.get("sku").and_then(|v| v.as_str()) {
                if sku.len() < 3 {
                    return StepResult::Stop(anyhow::anyhow!("retail: SKU 至少 3 位"));
                }
            }
        }
        StepResult::Continue
    }
}
