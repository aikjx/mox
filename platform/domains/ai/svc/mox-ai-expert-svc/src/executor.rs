//! 执行态可视化监控闭环（Phase 3）
//!
//! 把「可预测」优化报告变成「可观察」执行轨迹：
//! 输入 `ProgrammingReport`（含优化后图 + 调度 slots），按 `Slot.start_ms/finish_ms`
//! 以可调速率回放确定性执行，逐节点产出 `ExecEvent` 并通过 `watch` 广播实时轨迹。
//! 前端轮询 `/api/trace` 即可渲染进度条、节点状态、关键路径跑动、完成率。
//!
//! 确定性保证：同一份报告 + 同一 rate → 完全一致的事件序列与完成时刻
//! （不依赖真实时钟抖动，只用「相对虚拟时间轴」× rate 换算 sleep）。

use crate::programming::ProgrammingReport;
use mox_ai_flow_svc::schedule::Schedule;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex};

/// 单节点执行状态
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecStatus {
    /// 已入队，未开始
    Queued,
    /// 执行中（处于 Slot 时间窗内）
    Running,
    /// 完成
    Done,
    /// 失败（仅当图被否决时整体标记）
    Failed,
}

/// 一条执行轨迹事件（前端监听的最小单元）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecEvent {
    /// 节点 id
    pub id: String,
    /// 节点名
    pub label: String,
    /// 资源池（用于前端分池着色）
    pub pool: String,
    /// 状态
    pub status: ExecStatus,
    /// 相对虚拟时间轴（ms，已按 rate 缩放）
    pub t_ms: u64,
    /// 关键路径标记
    pub on_critical: bool,
}

/// 执行轨迹快照（前端每轮轮询拿到的完整态）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecTrace {
    /// 流程 id
    pub flow_id: String,
    /// 是否正在执行
    pub running: bool,
    /// 是否结束
    pub finished: bool,
    /// 虚拟时间轴当前 ms
    pub clock_ms: u64,
    /// 预测总耗时（schedule.makespan，ms；已含 loop 上限护栏）
    pub makespan_ms: u64,
    /// 完成率 0.0 ~ 1.0（Done 节点占比）
    pub progress: f64,
    /// 每个节点的当前状态
    pub nodes: Vec<ExecEvent>,
    /// 阻塞原因（被否决图不可执行时填）
    pub blocked_reason: Option<String>,
}

/// 共享执行状态（watch 广播源）
#[derive(Debug)]
pub struct ExecState {
    pub trace: Arc<Mutex<ExecTrace>>,
    pub tx: watch::Sender<ExecTrace>,
}

impl ExecState {
    pub fn new(flow_id: &str, makespan_ms: u64) -> Arc<Self> {
        let trace = ExecTrace {
            flow_id: flow_id.to_string(),
            running: false,
            finished: false,
            clock_ms: 0,
            makespan_ms,
            progress: 0.0,
            nodes: Vec::new(),
            blocked_reason: None,
        };
        let (tx, _rx) = watch::channel(trace.clone());
        Arc::new(Self {
            trace: Arc::new(Mutex::new(trace)),
            tx,
        })
    }

    /// 订阅实时轨迹（前端 / 测试用）
    pub fn subscribe(&self) -> watch::Receiver<ExecTrace> {
        self.tx.subscribe()
    }

    async fn emit(&self, snap: ExecTrace) {
        *self.trace.lock().await = snap.clone();
        let _ = self.tx.send(snap);
    }
}

/// 从优化报告派生执行计划（节点 → 时间窗 + 资源池 + 关键路径）
struct ExecPlan {
    flow_id: String,
    makespan_ms: u64,
    /// 每个可执行节点的 (id, label, pool, start, finish, on_critical)
    slots: Vec<(String, String, String, u64, u64, bool)>,
}

