// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # T4 事件驱动：进程内事件总线（Experts Events）
//!
//! 工程此前**零事件机制**（grep broadcast/subscribe/EventBus/notify_ 零命中），审计为直接
//! 调用、SSE 为单向日志流。本模块从零建立一条**进程内多消费者广播总线**，落地 T4 卡片
//! （§2.4：任务状态变更事件流 pending→running→completed/failed、节点启停）的第一性闭环：
//!
//! - **事件模型**：[`AllianceEventKind] 枚举（按真实 emit 点收敛，不为枚举而枚举）+
//!   [`AllianceEvent`] 信封（类型 + 结构化载荷 + 来源 handler + RFC3339 时间戳 + 租户）。
//! - **总线选型**：[`tokio::sync::broadcast`]。理由：
//!   1. tokio features=`full`，已含 `sync::broadcast`，**零新依赖**；
//!   2. 一对多广播，天然支持「事件日志消费者 + 未来 SSE 事件帧消费者 + 外部系统订阅」多下游；
//!   3. 生产路径 `send` 非阻塞、无消费者时返回 `Err` 被忽略，**失败不阻断业务**（与既有
//!      审计 best-effort 容错一致）；
//!   4. 背压由「有界 channel + Lagged 追赶」承担，慢消费者不拖垮写路径。
//! - **消费者**：[`spawn_event_log_consumer`] 订阅总线，把事件真实落 SQLite
//!   `alliance_event_log`（见 experts_db），形成可观察副作用与可查询的事件轨迹骨干。
//!
//! 与既有审计/租户关系：审计照旧经 `emit_audit` 直接调用（本模块不替换、不改动）；事件消费者
//! 为**增量**能力。事件信封携带 `tenant`，与 A1 多租户行级隔离一致，消费者按租户落库。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;

use super::experts_common::{ExpertsSharedState, gen_id, now_iso};

/// 联盟域事件类型（T4）。
///
/// 变体严格按**真实 emit 点**收敛（见 experts_orchestration / experts_registry 的 handler）：
/// - [`PlanCreated`][AllianceEventKind::PlanCreated]：计划生成（draft）。emit：generate_plan_handler / orchestrate。
/// - [`PlanStatusChanged`][AllianceEventKind::PlanStatusChanged]：计划状态推进
///   （draft→running→completed/failed/partial）。emit：execute_plan_handler / orchestrate。
/// - [`ExpertRegistered`][AllianceEventKind::ExpertRegistered]：专家注册。emit：create_expert。
/// - [`ExpertDisabled`][AllianceEventKind::ExpertDisabled]：专家软删除/禁用。emit：delete_expert。
///
/// `#[serde(tag = "type")]` 内部标记序列化为 `{"type":"PlanCreated", ...载荷}`，
/// 便于 event_log.event_type 列与未来 SSE 事件帧直接取用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AllianceEventKind {
    /// 计划创建（初始状态 draft）
    PlanCreated {
        plan_id: String,
        task_type: String,
        title: String,
    },
    /// 计划状态推进（from → to）；execution_id 仅执行类事件携带
    PlanStatusChanged {
        plan_id: String,
        from: String,
        to: String,
        execution_id: Option<String>,
    },
    /// 专家注册
    ExpertRegistered { expert_id: String, name: String },
    /// 专家禁用 / 软删除
    ExpertDisabled { expert_id: String },
}

impl AllianceEventKind {
    /// 稳定事件类型名（落 event_log.event_type 列；与 serde tag 对齐）
    pub fn type_name(&self) -> &'static str {
        match self {
            AllianceEventKind::PlanCreated { .. } => "PlanCreated",
            AllianceEventKind::PlanStatusChanged { .. } => "PlanStatusChanged",
            AllianceEventKind::ExpertRegistered { .. } => "ExpertRegistered",
            AllianceEventKind::ExpertDisabled { .. } => "ExpertDisabled",
        }
    }

    /// 事件关联的计划 ID（无则 None；落 event_log.plan_id 列）
    pub fn plan_id(&self) -> Option<&str> {
        match self {
            AllianceEventKind::PlanCreated { plan_id, .. }
            | AllianceEventKind::PlanStatusChanged { plan_id, .. } => Some(plan_id),
            AllianceEventKind::ExpertRegistered { .. } | AllianceEventKind::ExpertDisabled { .. } => None,
        }
    }
}

