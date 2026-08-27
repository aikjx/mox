// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! OperatorEngine：算子注册表 + 三策略 dispatch_intent 主调度（对齐 Python operator.base.OperatorEngine）
//!
//! 流水线（对应 alliance-fr13-fr5-integration.md §2 S1→S4→S5）：
//!
//! ```text
//! dispatch_intent(text, identity, mode)
//!   │
//!   ▼ S1 意图路由（xiaobai-intent crate，Engine 回调 router.dispatch）
//!   │   ├─ 唯一命中 (action, category, score, confidence_delta) → 继续
//!   │   ├─ 歧义 delta ≤ AMBIGUITY_THRESHOLD → 三策略：
//!   │   │    LocalFirst 本地高分直干 + 异步上报联盟求裁决解释
//!   │   │    CloudFallback 800ms 等联盟 → 超时本地高分
//!   │   │    CloudOnly 强制联盟 → 超时 XB-006
//!   │   └─ 全未命中 → XB-004 IntentUnknown
//!   │
//!   ▼ 查注册表：action → (operator 引用, ActionSignature)
//!   │   ├─ clearance_required 计算（若 file 且命中 PII 敏感资源 → 强制 +1）
//!   │   ├─ check_clearance 鉴权 → 失败 XB-001
//!   │   └─ 三策略模式下调用联盟事前裁决闸（reconcile Parallelize⋀MustSerialize → XB-011）
//!   │
//!   ▼ 执行 operator.execute(action, param, identity)
//!   │   ├─ spawn_blocking 保护阻塞系统调用（Win32/FS/音频/截屏）
//!   │   ├─ 成功 → 封装 ExecPayload + Envelope
//!   │   └─ 失败 → XB-007 OperatorUnsupported 或 XB-010 ExecutionError
//!   │
//!   ▼ S5 审计回调（audit_fn：入审计流；失败记 XB-008 但不阻塞用户返回）
//!   │
//!   ▼ 返回 DispatchIntentResult：{action, executed_where, output, audit_id}
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{AMBIGUITY_THRESHOLD, BRIDGE_CLOUD_DEADLINE_MS, PII_SENSITIVE_FORCE_LEVEL};
pub use crate::errors::XiaobaiResult;
use crate::errors::{ClearanceLevelRepr, XiaobaiError};
use crate::identity::OperatorIdentity;
use crate::operator::{ActionSignature, OperatorCategory, OperatorOutput, SystemOperator};
use crate::protocol::{AuditPayload, Envelope};
use crate::rbac::{check_clearance, ClearanceLevel, DispatchMode};

// ==================== Router 回调接口 ====================

/// 意图路由结果（xiaobai-intent 返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedAction {
    pub action: String,
    pub category: String,
    pub score: f32,
    /// top1 - top2 的差值；Engine 用这个做歧义判断
    pub confidence_delta: f32,
    /// 提取到的动作参数（如 app_name="chrome" / path="C:/a.txt" / volume_pct=33）
    pub param: serde_json::Value,
}

/// 意图路由抽象（xiaobai-intent 实现；Engine 不依赖具体路由实现便于 P2 替换为 mox-intent-core PPR 精确图谱）
#[async_trait]
pub trait IntentRouter: Send + Sync {
    async fn dispatch(&self, text: &str) -> XiaobaiResult<Vec<RoutedAction>>;
    fn ambiguity_threshold(&self) -> f32 {
        AMBIGUITY_THRESHOLD
    }
}

/// 联盟裁决客户端回调（cloud_fallback / cloud_only 模式下调用；LocalFirst 异步）
#[async_trait]
pub trait AllianceJudgeClient: Send + Sync {
    /// 返回 Ok(verdict: String, reasons: Vec<String>)；Err 是 BRIDGE_DISCONNECTED
    async fn ask_verdict(
        &self,
        envelope: &Envelope,
        candidate_actions: &[RoutedAction],
    ) -> XiaobaiResult<(String, Vec<String>)>;
}