fn build_plan(rep: &ProgrammingReport) -> Option<ExecPlan> {
    let gov = rep.governance.as_ref()?;
    let opt = &gov.optimization;
    let g = &opt.optimized_graph;
    let sched: &Schedule = &opt.schedule;
    // 关键路径节点集合（用于前端高亮跑动）
    let critical: std::collections::HashSet<String> = opt
        .critical_path
        .critical_paths
        .iter()
        .flat_map(|p| p.iter().cloned())
        .collect();
    // 资源池映射：复用 schedule 的 pool_of 规则（control / pool:xxx）
    let pool_of = |id: &str| -> String {
        g.node(id)
            .and_then(|n| n.tool)
            .map(|t| match t {
                mox_ai_flow_svc::model::ToolKind::Browser => "pool:browser".into(),
                mox_ai_flow_svc::model::ToolKind::Database => "pool:db".into(),
                mox_ai_flow_svc::model::ToolKind::Llm => "pool:llm".into(),
                _ => "pool:compute".into(),
            })
            .unwrap_or_else(|| "control".into())
    };
    let mut slots = Vec::new();
    for s in &sched.slots {
        let label = g
            .node(&s.id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| s.id.clone());
        slots.push((
            s.id.clone(),
            label,
            pool_of(&s.id),
            s.start_ms,
            s.finish_ms,
            critical.contains(&s.id),
        ));
    }
    Some(ExecPlan {
        flow_id: g.id.clone(),
        makespan_ms: sched.makespan_ms,
        slots,
    })
}

