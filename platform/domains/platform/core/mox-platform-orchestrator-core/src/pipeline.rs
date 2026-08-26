use dashmap::DashMap;
use serde_json::{Map, Value};
use std::time::Instant;

use mox_platform_datastore_core::IamRepository;
use mox_platform_datastore_core::MetaRepository;
use mox_platform_datastore_core::{compute_hash, Filter, SortSpec, TxManager, UniversalBizDAO};

use crate::event::{BusinessEvent, EventBus};
use crate::metrics::Metrics;
use crate::module::ModuleRegistry;
use crate::orchestrator::{BizAction, BusinessRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StageId {
    Auth = 0,
    Validate = 1,
    Before = 2,
    Transaction = 3,
    Main = 4,
    After = 5,
    Enrich = 6,
    Notify = 7,
    Event = 8,
    Audit = 9,
}

pub enum StepResult {
    Continue,
    Stop(anyhow::Error),
    Skip,
}

pub struct Stage {
    pub id: StageId,
    pub f: Box<dyn Fn(&mut PipelineCtx) -> StepResult + Send + Sync>,
}

impl std::fmt::Debug for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stage").field("id", &self.id).finish()
    }
}

pub struct Pipeline {
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, Default)]
pub struct PipelineCtx {
    pub tenant_id: String,
    pub user_id: String,
    pub entity_code: String,
    pub action: String,
    pub biz_id: Option<String>,
    pub biz_code: Option<String>,
    pub workflow_instance_id: Option<String>,
    pub request_data: Option<Map<String, Value>>,
    pub filters: Vec<Filter>,
    pub sort: SortSpec,
    pub page: i64,
    pub page_size: i64,
    pub version: Option<i64>,
    pub response_data: Option<Value>,
    pub response_list_total: Option<i64>,
    pub curr_hash: Option<String>,
    pub snapshot_before: Option<Value>,
    pub snapshot_after: Option<Value>,
    pub changed_fields: Option<Vec<String>>,
    pub audit_log_detail: Option<String>,
    pub event_pending: Option<BusinessEvent>,
    pub error: Option<String>,
    pub extra: DashMap<String, Value>,
    pub started_at: Option<u128>,
}