/// 审计回调（每次 dispatch 成功/失败都会调用；LocalFirst 可用内存 stub 收集 selftest 审计链）
pub type AuditFn = Arc<dyn Fn(AuditPayload) -> XiaobaiResult<()> + Send + Sync>;

// ==================== Engine 配置与主结构 ====================

#[derive(Clone)]
pub struct EngineConfig {
    pub mode: DispatchMode,
    pub nonce_ttl: Duration,
    pub cloud_deadline: Duration,
    pub audit_fn: AuditFn,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            mode: DispatchMode::LocalFirst,
            nonce_ttl: Duration::from_secs(30),
            cloud_deadline: Duration::from_millis(BRIDGE_CLOUD_DEADLINE_MS),
            audit_fn: Arc::new(|_| Ok(())),
        }
    }
}

/// 算子注册表条目：`(Arc<dyn SystemOperator>, ActionSignature)`
type RegEntry = (Arc<dyn SystemOperator>, ActionSignature);

pub struct OperatorEngine {
    config: EngineConfig,
    registry: RwLock<BTreeMap<String, RegEntry>>,
    nonce_log: RwLock<BTreeMap<String, Instant>>,
    router: Arc<dyn IntentRouter>,
    alliance: Option<Arc<dyn AllianceJudgeClient>>,
}

impl OperatorEngine {
    pub fn new(config: EngineConfig, router: Arc<dyn IntentRouter>) -> Self {
        Self {
            config,
            registry: RwLock::new(BTreeMap::new()),
            nonce_log: RwLock::new(BTreeMap::new()),
            router,
            alliance: None,
        }
    }

    pub fn with_alliance(mut self, client: Arc<dyn AllianceJudgeClient>) -> Self {
        self.alliance = Some(client);
        self
    }

    /// 注册一个 SystemOperator（把它的 list_actions 全塞进注册表）
    pub fn register(&self, op: Arc<dyn SystemOperator>) {
        let mut w = self.registry.write();
        for sig in op.list_actions() {
            let key = sig.name.to_string();
            w.insert(key, (op.clone(), sig));
        }
    }

    pub fn list_registered_actions(&self) -> Vec<(String, OperatorCategory, u8)> {
        let r = self.registry.read();
        r.iter()
            .map(|(k, (_, sig))| (k.clone(), sig.category, sig.clearance.as_u8()))
            .collect()
    }

    /// 幂等检查 + 清理 30s 过期 nonce
    fn idempotent_check_and_mark(&self, nonce: &str) -> bool {
        let mut w = self.nonce_log.write();
        // 先清理过期条目
        let ttl = self.config.nonce_ttl;
        let now = Instant::now();
        w.retain(|_, t| now.duration_since(*t) < ttl);
        if w.contains_key(nonce) {
            return false; // 重复 nonce
        }
        w.insert(nonce.to_string(), now);
        true
    }