/// 启动确定性执行（异步，按 rate 缩放虚拟时间轴）。
/// rate=1.0 → 真实 ms；rate=0.01 → 1ms 虚拟时间 = 10µs 真实（演示加速）。
/// 返回 ExecState 供前端订阅。
pub async fn run_report(rep: &ProgrammingReport, rate: f64) -> Arc<ExecState> {
    let rate = if rate <= 0.0 { 1.0 } else { rate };
    let plan = build_plan(rep);

    let (flow_id, makespan) = match &plan {
        Some(p) => (p.flow_id.clone(), p.makespan_ms),
        None => (rep.flow_id.clone(), 0),
    };
    let state = ExecState::new(&flow_id, makespan);

    // 被否决/未出码 → 标记 blocked，立即结束（不执行任何节点）
    if !rep.safe_to_emit {
        let reason = rep
            .governance
            .as_ref()
            .map(|g| {
                if g.algo.vetoed {
                    "算法验证网关否决：不可执行".to_string()
                } else {
                    format!("治理闸门未通过：{}", g.gate.reason)
                }
            })
            .unwrap_or_else(|| "需求未确认或流程不合规".to_string());
        let snap = {
            let mut t = state.trace.lock().await.clone();
            t.finished = true;
            t.blocked_reason = Some(reason);
            t
        };
        state.emit(snap).await;
        return state;
    }

    let plan = plan.expect("safe_to_emit 保证有 plan");
    let st = state.clone();

    // 初始化所有节点为 Queued
    {
        let snap = {
            let mut t = st.trace.lock().await.clone();
            t.running = true;
            t.nodes = plan
                .slots
                .iter()
                .map(|(id, label, pool, _, _, on_c)| ExecEvent {
                    id: id.clone(),
                    label: label.clone(),
                    pool: pool.clone(),
                    status: ExecStatus::Queued,
                    t_ms: 0,
                    on_critical: *on_c,
                })
                .collect();
            t
        };
        st.emit(snap).await;
    }

    // 按虚拟时间轴推进：把 slots 展开成「开始/结束」事件，按 start 排序
    tokio::spawn(async move {
        let mut timeline: Vec<(u64, String, bool)> = Vec::new(); // (t_ms, node_id, is_start)
        for (id, _, _, start, finish, _) in &plan.slots {
            timeline.push((*start, id.clone(), true));
            timeline.push((*finish, id.clone(), false));
        }
        timeline.sort_by_key(|(t, _, _)| *t);

        let mut last_t = 0u64;
        for (t, id, is_start) in timeline {
            // 推进虚拟时钟到 t
            if t > last_t {
                let real = ((t - last_t) as f64 * rate) as u64;
                if real > 0 {
                    tokio::time::sleep(Duration::from_micros(real.max(1))).await;
                }
                last_t = t;
            }
            // 更新对应节点状态
            let snap = {
                let mut t2 = st.trace.lock().await.clone();
                t2.clock_ms = t;
                if let Some(ev) = t2.nodes.iter_mut().find(|n| n.id == id) {
                    ev.status = if is_start {
                        ExecStatus::Running
                    } else {
                        ExecStatus::Done
                    };
                    ev.t_ms = t;
                }
                let done = t2
                    .nodes
                    .iter()
                    .filter(|n| n.status == ExecStatus::Done)
                    .count();
                t2.progress = if t2.nodes.is_empty() {
                    1.0
                } else {
                    done as f64 / t2.nodes.len() as f64
                };
                t2
            };
            st.emit(snap).await;
        }
        // 收尾：全部 Done
        let snap = {
            let mut t3 = st.trace.lock().await.clone();
            t3.clock_ms = plan.makespan_ms;
            t3.running = false;
            t3.finished = true;
            t3.progress = 1.0;
            for n in t3.nodes.iter_mut() {
                n.status = ExecStatus::Done;
            }
            t3
        };
        st.emit(snap).await;
    });

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{GovernContext, LoopGuard, LoopPolicy, Principal, Tenant};
    use crate::programming::programming_pipeline;
    use mox_ai_flow_svc::model::{FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};

    fn demo_graph() -> FlowGraph {
        let mut g = FlowGraph::new("demo", "演示执行");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("a", "拉取", ToolKind::Database, 200)
                .with_access(mox_ai_flow_svc::model::Access::read("db:x")),
        );
        g.add_node(FlowNode::task("b", "处理", ToolKind::Compute, 100));
        g.add_node(FlowNode::task("c", "汇总", ToolKind::Compute, 50));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "a"));
        g.add_edge(FlowEdge::seq("a", "b"));
        g.add_edge(FlowEdge::seq("b", "c"));
        g.add_edge(FlowEdge::seq("c", "e"));
        g
    }

    fn ctx() -> GovernContext {
        let t = Tenant::new("acme", "fin").with_pool("browser", 4);
        let p = Principal::new("ops").with_roles(vec!["editor".into(), "approver".into()]);
        let mut c = GovernContext::new(t, p);
        c.quota.max_parallel = 8;
        c.quota.max_cost_budget = 100.0;
        c.quota.sla_ms = 50_000;
        c
    }

    #[tokio::test]
    async fn deterministic_execution_completes() {
        let g = demo_graph();
        let rep = programming_pipeline("演示执行流程", vec!["顺序执行".into()], true, &g, &ctx());
        assert!(rep.safe_to_emit, "演示图应可出码");
        let state = run_report(&rep, 0.001).await; // 极速回放
                                                   // 等待执行完成
        let mut rx = state.subscribe();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            match tokio::time::timeout_at(deadline, rx.changed()).await {
                Ok(Ok(())) => {
                    let snap = rx.borrow().clone();
                    if snap.finished {
                        break;
                    }
                }
                _ => break,
            }
        }
        let snap = state.trace.lock().await.clone();
        assert!(snap.finished, "应执行完毕");
        assert_eq!(snap.progress, 1.0, "完成率应为 1.0");
        assert!(
            snap.nodes.iter().all(|n| n.status == ExecStatus::Done),
            "所有节点应 Done"
        );
        assert!(snap.makespan_ms > 0, "应有预测耗时");
    }

    #[tokio::test]
    async fn blocked_graph_not_executed() {
        // 危险无界循环 → 被 check_loops 否决 → 不可执行
        let mut g = FlowGraph::new("bad", "危险");
        g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
        g.add_node(FlowNode::new("ls", "循环", NodeKind::LoopStart));
        g.add_node(
            FlowNode::task("scr", "改写", ToolKind::Database, 100)
                .with_access(mox_ai_flow_svc::model::Access::write("db:prod")),
        );
        g.add_node(FlowNode::new("le", "出口", NodeKind::LoopEnd));
        g.add_node(FlowNode::new("e", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("s", "ls"));
        g.add_edge(FlowEdge::seq("ls", "scr"));
        g.add_edge(FlowEdge::seq("scr", "le"));
        g.add_edge(FlowEdge::seq("le", "ls"));
        g.add_edge(FlowEdge::seq("le", "e"));
        let mut c = ctx();
        c.registry.register_loop(LoopGuard {
            node: "ls".into(),
            policy: LoopPolicy::Unbounded,
        });
        let rep = programming_pipeline("危险自循环", vec!["循环".into()], true, &g, &c);
        assert!(!rep.safe_to_emit);
        let state = run_report(&rep, 1.0).await;
        let snap = state.trace.lock().await.clone();
        assert!(snap.finished, "否决图应立即结束");
        assert!(snap.blocked_reason.is_some(), "应给出阻塞原因");
        assert_eq!(snap.progress, 0.0, "否决图不执行任何节点");
    }
}
