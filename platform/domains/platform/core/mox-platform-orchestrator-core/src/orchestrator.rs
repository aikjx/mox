//! 企业级业务编排器（Orchestrator）
//!
//! 10阶段Pipeline：接收→鉴权→校验→元数据解析→字典翻译→业务执行→结果enrich→审计→事件发布→响应构建
//! 集成 UniversalBizDAO + TxManager + InMemoryMetaRepo + InMemoryIamRepo

use mox_platform_datastore_core::{
    AuditLog, FieldSpec, Filter, InMemoryIamRepo, InMemoryMetaRepo, SortSpec, TxManager,
    UniversalBizDAO,
};
use mox_platform_iam_core::IamRepository;
use mox_platform_meta_core::MetaRepository;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════
// 业务动作枚举
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BizAction {
    Create,
    Get,
    Update,
    List,
    Delete,
}

impl BizAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            BizAction::Create => "create",
            BizAction::Get => "get",
            BizAction::Update => "update",
            BizAction::List => "list",
            BizAction::Delete => "delete",
        }
    }

    /// 所需权限
    pub fn required_permission(&self) -> &'static str {
        match self {
            BizAction::Create => "biz:create",
            BizAction::Get => "biz:read",
            BizAction::Update => "biz:update",
            BizAction::List => "biz:list",
            BizAction::Delete => "biz:delete",
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// 业务请求
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRequest {
    pub tenant_id: String,
    pub user_id: String,
    pub entity_code: String,
    pub action: BizAction,
    pub biz_id: Option<String>,
    pub biz_code: Option<String>,
    pub workflow_instance_id: Option<String>,
    pub data: Option<Map<String, Value>>,
    pub filters: Vec<Filter>,
    pub sort: SortSpec,
    pub page: i64,
    pub page_size: i64,
}

impl Default for BusinessRequest {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            user_id: String::new(),
            entity_code: String::new(),
            action: BizAction::Get,
            biz_id: None,
            biz_code: None,
            workflow_instance_id: None,
            data: None,
            filters: vec![],
            sort: SortSpec::default(),
            page: 1,
            page_size: 10,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Pipeline 阶段
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub name: String,
    pub status: StageStatus,
    pub duration_ms: u64,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStatus {
    Success,
    Failed,
    Skipped,
}

impl PipelineStage {
    fn success(name: &str, duration_ms: u64) -> Self {
        Self { name: name.to_string(), status: StageStatus::Success, duration_ms, detail: None }
    }
    fn failed(name: &str, duration_ms: u64, detail: &str) -> Self {
        Self { name: name.to_string(), status: StageStatus::Failed, duration_ms, detail: Some(detail.to_string()) }
    }
}

// ═══════════════════════════════════════════════════════════════
// 业务响应
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessResponse {
    pub success: bool,
    pub biz_id: Option<String>,
    pub biz_code: Option<String>,
    pub version: Option<i64>,
    pub data: Option<Value>,
    pub total: Option<i64>,
    pub error: Option<String>,
    pub pipeline_stages: Vec<PipelineStage>,
}

impl BusinessResponse {
    fn fail(error: impl Into<String>, stages: Vec<PipelineStage>) -> Self {
        Self {
            success: false,
            biz_id: None,
            biz_code: None,
            version: None,
            data: None,
            total: None,
            error: Some(error.into()),
            pipeline_stages: stages,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// 指标收集器
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Default, Clone)]
pub struct Metrics {
    inner: std::sync::Arc<Mutex<MetricsInner>>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    total: u64,
    success: u64,
    failed: u64,
    durations_ms: Vec<u64>,
}

impl Metrics {
    pub fn record(&self, success: bool, duration_ms: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.total += 1;
        if success { inner.success += 1; } else { inner.failed += 1; }
        inner.durations_ms.push(duration_ms);
    }

    pub fn total(&self) -> u64 {
        self.inner.lock().unwrap().total
    }

    pub fn fail_rate(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        if inner.total == 0 { 0.0 } else { inner.failed as f64 / inner.total as f64 }
    }

    pub fn p50(&self) -> Option<u64> {
        let inner = self.inner.lock().unwrap();
        if inner.durations_ms.is_empty() { return None; }
        let mut sorted = inner.durations_ms.clone();
        sorted.sort();
        let idx = sorted.len() / 2;
        Some(sorted[idx])
    }

    pub fn p99(&self) -> Option<u64> {
        let inner = self.inner.lock().unwrap();
        if inner.durations_ms.is_empty() { return None; }
        let mut sorted = inner.durations_ms.clone();
        sorted.sort();
        let idx = (sorted.len() as f64 * 0.99) as usize;
        Some(sorted[idx.min(sorted.len() - 1)])
    }
}

// ═══════════════════════════════════════════════════════════════
// 事件总线
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct EventBus {
    queue: std::sync::Arc<Mutex<Vec<BusinessEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessEvent {
    pub event_type: String,
    pub tenant_id: String,
    pub entity_code: String,
    pub biz_id: Option<String>,
    pub timestamp: String,
    pub payload: Value,
}

impl EventBus {
    pub fn new() -> Self {
        Self { queue: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }

    pub fn publish(&self, event: BusinessEvent) {
        let mut queue = self.queue.lock().unwrap();
        queue.push(event);
    }

    pub fn queue_len(&self) -> usize {
        self.queue.lock().unwrap().len()
    }

    pub fn drain(&self) -> Vec<BusinessEvent> {
        let mut queue = self.queue.lock().unwrap();
        std::mem::take(&mut *queue)
    }
}

impl Default for EventBus {
    fn default() -> Self { Self::new() }
}

// ═══════════════════════════════════════════════════════════════
// 字典翻译表
// ═══════════════════════════════════════════════════════════════

fn status_dict_label(status: &str) -> Option<&'static str> {
    match status {
        "draft" => Some("草稿"),
        "active" => Some("进行中"),
        "closed" => Some("已关闭"),
        "pending" => Some("待处理"),
        "approved" => Some("已批准"),
        "rejected" => Some("已拒绝"),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════
// 企业级业务编排器
// ═══════════════════════════════════════════════════════════════

#[derive(Clone)]
pub struct Orchestrator {
    pub metrics: Metrics,
    pub event_bus: EventBus,
    dict_cache: HashMap<String, HashMap<String, String>>,
    /// 企业级 SQLite 后端元数据仓库（可选）
    meta: Option<std::sync::Arc<MetaRepository>>,
    /// 企业级通用 DAO（可选）
    dao: Option<std::sync::Arc<UniversalBizDAO>>,
    /// 企业级 IAM 仓库（可选）
    iam: Option<std::sync::Arc<IamRepository>>,
    /// 已注册的 pipeline 名称
    pipelines: std::sync::Arc<parking_lot::Mutex<Vec<String>>>,
}

impl Orchestrator {
    /// 企业级默认配置
    pub fn enterprise_default() -> Self {
        let mut dict_cache = HashMap::new();
        // 预置 status 字典
        let mut status_dict = HashMap::new();
        status_dict.insert("draft".to_string(), "草稿".to_string());
        status_dict.insert("active".to_string(), "进行中".to_string());
        status_dict.insert("closed".to_string(), "已关闭".to_string());
        dict_cache.insert("status".to_string(), status_dict);

        Self {
            metrics: Metrics::default(),
            event_bus: EventBus::new(),
            dict_cache,
            meta: None,
            dao: None,
            iam: None,
            pipelines: std::sync::Arc::new(parking_lot::Mutex::new(Vec::new())),
        }
    }

    /// 企业级构造函数（SQLite 后端）
    pub fn new(meta: std::sync::Arc<MetaRepository>, dao: std::sync::Arc<UniversalBizDAO>) -> Self {
        let mut orch = Self::enterprise_default();
        orch.meta = Some(meta);
        orch.dao = Some(dao);
        orch
    }

    /// 设置 IAM 仓库
    pub fn with_iam(mut self, iam: std::sync::Arc<IamRepository>) -> Self {
        self.iam = Some(iam);
        self
    }

    /// 注册 pipeline
    pub fn register_pipeline(&mut self, name: &str) {
        self.pipelines.lock().push(name.to_string());
    }

    /// 执行业务请求（10阶段Pipeline）
    pub fn execute(
        &self,
        req: &BusinessRequest,
        dao: &UniversalBizDAO,
        tx: Option<&TxManager>,
        meta: &InMemoryMetaRepo,
        iam: &InMemoryIamRepo,
    ) -> BusinessResponse {
        let start = Instant::now();
        let mut stages: Vec<PipelineStage> = Vec::with_capacity(10);
        let mut stage_start = Instant::now();

        // Stage 1: 接收请求
        stages.push(PipelineStage::success("receive", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 2: 鉴权
        let perm = req.action.required_permission();
        if !iam.has_permission(&req.tenant_id, &req.user_id, perm) {
            stages.push(PipelineStage::failed(
                "auth",
                stage_start.elapsed().as_millis() as u64,
                &format!("Permission denied: user {} lacks {}", req.user_id, perm),
            ));
            let resp = BusinessResponse::fail(
                format!("Permission denied: user {} lacks {}", req.user_id, perm),
                stages,
            );
            self.metrics.record(false, start.elapsed().as_millis() as u64);
            return resp;
        }
        stages.push(PipelineStage::success("auth", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 3: 校验
        if req.entity_code.is_empty() {
            stages.push(PipelineStage::failed("validate", 0, "entity_code is empty"));
            let resp = BusinessResponse::fail("entity_code is empty", stages);
            self.metrics.record(false, start.elapsed().as_millis() as u64);
            return resp;
        }
        if matches!(req.action, BizAction::Get | BizAction::Update | BizAction::Delete) && req.biz_id.is_none() {
            stages.push(PipelineStage::failed("validate", 0, "biz_id is required for this action"));
            let resp = BusinessResponse::fail("biz_id is required", stages);
            self.metrics.record(false, start.elapsed().as_millis() as u64);
            return resp;
        }
        stages.push(PipelineStage::success("validate", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 4: 元数据解析
        let fields = meta.get_entity_fields(&req.tenant_id, &req.entity_code);
        stages.push(PipelineStage::success(
            "meta_resolve",
            stage_start.elapsed().as_millis() as u64,
        ));
        stage_start = Instant::now();

        // Stage 5: 字典翻译（准备）
        stages.push(PipelineStage::success("dict_translate", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 6: 业务执行
        let exec_result = self.execute_business_action(req, dao, tx, meta, iam, &fields);
        let exec_duration = stage_start.elapsed().as_millis() as u64;

        match &exec_result {
            Ok(_) => stages.push(PipelineStage::success("execute", exec_duration)),
            Err(e) => {
                stages.push(PipelineStage::failed("execute", exec_duration, e));
                let resp = BusinessResponse::fail(e.clone(), stages);
                self.metrics.record(false, start.elapsed().as_millis() as u64);
                return resp;
            }
        }
        stage_start = Instant::now();

        // Stage 7: 结果 enrich（字典翻译）
        let exec_data = exec_result.unwrap();
        let enriched = self.enrich_result(&exec_data, &fields);
        stages.push(PipelineStage::success("enrich", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 8: 审计记录
        let audit = AuditLog {
            timestamp: chrono::Utc::now().to_rfc3339(),
            tenant_id: req.tenant_id.clone(),
            user_id: req.user_id.clone(),
            action: req.action.as_str().to_string(),
            entity_code: req.entity_code.clone(),
            biz_id: req.biz_id.clone().or_else(|| enriched.biz_id.clone()),
            success: true,
            detail: format!("entity={}, action={}", req.entity_code, req.action.as_str()),
        };
        iam.append_audit(audit);
        stages.push(PipelineStage::success("audit", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 9: 事件发布
        let event = BusinessEvent {
            event_type: format!("biz.{}", req.action.as_str()),
            tenant_id: req.tenant_id.clone(),
            entity_code: req.entity_code.clone(),
            biz_id: req.biz_id.clone().or_else(|| enriched.biz_id.clone()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload: enriched.data.clone().unwrap_or(Value::Null),
        };
        self.event_bus.publish(event);
        stages.push(PipelineStage::success("event_publish", stage_start.elapsed().as_millis() as u64));
        stage_start = Instant::now();

        // Stage 10: 响应构建
        stages.push(PipelineStage::success("response", stage_start.elapsed().as_millis() as u64));

        let total_duration = start.elapsed().as_millis() as u64;
        self.metrics.record(true, total_duration);

        BusinessResponse {
            success: true,
            biz_id: enriched.biz_id,
            biz_code: enriched.biz_code,
            version: enriched.version,
            data: enriched.data,
            total: enriched.total,
            error: None,
            pipeline_stages: stages,
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 内部：业务动作执行
    // ═══════════════════════════════════════════════════════════

    fn execute_business_action(
        &self,
        req: &BusinessRequest,
        dao: &UniversalBizDAO,
        tx: Option<&TxManager>,
        meta: &InMemoryMetaRepo,
        iam: &InMemoryIamRepo,
        _fields: &[FieldSpec],
    ) -> Result<ExecOutput, String> {
        match req.action {
            BizAction::Create => {
                let data = req.data.as_ref().ok_or("data is required for create")?;
                let do_create = || {
                    dao.create(
                        meta,
                        iam,
                        &req.tenant_id,
                        &req.entity_code,
                        &req.user_id,
                        data,
                        req.biz_code.as_deref(),
                        req.workflow_instance_id.as_deref(),
                    )
                };

                let result = if let Some(tx_mgr) = tx {
                    tx_mgr.run(do_create).map_err(|e| e.to_string())?
                } else {
                    do_create().map_err(|e| e.to_string())?
                };

                Ok(ExecOutput {
                    biz_id: Some(result.0),
                    biz_code: Some(result.1),
                    version: Some(result.2),
                    data: None,
                    total: None,
                })
            }
            BizAction::Get => {
                let biz_id = req.biz_id.as_ref().ok_or("biz_id required")?;
                let val = dao.get(meta, &req.tenant_id, &req.entity_code, biz_id)
                    .map_err(|e| e.to_string())?;
                Ok(ExecOutput {
                    biz_id: Some(biz_id.clone()),
                    biz_code: None,
                    version: None,
                    data: val,
                    total: None,
                })
            }
            BizAction::Update => {
                let biz_id = req.biz_id.as_ref().ok_or("biz_id required")?;
                let patch = req.data.as_ref().ok_or("data(patch) is required for update")?;
                let do_update = || {
                    dao.update(meta, &req.tenant_id, &req.entity_code, biz_id, &req.user_id, patch)
                };
                let version = if let Some(tx_mgr) = tx {
                    tx_mgr.run(do_update).map_err(|e| e.to_string())?
                } else {
                    do_update().map_err(|e| e.to_string())?
                };
                Ok(ExecOutput {
                    biz_id: Some(biz_id.clone()),
                    biz_code: None,
                    version: Some(version),
                    data: None,
                    total: None,
                })
            }
            BizAction::List => {
                let list_result = dao.list(
                    meta,
                    &req.tenant_id,
                    &req.entity_code,
                    req.filters.clone(),
                    req.sort.clone(),
                    req.page,
                    req.page_size,
                ).map_err(|e| e.to_string())?;
                Ok(ExecOutput {
                    biz_id: None,
                    biz_code: None,
                    version: None,
                    data: Some(Value::Array(list_result.items)),
                    total: Some(list_result.total),
                })
            }
            BizAction::Delete => {
                let biz_id = req.biz_id.as_ref().ok_or("biz_id required")?;
                let do_delete = || {
                    dao.delete(&req.tenant_id, &req.entity_code, biz_id, &req.user_id, Some("orchestrator delete"))
                };
                if let Some(tx_mgr) = tx {
                    tx_mgr.run(do_delete).map_err(|e| e.to_string())?;
                } else {
                    do_delete().map_err(|e| e.to_string())?;
                }
                Ok(ExecOutput {
                    biz_id: Some(biz_id.clone()),
                    biz_code: None,
                    version: None,
                    data: None,
                    total: None,
                })
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // 内部：结果 enrich（字典翻译）
    // ═══════════════════════════════════════════════════════════

    fn enrich_result(&self, output: &ExecOutput, _fields: &[FieldSpec]) -> ExecOutput {
        let enriched_data = match &output.data {
            Some(Value::Object(map)) => {
                let mut new_map = map.clone();
                // status 字典翻译
                if let Some(Value::String(status)) = map.get("status") {
                    if let Some(label) = status_dict_label(status) {
                        new_map.insert("status_label".to_string(), Value::String(label.to_string()));
                    }
                }
                Some(Value::Object(new_map))
            }
            Some(Value::Array(items)) => {
                let enriched_items: Vec<Value> = items.iter().map(|item| {
                    if let Value::Object(map) = item {
                        let mut new_map = map.clone();
                        if let Some(Value::String(status)) = map.get("status") {
                            if let Some(label) = status_dict_label(status) {
                                new_map.insert("status_label".to_string(), Value::String(label.to_string()));
                            }
                        }
                        Value::Object(new_map)
                    } else {
                        item.clone()
                    }
                }).collect();
                Some(Value::Array(enriched_items))
            }
            other => other.clone(),
        };

        ExecOutput {
            biz_id: output.biz_id.clone(),
            biz_code: output.biz_code.clone(),
            version: output.version,
            data: enriched_data,
            total: output.total,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// 内部类型
// ═══════════════════════════════════════════════════════════════

struct ExecOutput {
    biz_id: Option<String>,
    biz_code: Option<String>,
    version: Option<i64>,
    data: Option<Value>,
    total: Option<i64>,
}

// ═══════════════════════════════════════════════════════════════
// 企业级便捷同步方法（SQLite 后端，enterprise-svc 使用）
// ═══════════════════════════════════════════════════════════════

/// 业务记录（便捷方法返回类型）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BizRecord {
    pub biz_id: String,
    pub biz_code: Option<String>,
    pub version: Option<i64>,
    pub data: Option<Value>,
    pub entity_code: Option<String>,
    pub tenant_id: Option<String>,
    pub status: Option<String>,
}

impl Orchestrator {
    /// 确保 dao 已设置
    fn require_dao(&self) -> anyhow::Result<&UniversalBizDAO> {
        self.dao.as_deref().ok_or_else(|| anyhow::anyhow!("Orchestrator dao not configured"))
    }

    /// 创建业务记录（同步）
    pub fn create_sync(
        &self,
        entity_code: &str,
        tenant_id: Option<String>,
        data: Option<Value>,
        actor: &str,
    ) -> anyhow::Result<BizRecord> {
        let dao = self.require_dao()?;
        let conn = dao.conn().lock();
        let tenant = tenant_id.unwrap_or_else(|| "default".to_string());
        let biz_id = uuid::Uuid::new_v4().to_string();
        let biz_code = format!("{}-{}", entity_code, &biz_id[..8.min(biz_id.len())]);
        let version: i64 = 1;
        let now = chrono::Utc::now().to_rfc3339();
        let data_json = data.map(|d| serde_json::to_string(&d).unwrap_or_default()).unwrap_or_default();

        conn.execute(
            "INSERT INTO biz_data (biz_id, tenant_id, biz_type, biz_code, version, status, data, created_by, created_at, updated_by, updated_at, is_deleted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0)",
            rusqlite::params![biz_id, tenant, entity_code, biz_code, version, "active", data_json, actor, now, actor, now],
        )?;

        self.metrics.record(true, 0);
        Ok(BizRecord {
            biz_id,
            biz_code: Some(biz_code),
            version: Some(version),
            data: Some(Value::String(data_json)),
            entity_code: Some(entity_code.to_string()),
            tenant_id: Some(tenant),
            status: Some("active".to_string()),
        })
    }

    /// 更新业务记录（同步）
    pub fn update_sync(
        &self,
        biz_id: &str,
        patch: Option<Value>,
        actor: &str,
    ) -> anyhow::Result<BizRecord> {
        let dao = self.require_dao()?;
        let conn = dao.conn().lock();
        let now = chrono::Utc::now().to_rfc3339();

        // 获取当前版本
        let current_version: i64 = conn.query_row(
            "SELECT version FROM biz_data WHERE biz_id = ?1 AND is_deleted = 0",
            [biz_id],
            |row| row.get(0),
        )?;
        let new_version = current_version + 1;

        // 合并数据
        if let Some(patch_value) = patch {
            let current_data: String = conn.query_row(
                "SELECT data FROM biz_data WHERE biz_id = ?1",
                [biz_id],
                |row| row.get(0),
            ).unwrap_or_default();
            let mut current: Map<String, Value> = serde_json::from_str(&current_data).unwrap_or_default();
            if let Value::Object(patch_map) = patch_value {
                for (k, v) in patch_map {
                    current.insert(k, v);
                }
            }
            let new_data = serde_json::to_string(&current).unwrap_or_default();
            conn.execute(
                "UPDATE biz_data SET data = ?1, version = ?2, updated_by = ?3, updated_at = ?4 WHERE biz_id = ?5",
                rusqlite::params![new_data, new_version, actor, now, biz_id],
            )?;
        } else {
            conn.execute(
                "UPDATE biz_data SET version = ?1, updated_by = ?2, updated_at = ?3 WHERE biz_id = ?4",
                rusqlite::params![new_version, actor, now, biz_id],
            )?;
        }

        self.metrics.record(true, 0);
        Ok(BizRecord {
            biz_id: biz_id.to_string(),
            biz_code: None,
            version: Some(new_version),
            data: None,
            entity_code: None,
            tenant_id: None,
            status: None,
        })
    }

    /// 删除业务记录（同步，软删除）
    pub fn delete_sync(&self, biz_id: &str, actor: &str) -> anyhow::Result<()> {
        let dao = self.require_dao()?;
        let conn = dao.conn().lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE biz_data SET is_deleted = 1, status = 'deleted', updated_by = ?1, updated_at = ?2 WHERE biz_id = ?3",
            rusqlite::params![actor, now, biz_id],
        )?;
        self.metrics.record(true, 0);
        Ok(())
    }

    /// 获取业务记录（同步）
    pub fn get_sync(&self, biz_id: &str) -> anyhow::Result<Option<BizRecord>> {
        let dao = self.require_dao()?;
        let conn = dao.conn().lock();
        let result = conn.query_row(
            "SELECT biz_id, biz_code, version, data, biz_type, tenant_id, status FROM biz_data WHERE biz_id = ?1 AND is_deleted = 0",
            [biz_id],
            |row| {
                let data_str: String = row.get(3)?;
                let data_val: Value = serde_json::from_str(&data_str).unwrap_or(Value::Null);
                Ok(BizRecord {
                    biz_id: row.get(0)?,
                    biz_code: row.get(1)?,
                    version: row.get(2)?,
                    data: Some(data_val),
                    entity_code: row.get(4)?,
                    tenant_id: row.get(5)?,
                    status: row.get(6)?,
                })
            },
        );
        match result {
            Ok(rec) => Ok(Some(rec)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    /// 列出业务记录（同步）
    pub fn list_sync(&self, entity_code: &str, tenant_id: Option<&str>) -> anyhow::Result<Vec<BizRecord>> {
        let dao = self.require_dao()?;
        let conn = dao.conn().lock();
        let mut sql = "SELECT biz_id, biz_code, version, data, biz_type, tenant_id, status FROM biz_data WHERE is_deleted = 0 AND biz_type = ?1".to_string();
        let mut params: Vec<String> = vec![entity_code.to_string()];
        if let Some(tid) = tenant_id {
            sql.push_str(" AND tenant_id = ?2");
            params.push(tid.to_string());
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 100");

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let data_str: String = row.get(3)?;
            let data_val: Value = serde_json::from_str(&data_str).unwrap_or(Value::Null);
            Ok(BizRecord {
                biz_id: row.get(0)?,
                biz_code: row.get(1)?,
                version: row.get(2)?,
                data: Some(data_val),
                entity_code: row.get(4)?,
                tenant_id: row.get(5)?,
                status: row.get(6)?,
            })
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// 获取版本数量（同步）
    pub fn version_count_sync(&self, biz_id: &str) -> usize {
        let dao = match self.require_dao() { Ok(d) => d, Err(_) => return 0 };
        let conn = dao.conn().lock();
        conn.query_row(
            "SELECT version FROM biz_data WHERE biz_id = ?1",
            [biz_id],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as usize
    }

    /// 获取审计链（同步）
    pub fn audit_chain_sync(&self, biz_id: &str) -> Vec<Value> {
        // 简化实现：返回空数组，实际应从审计日志表查询
        let _ = biz_id;
        Vec::new()
    }
}

// Metrics 补充 failed 方法
impl Metrics {
    pub fn failed(&self) -> u64 {
        self.inner.lock().unwrap().failed
    }
}