    /// 主调度入口（对应 BallWidget.execute_text / voice_proxy intent 消息处理）
    pub async fn dispatch_intent(
        &self,
        text: &str,
        identity: &OperatorIdentity,
    ) -> XiaobaiResult<DispatchIntentResult> {
        let t0 = Instant::now();
        let nonce = Uuid::new_v4().to_string();
        if !self.idempotent_check_and_mark(&nonce) {
            return Err(XiaobaiError::InvalidArgument {
                action: "dispatch_intent".into(),
                param: "nonce".into(),
                value: nonce,
                hint: "该意图 30s 内已处理，为保证幂等拒绝重复执行".into(),
            });
        }

        // ---- S1 意图路由 ----
        let candidates = self.router.dispatch(text).await?;
        if candidates.is_empty() {
            return Err(XiaobaiError::IntentUnknown(text.into()));
        }
        let winner = &candidates[0];
        let need_alliance = winner.confidence_delta <= self.router.ambiguity_threshold();

        // ---- S1b 三策略 × 歧义处理 ----
        let (verdict, verdict_reasons) = match (self.config.mode, need_alliance, self.alliance.as_ref()) {
            (DispatchMode::LocalFirst, false, _) => ("local_first_direct".to_string(), Vec::new()),
            (DispatchMode::LocalFirst, true, Some(a)) => {
                // 异步发联盟裁决（不阻塞用户；结果写审计），本地直干高分项
                let env = Envelope::new_intent("xiaobai-engine-rust", text, identity, DispatchMode::LocalFirst);
                let a_clone = a.clone();
                let cands = candidates.clone();
                tokio::spawn(async move {
                    let _ = a_clone.ask_verdict(&env, &cands).await;
                });
                ("local_first_ambiguous_direct_high_score".to_string(), Vec::new())
            }
            (DispatchMode::LocalFirst, true, None) => ("local_first_no_alliance_client".to_string(), Vec::new()),
            (DispatchMode::CloudFallback, false, _) => ("cloud_fallback_clear_highscore".to_string(), Vec::new()),
            (mode @ _, _need, Some(a)) => {
                let env = Envelope::new_intent("xiaobai-engine-rust", text, identity, mode);
                let deadline = self.config.cloud_deadline;
                let timed = tokio::time::timeout(deadline, a.ask_verdict(&env, &candidates)).await;
                match timed {
                    Ok(Ok((v, rs))) => (v, rs),
                    Ok(Err(XiaobaiError::BridgeDisconnected { .. }))
                        if mode == DispatchMode::CloudFallback =>
                    {
                        ("cloud_fallback_bridge_down_local".to_string(), Vec::new())
                    }
                    Ok(Err(e)) => {
                        if mode == DispatchMode::CloudOnly {
                            return Err(e);
                        }
                        ("cloud_fallback_err_local_highscore".to_string(), Vec::new())
                    }
                    Err(_elapsed) => {
                        if mode == DispatchMode::CloudOnly {
                            return Err(XiaobaiError::BridgeDisconnected {
                                mode: mode.as_str(),
                                target: "mox-alliance-adjudicator".into(),
                                elapsed_ms: deadline.as_millis() as u64,
                            });
                        }
                        ("cloud_fallback_timeout_local_highscore".to_string(), Vec::new())
                    }
                }
            }
            (mode @ _, true, None) => {
                if mode == DispatchMode::CloudOnly {
                    return Err(XiaobaiError::BridgeDisconnected {
                        mode: mode.as_str(),
                        target: "mox-alliance-adjudicator".into(),
                        elapsed_ms: self.config.cloud_deadline.as_millis() as u64,
                    });
                }
                ("cloud_fallback_no_client_local_highscore".to_string(), Vec::new())
            }
            (DispatchMode::CloudOnly, false, None) => {
                return Err(XiaobaiError::BridgeDisconnected {
                    mode: "cloud_only",
                    target: "mox-alliance-adjudicator".into(),
                    elapsed_ms: self.config.cloud_deadline.as_millis() as u64,
                });
            }
        };
        if verdict == "alliance_rejected_veto" {
            return Err(XiaobaiError::AllianceRejected {
                verdict: "alliance_rejected_veto",
                reasons: verdict_reasons,
            });
        }

        // ---- S2 查注册表 + RBAC 鉴权（PII 强制升 L3）----
        // 关键：把 sig 和 Arc<op> clone 出来，解除对 RwLockReadGuard<'_, _> 的借用，
        // 否则 spawn_blocking 闭包要求 'static 会编译失败（借用守卫活不到闭包执行）
        let (op_clone, sig_clone): (Arc<dyn SystemOperator>, ActionSignature) = {
            let reg_snapshot = self.registry.read();
            let (op, sig) = reg_snapshot
                .get(&winner.action)
                .ok_or_else(|| XiaobaiError::IntentUnknown(format!("action={} not registered", winner.action)))?;
            (op.clone(), sig.clone())
        };

        let mut required_level = sig_clone.clearance;
        // 判定 PII 敏感资源（File 类的 path 参数或 OpenFileWithApp 的 path）
        if sig_clone.category == OperatorCategory::File {
            let resource_path = winner
                .param
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if crate::constants::SENSITIVE_DOMAINS
                .iter()
                .any(|d| resource_path.to_lowercase().contains(d))
                && !crate::constants::DESENSITIZED_SUFFIXES
                    .iter()
                    .any(|s| resource_path.ends_with(s))
            {
                required_level = ClearanceLevel::from_u8(PII_SENSITIVE_FORCE_LEVEL)?;
            }
        }
        check_clearance(&winner.action, required_level, identity, sig_clone.own_qualified)?;

        // ---- S3 执行（spawn_blocking 封装底层阻塞系统调用）----
        let action = winner.action.clone();
        let action_err_clone = winner.action.clone();
        let param = crate::operator::ActionParam::new(winner.param.clone());
        let id = identity.clone();
        let category_for_err = sig_clone.category;
        // op_clone 已在 S2 从 reg_snapshot 里 clone 出来（Arc，引用计数 +1），直接 move 进闭包
        let execute_result = tokio::task::spawn_blocking(move || {
            // execute 本身是 async，spawn_blocking 里不能直接 await；
            // 解决方式：建一个 mini tokio current_thread mox_platform_orchestrator_svc 在阻塞线程内跑
            let rt = tokio::runtime::Handle::try_current().ok();
            match rt {
                Some(handle) => handle.block_on(op_clone.execute(&action, param, &id)),
                None => {
                    let mini = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| XiaobaiError::ExecutionError {
                            category: category_for_err.as_str().into(),
                            action: action.clone(),
                            detail: format!("failed to build mini tokio mox_platform_orchestrator_svc: {e}"),
                        })?;
                    mini.block_on(op_clone.execute(&action, param, &id))
                }
            }
        })
        .await
        .map_err(|e| XiaobaiError::ExecutionError {
            category: sig_clone.category.as_str().into(),
            action: action_err_clone,
            detail: format!("tokio join error: {e}"),
        })??;