/// 事件信封：类型 + 载荷 + 来源 + 时间戳 + 租户。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllianceEvent {
    /// 事件唯一 ID（evt-<uuid>）
    pub id: String,
    /// 事件类型与结构化载荷
    #[serde(flatten)]
    pub kind: AllianceEventKind,
    /// 来源 handler / 模块（可观测归因）
    pub source: String,
    /// 所属租户（A1：与请求租户一致；消费者按租户落库）
    pub tenant: String,
    /// 发生时间（RFC3339，UTC，秒精度）
    pub occurred_at: String,
}

impl AllianceEvent {
    /// 构造新事件（自动生成 id 与当前时间戳）
    pub fn new(kind: AllianceEventKind, tenant: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: gen_id("evt"),
            kind,
            source: source.into(),
            tenant: tenant.into(),
            occurred_at: now_iso(),
        }
    }
}

/// Webhook 外部订阅（T4 对外出口）。
///
/// 订阅者登记一个目标 URL 与事件类型过滤；总线派发器在真实事件发生时，把事件信封
/// 原样 `POST` 到该 URL。
///
/// **持久化（v5，2026-10-03 全维终验）**：注册表为「进程内内存 HashMap + SQLite 写穿」。
/// CRUD 经 [`EventBus::register_webhook`] / [`EventBus::delete_webhook`] 写穿
/// `alliance_webhooks` 表（experts_db），启动时 [`EventBus::restore_webhooks_from_db`]
/// 一次读回重建内存态——**重启后订阅与派发自动恢复**，不再重启即失。内存 HashMap 仍是
/// 派发器逐事件查匹配的热读投影（持锁仅快照克隆），权威源是 SQLite。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    /// 订阅 ID（wh-<uuid>）
    pub id: String,
    /// 所属租户（仅投递本租户事件）
    pub tenant: String,
    /// 目标 URL（须 http/https）
    pub url: String,
    /// 事件类型过滤（空数组 = 全部；否则仅投递列出的类型名）
    pub event_types: Vec<String>,
    /// 创建时间（RFC3339）
    pub created_at: String,
}

/// Webhook 注册表（id -> 订阅；挂在总线上，随总线克隆传播）。
pub type WebhookTable = Arc<Mutex<HashMap<String, WebhookSubscription>>>;

/// 进程内事件总线（tokio broadcast 广播端的薄封装）。
///
/// 挂在 [`ExpertsSharedState.events`] 上，随 state 克隆传播（内部为 `Arc` 语义的 Sender）。
/// 多消费者各自持有独立 `Receiver`，互不影响；慢消费者落后会被 `Lagged` 丢弃追赶，
/// 不反压写路径。除广播外，本结构还持有 **Webhook 内存注册表**（见 [`WebhookSubscription`]）。
#[derive(Clone)]
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<AllianceEvent>,
    webhooks: WebhookTable,
}