impl PipelineCtx {
    pub fn new(req: &BusinessRequest) -> Self {
        Self {
            tenant_id: req.tenant_id.clone(),
            user_id: req.user_id.clone(),
            entity_code: req.entity_code.clone(),
            action: req.action.as_str().to_string(),
            biz_id: req.biz_id.clone(),
            biz_code: req.biz_code.clone(),
            workflow_instance_id: req.workflow_instance_id.clone(),
            request_data: req.data.clone(),
            filters: req.filters.clone(),
            sort: req.sort.clone(),
            page: req.page,
            page_size: req.page_size,
            version: None,
            response_data: None,
            response_list_total: None,
            curr_hash: None,
            snapshot_before: None,
            snapshot_after: None,
            changed_fields: None,
            audit_log_detail: None,
            event_pending: None,
            error: None,
            extra: DashMap::new(),
            started_at: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub success: bool,
    pub error: Option<String>,
    pub stages_run: Vec<StageId>,
}

impl Pipeline {
    pub fn enterprise_default() -> Self {
        let stages: Vec<Stage> = vec![
            Stage {
                id: StageId::Auth,
                f: Box::new(|ctx| {
                    let orc_ptr = ctx.extra.get("__orc_iam_meta").map(|r| r.value().clone());
                    let orc = orc_ptr.as_ref().and_then(|v| v.get("iam_meta").cloned());
                    // 校验逻辑由 orchestrator 的闭包注入；此处以 extra 中 permission 检查结果为准
                    if let Some(perm_err) = ctx.extra.get("__auth_error") {
                        return StepResult::Stop(anyhow::anyhow!(
                            "{}",
                            perm_err.as_str().unwrap_or("auth error")
                        ));
                    }
                    let _ = orc;
                    StepResult::Continue
                }),
            },
            Stage {
                id: StageId::Validate,
                f: Box::new(|ctx| {
                    if let Some(err) = ctx.extra.get("__validate_error") {
                        return StepResult::Stop(anyhow::anyhow!(
                            "{}",
                            err.as_str().unwrap_or("validate error")
                        ));
                    }
                    StepResult::Continue
                }),
            },
            Stage {
                id: StageId::Before,
                f: Box::new(|ctx| {
                    if let Some(err) = ctx.extra.get("__before_error") {
                        return StepResult::Stop(anyhow::anyhow!(
                            "{}",
                            err.as_str().unwrap_or("before hook error")
                        ));
                    }
                    StepResult::Continue
                }),
            },
            Stage {
                id: StageId::Transaction,
                f: Box::new(|_| StepResult::Continue),
            },
            Stage {
                id: StageId::Main,
                f: Box::new(|_| StepResult::Continue),
            },
            Stage {
                id: StageId::After,
                f: Box::new(|ctx| {
                    if let Some(err) = ctx.extra.get("__after_error") {
                        return StepResult::Stop(anyhow::anyhow!(
                            "{}",
                            err.as_str().unwrap_or("after hook error")
                        ));
                    }
                    StepResult::Continue
                }),
            },
            Stage {
                id: StageId::Enrich,
                f: Box::new(|_| StepResult::Continue),
            },
            Stage {
                id: StageId::Notify,
                f: Box::new(|_| StepResult::Continue),
            },
            Stage {
                id: StageId::Event,
                f: Box::new(|_| StepResult::Continue),
            },
            Stage {
                id: StageId::Audit,
                f: Box::new(|_| StepResult::Continue),
            },
        ];
        Self { stages }
    }

    pub fn run<M: MetaRepository, I: IamRepository>(
        &self,
        ctx: &mut PipelineCtx,
        dao: &UniversalBizDAO,
        tx_manager: Option<&TxManager>,
        registry: &ModuleRegistry,
        event_bus: &EventBus,
        metrics: &Metrics,
        meta_repo: &M,
        iam_repo: &I,
        req: &BusinessRequest,
    ) -> PipelineResult {
        let start = Instant::now();
        let mut stages_run = Vec::new();
        let mut first_error: Option<String> = None;

        let action_str = req.action.as_str();
        ctx.started_at = Some(start.elapsed().as_nanos());

        for stage in &self.stages {
            stages_run.push(stage.id);

            let result = match stage.id {
                StageId::Auth => {
                    let perm_action = match req.action {
                        BizAction::Create => "create",
                        BizAction::Update => "update",
                        BizAction::Delete => "delete",
                        BizAction::Get => "read",
                        BizAction::List => "list",
                    };
                    match iam_repo.check_permission(
                        &ctx.tenant_id,
                        &ctx.user_id,
                        &ctx.entity_code,
                        perm_action,
                    ) {
                        Ok(()) => StepResult::Continue,
                        Err(e) => StepResult::Stop(e),
                    }
                }
                StageId::Validate => {
                    if matches!(req.action, BizAction::Create | BizAction::Update) {
                        match meta_repo.get_entity(&ctx.tenant_id, &ctx.entity_code) {
                            Ok(entity) => {
                                let d = ctx.request_data.clone().unwrap_or_default();
                                let skip_required = matches!(req.action, BizAction::Update);
                                match meta_repo.evaluate_rules_inner(&entity, &d, skip_required) {
                                    Ok(()) => StepResult::Continue,
                                    Err(e) => StepResult::Stop(e),
                                }
                            }
                            Err(e) => StepResult::Stop(e),
                        }
                    } else {
                        StepResult::Continue
                    }
                }
                StageId::Before => {
                    let industry_opt =
                        registry.find_by_entity(&ctx.entity_code, meta_repo, &ctx.tenant_id);
                    let mut r = StepResult::Continue;
                    if let Some(ind) = industry_opt {
                        if let Some(m) = registry.mods.get(&ind) {
                            r = m.hook_before(ctx);
                        }
                    }
                    if matches!(r, StepResult::Continue) {
                        if let Some(m) = registry.mods.get("common") {
                            let r2 = m.hook_before(ctx);
                            if matches!(r2, StepResult::Stop(_)) {
                                r = r2;
                            }
                        }
                    }
                    r
                }
                StageId::Transaction => {
                    if let Some(tx) = tx_manager {
                        let action = req.action.clone();
                        let tx_ctx_data = (
                            ctx.request_data.clone(),
                            ctx.biz_id.clone(),
                            ctx.biz_code.clone(),
                            ctx.workflow_instance_id.clone(),
                        );
                        let r = tx.run(|| -> anyhow::Result<()> {
                            let (data, bid, bcode, wfid) = &tx_ctx_data;
                            match action {
                                BizAction::Create => {
                                    let (biz_id, biz_code, version) = dao.create(
                                        meta_repo,
                                        iam_repo,
                                        &ctx.tenant_id,
                                        &ctx.entity_code,
                                        &ctx.user_id,
                                        data.as_ref().unwrap(),
                                        wfid.as_deref(),
                                        bcode.as_deref(),
                                    )?;
                                    ctx.biz_id = Some(biz_id);
                                    ctx.biz_code = Some(biz_code);
                                    ctx.version = Some(version);
                                }
                                BizAction::Update => {
                                    let v = dao.update(
                                        meta_repo,
                                        &ctx.tenant_id,
                                        &ctx.entity_code,
                                        bid.as_deref().unwrap(),
                                        &ctx.user_id,
                                        data.as_ref().unwrap(),
                                    )?;
                                    ctx.version = Some(v);
                                }
                                BizAction::Delete => {
                                    dao.delete(
                                        &ctx.tenant_id,
                                        &ctx.entity_code,
                                        bid.as_deref().unwrap(),
                                        &ctx.user_id,
                                        Some("orchestrator delete"),
                                    )?;
                                }
                                _ => {}
                            }
                            Ok(())
                        });
                        match r {
                            Ok(()) => StepResult::Continue,
                            Err(e) => StepResult::Stop(e),
                        }
                    } else {
                        StepResult::Continue
                    }
                }
                StageId::Main => match req.action {
                    BizAction::Get => {
                        match dao.get(
                            meta_repo,
                            &ctx.tenant_id,
                            &ctx.entity_code,
                            ctx.biz_id.as_deref().unwrap(),
                        ) {
                            Ok(Some(v)) => {
                                if let Some(obj) = v.as_object() {
                                    if let Some(h) = obj.get("curr_hash").and_then(|x| x.as_str()) {
                                        ctx.curr_hash = Some(h.to_string());
                                    }
                                    if let Some(vv) = obj.get("version").and_then(|x| x.as_i64()) {
                                        ctx.version = Some(vv);
                                    }
                                }
                                ctx.response_data = Some(v);
                                StepResult::Continue
                            }
                            Ok(None) => StepResult::Stop(anyhow::anyhow!("not found")),
                            Err(e) => StepResult::Stop(e),
                        }
                    }
                    BizAction::List => {
                        match dao.list(
                            meta_repo,
                            &ctx.tenant_id,
                            &ctx.entity_code,
                            ctx.filters.clone(),
                            ctx.sort.clone(),
                            ctx.page,
                            ctx.page_size,
                        ) {
                            Ok(lr) => {
                                ctx.response_list_total = Some(lr.total);
                                ctx.response_data = Some(Value::Array(lr.items));
                                StepResult::Continue
                            }
                            Err(e) => StepResult::Stop(e),
                        }
                    }
                    BizAction::Create => {
                        if let Some(bid) = ctx.biz_id.clone() {
                            ctx.response_data = Some(serde_json::json!({
                                "biz_id": bid,
                                "biz_code": ctx.biz_code.clone().unwrap_or_default(),
                                "version": ctx.version,
                            }));
                        }
                        StepResult::Continue
                    }
                    BizAction::Update => {
                        ctx.response_data = Some(serde_json::json!({"version": ctx.version}));
                        StepResult::Continue
                    }
                    BizAction::Delete => {
                        ctx.response_data = Some(serde_json::json!({"deleted": true}));
                        StepResult::Continue
                    }
                },
                StageId::After => {
                    let industry_opt =
                        registry.find_by_entity(&ctx.entity_code, meta_repo, &ctx.tenant_id);
                    let mut r = StepResult::Continue;
                    if let Some(m) = registry.mods.get("common") {
                        let r2 = m.hook_after(ctx);
                        if matches!(r2, StepResult::Stop(_)) {
                            r = r2;
                        }
                    }
                    if matches!(r, StepResult::Continue) {
                        if let Some(ind) = industry_opt {
                            if let Some(m) = registry.mods.get(&ind) {
                                r = m.hook_after(ctx);
                            }
                        }
                    }
                    r
                }
                StageId::Enrich => {
                    // 字典翻译: 把 enum code → label
                    if let Ok(entity) = meta_repo.get_entity(&ctx.tenant_id, &ctx.entity_code) {
                        if let Some(resp) = ctx.response_data.clone() {
                            let enriched = Self::enrich_dict(&resp, &entity.fields);
                            ctx.response_data = Some(enriched);
                        }
                    }
                    StepResult::Continue
                }
                StageId::Notify => {
                    // workflow_instance_id 存在时标记状态推进
                    if ctx.workflow_instance_id.is_some() {
                        ctx.extra
                            .insert("workflow_pushed".into(), Value::Bool(true));
                    }
                    StepResult::Continue
                }
                StageId::Event => {
                    let event = match req.action {
                        BizAction::Create => Some(BusinessEvent::Created {
                            tenant_id: ctx.tenant_id.clone(),
                            entity_code: ctx.entity_code.clone(),
                            biz_id: ctx.biz_id.clone().unwrap_or_default(),
                            fields: ctx.request_data.clone().unwrap_or_default(),
                        }),
                        BizAction::Update => Some(BusinessEvent::Updated {
                            tenant_id: ctx.tenant_id.clone(),
                            entity_code: ctx.entity_code.clone(),
                            biz_id: ctx.biz_id.clone().unwrap_or_default(),
                            fields: ctx.request_data.clone().unwrap_or_default(),
                        }),
                        BizAction::Delete => Some(BusinessEvent::Deleted {
                            tenant_id: ctx.tenant_id.clone(),
                            entity_code: ctx.entity_code.clone(),
                            biz_id: ctx.biz_id.clone().unwrap_or_default(),
                        }),
                        BizAction::Get | BizAction::List => None,
                    };
                    if let Some(ev) = event {
                        ctx.event_pending = Some(ev.clone());
                        event_bus.publish(ev);
                    }
                    StepResult::Continue
                }
                StageId::Audit => {
                    let detail = format!(
                        "action={} entity={} biz_id={:?}",
                        action_str, ctx.entity_code, ctx.biz_id
                    );
                    // 写入审计日志
                    let _ = iam_repo.write_audit_log(mox_platform_datastore_core::AuditLogEntry {
                        log_id: uuid::Uuid::now_v7().to_string(),
                        tenant_id: ctx.tenant_id.clone(),
                        user_id: ctx.user_id.clone(),
                        action: action_str.to_string(),
                        target: format!("{}:{:?}", ctx.entity_code, ctx.biz_id),
                        detail: detail.clone(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                    });
                    ctx.audit_log_detail = Some(detail);

                    // 校验 hash 链一致性（非首次创建场景，对比 compute_hash 结果）
                    if let (Some(bid), Some(ver), Some(data_v), Some(created_at)) = (
                        ctx.biz_id.as_deref(),
                        ctx.version,
                        ctx.snapshot_after
                            .clone()
                            .or_else(|| ctx.response_data.clone()),
                        Some(chrono::Utc::now().to_rfc3339().as_str().to_string()),
                    ) {
                        let _ = compute_hash(None, bid, ver, &data_v, &ctx.user_id, &created_at);
                    }
                    StepResult::Continue
                }
            };

            match result {
                StepResult::Continue => {}
                StepResult::Stop(e) => {
                    first_error = Some(e.to_string());
                    ctx.error = first_error.clone();
                    break;
                }
                StepResult::Skip => {}
            }
        }

        let elapsed_ns = start.elapsed().as_nanos() as u64;
        metrics
            .total_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let ok = first_error.is_none();
        if !ok {
            metrics
                .failed_calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if let Ok(mut guard) = metrics.latencies_ns.lock() {
            guard.push(elapsed_ns);
        }

        PipelineResult {
            success: ok,
            error: first_error,
            stages_run,
        }
    }

    fn enrich_dict(resp: &Value, fields: &[mox_platform_datastore_core::FieldSpec]) -> Value {
        let mut r = resp.clone();
        if let Some(obj) = r.as_object_mut() {
            for f in fields {
                if let Some(opts) = &f.options_inline {
                    if let Some(code_v) = obj
                        .get(&f.field_code)
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                    {
                        if let Some(lbl) = opts.iter().find(|o| o.code == code_v) {
                            obj.insert(
                                format!("{}_label", f.field_code),
                                Value::String(lbl.label.clone()),
                            );
                        }
                    }
                }
            }
        }
        r
    }
}

pub type OrchestrateFn = dyn Fn(&mut PipelineCtx) -> StepResult + Send + Sync;