        // ---- S5 审计回调（XB-008 仅记录，不阻塞返回）----
        let audit = AuditPayload {
            trace_id: nonce.clone(),
            action: winner.action.clone(),
            identity_user_id: identity.user_id.clone(),
            result: "passed".to_string(),
            level: required_level.as_u8(),
            detail: format!(
                "category={} elapsed={}ms verdict={}",
                sig_clone.category.as_str(),
                execute_result.elapsed_ms,
                verdict
            ),
            executed_at_ms: chrono::Utc::now().timestamp_millis(),
            source: "local_operator".to_string(),
        };
        if let Err(e) = (self.config.audit_fn)(audit.clone()) {
            tracing::warn!(
                "XB-008 AuditCallbackFailed sink=engine_config_audit_fn reason={}",
                e
            );
        }

        Ok(DispatchIntentResult {
            trace_id: nonce,
            action: winner.action.clone(),
            category: sig_clone.category,
            executed_where: "local".to_string(),
            required_level: ClearanceLevelRepr(required_level.as_u8()),
            output: execute_result,
            audit: Some(audit),
            verdict,
            total_elapsed_ms: t0.elapsed().as_millis() as u64,
        })
    }
}

// 把 Protocol 的 ExecPayload / DispatchIntentResult 合并成一个对外结构体（避免外部引 protocol 重名）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchIntentResult {
    pub trace_id: String,
    pub action: String,
    pub category: crate::operator::OperatorCategory,
    pub executed_where: String, // "local" / "remote_alliance"
    pub required_level: ClearanceLevelRepr,
    pub output: OperatorOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit: Option<AuditPayload>,
    pub verdict: String,
    pub total_elapsed_ms: u64,
}