impl EventBus {
    /// 新建总线（capacity 为有界 channel 容量；事件为低频业务事件，64/128 足够）
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(capacity.max(8));
        Self {
            tx,
            webhooks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 发布事件（生产路径，非阻塞）。
    ///
    /// 无任何活跃消费者时 `send` 返回 `Err(SendError)`，此处**忽略**——业务写路径不因
    /// 「没人订阅」而失败（与审计 best-effort 一致）。
    pub fn emit(&self, event: AllianceEvent) {
        let _ = self.tx.send(event);
    }

    /// 订阅总线（拿到独立 Receiver）
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AllianceEvent> {
        self.tx.subscribe()
    }

    /// 当前活跃消费者数（观测/测试断言用）
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    // ---- Webhook 注册表面（内存热投影 + SQLite 写穿，v5 重启恢复） ----

    /// 登记一个 webhook 订阅（按租户隔离）。
    ///
    /// 先入内存热投影，再写穿 `alliance_webhooks`（best-effort，失败仅 log 不阻断——
    /// 内存投影仍即时生效，落盘失败下次启动可能丢失该订阅，与本模块 best-effort 约定一致）。
    pub fn register_webhook(&self, tenant: impl Into<String>, url: String, event_types: Vec<String>) -> WebhookSubscription {
        let wh = WebhookSubscription {
            id: gen_id("wh"),
            tenant: tenant.into(),
            url,
            event_types,
            created_at: now_iso(),
        };
        self.webhooks.lock().insert(wh.id.clone(), wh.clone());
        // v5：写穿 SQLite（best-effort）。event_types 序列化为 JSON 数组字符串存列。
        let et_json = serde_json::to_string(&wh.event_types).unwrap_or_else(|_| "[]".to_string());
        crate::alliance::experts_db::upsert_webhook(
            &wh.id, &wh.tenant, &wh.url, &et_json, &wh.created_at,
        );
        wh
    }

    /// 列出某租户的全部订阅
    pub fn list_webhooks(&self, tenant: &str) -> Vec<WebhookSubscription> {
        self.webhooks
            .lock()
            .values()
            .filter(|w| w.tenant == tenant)
            .cloned()
            .collect()
    }

    /// 删除订阅（仅租户本人可删；返回是否真的删掉）。
    ///
    /// 内存投影命中后立即删；再写穿 `alliance_webhooks`（best-effort），重启后不再恢复该订阅。
    pub fn delete_webhook(&self, tenant: &str, id: &str) -> bool {
        let mut g = self.webhooks.lock();
        let hit = matches!(g.get(id), Some(w) if w.tenant == tenant);
        if hit {
            g.remove(id);
            crate::alliance::experts_db::delete_webhook_row(id, tenant);
        }
        hit
    }

    /// 启动期从 SQLite 读回全部 webhook 订阅，重建内存热投影（v5 重启恢复）。
    ///
    /// 由 `ExpertsSharedState::new()` 在总线创建后调用一次。读失败/空库则内存投影保持空
    /// （与历史行为一致，不阻断启动）。event_types 列解析失败时降级为空数组（=全收），
    /// 不丢弃整条订阅。
    pub fn restore_webhooks_from_db(&self) {
        let rows = crate::alliance::experts_db::load_all_webhooks();
        let mut g = self.webhooks.lock();
        for r in rows {
            let event_types: Vec<String> = serde_json::from_str(&r.event_types).unwrap_or_default();
            g.insert(
                r.id.clone(),
                WebhookSubscription {
                    id: r.id,
                    tenant: r.tenant_id,
                    url: r.url,
                    event_types,
                    created_at: r.created_at,
                },
            );
        }
    }

    /// 选出应收到本次事件的订阅：同租户 + 类型过滤命中（空过滤=全收）。
    /// 派发器逐事件调用；持锁仅做快照克隆，不持有锁做网络 IO。
    pub fn matching_webhooks(&self, tenant: &str, type_name: &str) -> Vec<WebhookSubscription> {
        self.webhooks
            .lock()
            .values()
            .filter(|w| {
                w.tenant == tenant
                    && (w.event_types.is_empty() || w.event_types.iter().any(|t| t == type_name))
            })
            .cloned()
            .collect()
    }
}

/// 发布事件到 state 总线的便捷入口（handler 内一行调用）。
pub fn emit(state: &ExpertsSharedState, kind: AllianceEventKind, tenant: &str, source: &str) {
    state.events.emit(AllianceEvent::new(kind, tenant, source));
}

/// 启动「事件日志消费者」后台任务（T4 真实副作用）。
///
/// 订阅总线 → 把每个事件真实落 SQLite `alliance_event_log`（带 tenant/event_type/occurred_at/
/// plan_id/结构化 payload）。这是 T4 第一性闭环的消费者侧：事件→可观察落库轨迹。
///
/// **运行时检测**：仅在存在 tokio 运行时（服务进程 `#[tokio::main]` 或 `#[tokio::test]`）时
/// `tokio::spawn`；无运行时（如部分同步 `#[test]` 直接构造 state）时 noop——
/// 这类用例本就不 emit 业务事件，spawn 反而会因无 reactor panic。
pub fn spawn_event_log_consumer(bus: Arc<EventBus>) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    tokio::spawn(async move {
        let mut rx = bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let payload = match serde_json::to_value(&ev) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("[event_bus] 序列化事件 {} 失败: {e}", ev.id);
                            continue;
                        }
                    };
                    crate::alliance::experts_db::insert_event_log(
                        &ev.tenant,
                        &ev.id,
                        ev.kind.type_name(),
                        &ev.source,
                        &ev.occurred_at,
                        ev.kind.plan_id().unwrap_or(""),
                        &payload,
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("[event_bus] 消费者落后丢弃 {n} 条事件，继续追赶");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// 启动「Webhook 外部推送」后台任务（T4 对外出口，真实 HTTP 送达）。
///
/// 订阅总线 → 对每条事件查 [`EventBus::matching_webhooks`] 选出同租户、类型命中的订阅，
/// 把事件信封原样 `POST` 到订阅 URL（reqwest，5s 超时）。失败/非 2xx 记日志并**重试 1 次**
/// （间隔 300ms）；仍失败即丢弃，不反压业务总线。无运行时（同步测试）时 noop。
pub fn spawn_webhook_dispatcher(bus: Arc<EventBus>) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    tokio::spawn(async move {
        // 进程内一个共享 client（连接池）；rustls-tls 特性，回环 http 与外网 https 均可用
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[webhook] reqwest client 构建失败，webhook 推送停用: {e}");
                return;
            }
        };
        let mut rx = bus.subscribe();
        loop {
            let ev = match rx.recv().await {
                Ok(ev) => ev,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            let targets = bus.matching_webhooks(&ev.tenant, ev.kind.type_name());
            if targets.is_empty() {
                continue;
            }
            let body = match serde_json::to_vec(&ev) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("[webhook] 序列化事件 {} 失败: {e}", ev.id);
                    continue;
                }
            };
            for wh in targets {
                dispatch_once(&client, &wh.url, &body).await;
            }
        }
    });
}

/// 向单个 webhook URL 投递一次事件信封（最多 2 次尝试：失败后重试 1 次）。
async fn dispatch_once(client: &reqwest::Client, url: &str, body: &[u8]) {
    for attempt in 0..2 {
        match client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Alliance-Event", "alliance")
            .body(body.to_vec())
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => return,
            Ok(resp) => {
                eprintln!("[webhook] {} -> HTTP {}（尝试 {}/2）", url, resp.status().as_u16(), attempt + 1);
            }
            Err(e) => {
                eprintln!("[webhook] {} 投递失败（尝试 {}/2）: {e}", url, attempt + 1);
            }
        }
        if attempt == 0 {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
}

// =====================================================================
// 单元测试：事件模型序列化 / 总线收发 / 无消费者不阻断
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_serializes_with_type_tag() {
        let ev = AllianceEvent::new(
            AllianceEventKind::PlanStatusChanged {
                plan_id: "plan-1".into(),
                from: "draft".into(),
                to: "running".into(),
                execution_id: Some("exec-1".into()),
            },
            "tenant-a",
            "execute_plan_handler",
        );
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["type"], "PlanStatusChanged");
        assert_eq!(v["plan_id"], "plan-1");
        assert_eq!(v["to"], "running");
        assert_eq!(v["tenant"], "tenant-a");
        assert_eq!(v["source"], "execute_plan_handler");
        assert!(v["occurred_at"].is_string());
        assert!(v["id"].as_str().unwrap().starts_with("evt-"));

        // 往返
        let back: AllianceEvent = serde_json::from_value(v).unwrap();
        assert_eq!(back.kind.type_name(), "PlanStatusChanged");
        assert_eq!(back.kind.plan_id(), Some("plan-1"));
    }

    #[test]
    fn type_name_and_plan_id_cover_all_variants() {
        let cases = [
            AllianceEventKind::PlanCreated { plan_id: "p".into(), task_type: "t".into(), title: "x".into() },
            AllianceEventKind::PlanStatusChanged { plan_id: "p".into(), from: "a".into(), to: "b".into(), execution_id: None },
            AllianceEventKind::ExpertRegistered { expert_id: "e".into(), name: "n".into() },
            AllianceEventKind::ExpertDisabled { expert_id: "e".into() },
        ];
        assert_eq!(cases[0].type_name(), "PlanCreated");
        assert_eq!(cases[1].type_name(), "PlanStatusChanged");
        assert_eq!(cases[2].type_name(), "ExpertRegistered");
        assert_eq!(cases[3].type_name(), "ExpertDisabled");
        assert_eq!(cases[0].plan_id(), Some("p"));
        assert_eq!(cases[2].plan_id(), None);
    }

    #[test]
    fn emit_without_subscriber_does_not_panic() {
        let bus = EventBus::new(16);
        // 无订阅者：send 返回 Err，被忽略，不 panic、不阻断
        bus.emit(AllianceEvent::new(
            AllianceEventKind::ExpertRegistered { expert_id: "e1".into(), name: "n".into() },
            "default",
            "create_expert",
        ));
        assert_eq!(bus.receiver_count(), 0);
    }

    #[tokio::test]
    async fn subscriber_receives_broadcast_event() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        bus.emit(AllianceEvent::new(
            AllianceEventKind::PlanCreated {
                plan_id: "plan-x".into(),
                task_type: "research".into(),
                title: "t".into(),
            },
            "tenant-b",
            "generate_plan_handler",
        ));
        let ev = rx.recv().await.expect("订阅者应收到广播事件");
        assert_eq!(ev.kind.type_name(), "PlanCreated");
        assert_eq!(ev.tenant, "tenant-b");
        assert_eq!(ev.kind.plan_id(), Some("plan-x"));
    }

    #[tokio::test]
    async fn multiple_subscribers_each_receive() {
        let bus = EventBus::new(16);
        let mut r1 = bus.subscribe();
        let mut r2 = bus.subscribe();
        bus.emit(AllianceEvent::new(
            AllianceEventKind::ExpertDisabled { expert_id: "e9".into() },
            "default",
            "delete_expert",
        ));
        let e1 = r1.recv().await.unwrap();
        let e2 = r2.recv().await.unwrap();
        assert_eq!(e1.kind.type_name(), "ExpertDisabled");
        assert_eq!(e2.kind.type_name(), "ExpertDisabled");
        assert_eq!(e1.id, e2.id); // 同一条广播
    }

    #[test]
    fn spawn_consumer_no_runtime_is_noop() {
        // 同步上下文无运行时：不应 panic
        let bus = Arc::new(EventBus::new(16));
        spawn_event_log_consumer(bus);
    }

    #[test]
    fn payload_json_shape_is_stable() {
        let ev = AllianceEvent::new(
            AllianceEventKind::ExpertRegistered { expert_id: "exp-1".into(), name: "架构师".into() },
            "tenant-z",
            "create_expert",
        );
        let v = serde_json::to_value(&ev).unwrap();
        assert_eq!(v["type"], "ExpertRegistered");
        assert_eq!(v["expert_id"], "exp-1");
        assert!(json_eq(&v["tenant"], "tenant-z"));
    }

    fn json_eq(v: &serde_json::Value, s: &str) -> bool {
        v.as_str() == Some(s)
    }
}
