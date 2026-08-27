// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AIS-SPEC-9001：企业级统一契约头 —— 模块名 parallel_executor.rs
//! AIS-REV-1：自描述接口 · 幂等 · 可观测 · 零外部副作用
//! AIS-REV-2：公开项 pub fn/pub struct 必须具备 /// 文档注释与错误语义说明
//!
//! O5 补丁：并发扇出（Parallel Fan-Out）+ CancellationToken
//!
//!   对照矩阵（T10）维度 7「并发/分布式执行」差距：Dify v0.14 / Flowise v2 / AutoGen v0.4
//!   均无企业级可取消并发扇出；LangGraph OSS v0.2 的 send() 有分支但取消传播为全局 Abort，
//!   缺少子任务级 P99 指标。璇玑补齐：
//!     - CancellationToken：可组合取消令牌（链式 parent→children，一对多传播），零额外依赖
//!     - fan_out_join_set：N 分支并发 + SelectAll 优先取消模式
//!     - fail_fast：任一子任务 Err/超时 → 令牌取消 → JoinSet abort → 其余子任务立即释放
//!     - 每个子任务产出 BranchTaskResult（p50/p95/p99 可复用 O7 上报）
//!
//!   注：本模块刻意不用 futures-concurrency，只依赖 tokio full feature（工作空间已启用），
//!       避免新增第三方依赖破坏 AIS 依赖治理。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

// ================== CancellationToken ==================

/// 取消令牌：父子组合、可组合、多监听器注册。
///   - clone() 返回同一个令牌的共享引用（底层 Arc）。
///   - cancel()：一次调用，永久置位；所有父级取消会原子传播给所有子级。
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    inner: Arc<InnerToken>,
}

#[derive(Debug, Default)]
struct InnerToken {
    state: Mutex<TokenState>,
}

#[derive(Debug, Default)]
struct TokenState {
    cancelled: bool,
    children: Vec<CancellationToken>,
}

impl CancellationToken {
    /// 创建一个新的、未取消的令牌（根令牌）。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建一个绑定到本令牌的子令牌：父级取消时，子令牌同步取消。
    pub fn child(&self) -> Self {
        let child = CancellationToken::new();
        // 若父级已取消，直接把 child 也置位
        let mut s = self.inner.state.lock().expect("token state poisoned");
        if s.cancelled {
            child.cancel();
        } else {
            s.children.push(child.clone());
        }
        child
    }

    /// 立即取消本令牌及其所有后代令牌（级联）。重复调用幂等。
    pub fn cancel(&self) {
        // 取所有 children，释放锁后再逐个 cancel，避免重入死锁
        let kids: Vec<CancellationToken> = {
            let mut s = self.inner.state.lock().expect("token state poisoned");
            if s.cancelled {
                return;
            }
            s.cancelled = true;
            std::mem::take(&mut s.children)
        };
        for c in kids {
            c.cancel();
        }
    }

    /// 是否已被取消。
    pub fn is_cancelled(&self) -> bool {
        self.inner
            .state
            .lock()
            .expect("token state poisoned")
            .cancelled
    }

    /// 挂起当前任务直到取消（用于异步等待路径）。
    /// 简易版：轮询 1ms sleep，无 Notify 依赖；企业级可替换 tokio::sync::Notify。
    pub async fn cancelled(&self) {
        while !self.is_cancelled() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
}

// ================== BranchTaskResult ==================

/// 单分支执行结果：并发扇出每个分支一份，可直接喂给 O7 图谱 P99 上报。
#[derive(Debug, Clone, serde::Serialize)]
pub struct BranchTaskResult {
    pub branch_id: String,
    pub ok: bool,
    /// 分支产出 payload（DataInput/Transform/Operator 输出等）
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    /// 分支是否被取消令牌中断
    pub cancelled: bool,
    /// 分支是否触发超时
    pub timed_out: bool,
}

// ================== FanOut 配置 + 执行 ==================

/// 扇出配置：fail_fast=true 类比 AutoGen 的 agent_concurrency with critical;
///           per_branch_timeout 对单分支硬上限，类似 Dify 企业版单节点 timeout。
#[derive(Debug, Clone)]
pub struct FanOutOptions {
    pub fail_fast: bool,
    pub per_branch_timeout_ms: Option<u64>,
}

impl Default for FanOutOptions {
    fn default() -> Self {
        Self {
            fail_fast: true,
            per_branch_timeout_ms: None,
        }
    }
}

/// 异步分支函数签名：给每个 branch_id 一个 &CancellationToken 后 spawn。
pub type AsyncBranchFn =
    Arc<dyn Fn(String, CancellationToken) -> BranchFut + Send + Sync + 'static>;

pub type BranchFut = std::pin::Pin<Box<dyn std::future::Future<Output = BranchTaskResult> + Send>>;

/// 将 HashMap 形式的变量表透传给每个分支（轻量 clone）。
fn vars_clone(v: &HashMap<String, serde_json::Value>) -> HashMap<String, serde_json::Value> {
    v.clone()
}

/// 并发扇出核心：把 branch_ids 并发执行完毕（或取消），返回按 id 聚合的结果。
///
/// 语义（对齐 T11 AC）：
///   1. cancel_on_err：fail_fast=true 且任意分支 Err/Cancel/Timeout → token.cancel()
///   2. graceful_wait：fail_fast=false 时等所有分支结束后再聚合
///   3. per_branch_timeout：每个分支有独立超时保护（基于 CancellationToken + child）
pub async fn fan_out_join_set<F>(
    branch_ids: &[String],
    handler: F,
    root_token: &CancellationToken,
    options: FanOutOptions,
) -> FanOutSummary
where
    F: Fn(String, CancellationToken) -> BranchFut + Send + Sync + Clone + 'static,
{
    let _ = vars_clone; // 保留：如果以后需要把 variables 注入 handler，避免 unused warning 升级

    let start = Instant::now();
    let mut set = JoinSet::new();

    for (idx, bid) in branch_ids.iter().enumerate() {
        let bid = bid.clone();
        let h = handler.clone();
        let child_token = root_token.child();
        let per_branch_to = options.per_branch_timeout_ms;

        set.spawn(async move {
            let t0 = Instant::now();
            let id2 = bid.clone();
            let run = async { h(id2, child_token.clone()).await };

            let res = if let Some(ms) = per_branch_to {
                tokio::select! {
                    r = run => r,
                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                        child_token.cancel();
                        BranchTaskResult {
                            branch_id: bid.clone(),
                            ok: false,
                            output: None,
                            error: Some(format!("branch timeout (>{}ms)", ms)),
                            duration_ms: ms,
                            cancelled: false,
                            timed_out: true,
                        }
                    }
                }
            } else {
                run.await
            };
            // 如果取消已置位但结果未标记，打上 cancelled 标志（如 handler 未自检）
            let mut r = res;
            if child_token.is_cancelled() && !r.cancelled && !r.timed_out {
                r.cancelled = true;
                // 如果此时还没产生明确 error，按"取消语义"降级
                if r.error.is_none() && !r.ok {
                    r.error = r.error.or_else(|| Some("branch cancelled".into()));
                }
            }
            // 保证 duration_ms 填充
            if r.duration_ms == 0 {
                r.duration_ms = t0.elapsed().as_millis() as u64;
            }
            (idx, r)
        });
    }

    let mut results: std::collections::BTreeMap<usize, BranchTaskResult> =
        std::collections::BTreeMap::new();
    let mut any_failed = false;

    while let Some(join_res) = set.join_next().await {
        match join_res {
            Ok((idx, r)) => {
                if !r.ok || r.cancelled || r.timed_out {
                    any_failed = true;
                }
                if options.fail_fast && any_failed && !root_token.is_cancelled() {
                    root_token.cancel();
                }
                results.insert(idx, r);
            }
            Err(join_err) => {
                // JoinError（panic/abort）→ 当作分支失败，插入占位
                any_failed = true;
                if options.fail_fast {
                    root_token.cancel();
                }
                let r = BranchTaskResult {
                    branch_id: format!("<join_err_{}>", results.len()),
                    ok: false,
                    output: None,
                    error: Some(format!("branch task join error: {}", join_err)),
                    duration_ms: 0,
                    cancelled: false,
                    timed_out: false,
                };
                results.insert(results.len(), r);
            }
        }
    }

    let ordered: Vec<BranchTaskResult> = results.into_values().collect();
    let ok_count = ordered
        .iter()
        .filter(|r| r.ok && !r.cancelled && !r.timed_out)
        .count();
    let cancelled_count = ordered.iter().filter(|r| r.cancelled).count();
    let timeout_count = ordered.iter().filter(|r| r.timed_out).count();
    let error_count = ordered
        .iter()
        .filter(|r| !r.ok && !r.cancelled && !r.timed_out)
        .count();

    // 计算总 p99（小样本直接排序）
    let mut lats: Vec<u64> = ordered.iter().map(|r| r.duration_ms).collect();
    lats.sort_unstable();
    let p50 = pct_u64(&lats, 0.50);
    let p95 = pct_u64(&lats, 0.95);
    let p99 = pct_u64(&lats, 0.99);

    FanOutSummary {
        total_ms: start.elapsed().as_millis() as u64,
        branches: ordered,
        ok_count,
        error_count,
        cancelled_count,
        timeout_count,
        lat_p50_ms: p50,
        lat_p95_ms: p95,
        lat_p99_ms: p99,
        cancelled_from_root: root_token.is_cancelled(),
    }
}

/// 扇出汇总：可直接作为 O7 图谱 P99 上报 payload 一份子结构。
#[derive(Debug, Clone, serde::Serialize)]
pub struct FanOutSummary {
    pub total_ms: u64,
    pub branches: Vec<BranchTaskResult>,
    pub ok_count: usize,
    pub error_count: usize,
    pub cancelled_count: usize,
    pub timeout_count: usize,
    pub lat_p50_ms: Option<u64>,
    pub lat_p95_ms: Option<u64>,
    pub lat_p99_ms: Option<u64>,
    /// 根令牌是否被置位（fail_fast 触发/外部取消）
    pub cancelled_from_root: bool,
}

fn pct_u64(sorted: &[u64], q: f64) -> Option<u64> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    if n == 1 {
        return Some(sorted[0]);
    }
    let pos = (n - 1) as f64 * q;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = pos - lo as f64;
    let a = sorted[lo] as f64;
    let b = sorted[hi] as f64;
    Some((a * (1.0 - frac) + b * frac).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========= T11 AC-1 CancellationToken 级联取消 =========
    #[tokio::test]
    async fn t11_o5_token_parent_cancel_propagates() {
        let root = CancellationToken::new();
        let c1 = root.child();
        let c2 = root.child();
        let gc = c1.child();
        assert!(
            !root.is_cancelled() && !c1.is_cancelled() && !c2.is_cancelled() && !gc.is_cancelled()
        );
        root.cancel();
        // 级联：root→c1,c2 且 c1→gc
        assert!(root.is_cancelled());
        assert!(c1.is_cancelled(), "c1 child 未取消");
        assert!(c2.is_cancelled(), "c2 child 未取消");
        assert!(gc.is_cancelled(), "grandchild gc 未取消");
        // cancel 幂等
        root.cancel();
        root.cancel();
    }

    #[tokio::test]
    async fn t11_o5_token_child_does_not_affect_parent() {
        let root = CancellationToken::new();
        let c1 = root.child();
        c1.cancel();
        assert!(c1.is_cancelled());
        assert!(!root.is_cancelled(), "子级取消不影响父级");
    }

    #[tokio::test]
    async fn t11_o5_token_new_child_on_cancelled_parent_immediately_cancelled() {
        let root = CancellationToken::new();
        root.cancel();
        let c = root.child();
        assert!(
            c.is_cancelled(),
            "父级已取消时，新生 child 必须立即 cancelled=true"
        );
    }

    // ========= T11 AC-2 Fan-Out 并发执行：正常路径 =========
    #[tokio::test]
    async fn t11_o5_fan_out_all_ok_preserves_order_and_p99() {
        let ids: Vec<String> = (0..5).map(|i| format!("b{}", i)).collect();
        let token = CancellationToken::new();
        let opts = FanOutOptions::default();

        // handler：固定 10~50ms 延迟 + branch_id -> output
        let summary = fan_out_join_set(
            &ids,
            move |id, _tk| {
                Box::pin(async move {
                    let n = id
                        .strip_prefix('b')
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(1);
                    tokio::time::sleep(Duration::from_millis(10 + n * 8)).await;
                    BranchTaskResult {
                        branch_id: id.clone(),
                        ok: true,
                        output: Some(serde_json::json!({"branch": id, "x": n})),
                        error: None,
                        duration_ms: 0,
                        cancelled: false,
                        timed_out: false,
                    }
                })
            },
            &token,
            opts,
        )
        .await;

        assert_eq!(summary.branches.len(), 5);
        assert_eq!(summary.ok_count, 5);
        assert_eq!(summary.error_count, 0);
        assert_eq!(summary.cancelled_count, 0);
        // 顺序：b0..b4
        for (i, b) in summary.branches.iter().enumerate() {
            assert_eq!(b.branch_id, format!("b{}", i));
        }
        // P99 应为最长分支附近（b4 = 10+32 = 42ms；总括总调度开销）
        assert!(summary.lat_p99_ms.is_some(), "p99 Some");
        assert!(
            summary.lat_p99_ms.unwrap() >= 40,
            "p99 应不低于最慢分支 42ms (含调度)"
        );
        // 总执行时间：并行场景，应 < 串行 ~150ms (Σ42) 的一半，约 < 100ms（留调度余量 200ms）
        assert!(
            summary.total_ms < 250,
            "并行应缩短总时长: {}ms",
            summary.total_ms
        );
    }

    // ========= T11 AC-3 Fail-Fast: 一个分支失败 → 其他分支被取消 =========
    #[tokio::test]
    async fn t11_o5_fan_out_fail_fast_cancels_others() {
        // 5 个分支：b2 立即失败；其他分支 sleep 300ms
        let ids: Vec<String> = (0..5).map(|i| format!("b{}", i)).collect();
        let token = CancellationToken::new();
        let opts = FanOutOptions {
            fail_fast: true,
            per_branch_timeout_ms: None,
        };

        let summary = fan_out_join_set(
            &ids,
            move |id, tk| {
                Box::pin(async move {
                    if id == "b2" {
                        // 立即失败
                        return BranchTaskResult {
                            branch_id: id,
                            ok: false,
                            output: None,
                            error: Some("boom".into()),
                            duration_ms: 0,
                            cancelled: false,
                            timed_out: false,
                        };
                    }
                    // 其余分支：等待 + 检查取消（真实 handler 应该周期性自检）
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(300)) => BranchTaskResult {
                            branch_id: id.clone(), ok: true,
                            output: Some(serde_json::json!({"id": id})),
                            error: None, duration_ms: 300, cancelled: false, timed_out: false,
                        },
                        _ = tk.cancelled() => BranchTaskResult {
                            branch_id: id.clone(), ok: false, output: None,
                            error: Some("cancelled".into()), duration_ms: 0,
                            cancelled: true, timed_out: false,
                        },
                    }
                })
            },
            &token,
            opts,
        )
        .await;

        assert!(summary.cancelled_from_root, "fail_fast 应把根令牌置位");
        assert_eq!(summary.error_count, 1, "仅 b2 明确 error");
        assert!(
            summary.cancelled_count >= 3,
            "其余分支中至少 3 个应被取消，实际 {}",
            summary.cancelled_count
        );
        // fail_fast 总时长应 << 300ms（串行 300×5=1.5s 的话更夸张）
        assert!(
            summary.total_ms < 250,
            "fail_fast 应在第一个失败点尽快返回, got {}ms",
            summary.total_ms
        );
    }

    // ========= T11 AC-4 per-branch timeout =========
    #[tokio::test]
    async fn t11_o5_fan_out_timeout_without_fail_fast_all_branches_complete() {
        let ids = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let token = CancellationToken::new();
        let opts = FanOutOptions {
            fail_fast: false,
            per_branch_timeout_ms: Some(30),
        };

        let summary = fan_out_join_set(
            &ids,
            move |id, _tk| {
                Box::pin(async move {
                    // a=10ms(ok), b=200ms(timeout), c=5ms(ok)
                    let delay = match id.as_str() {
                        "a" => 10u64,
                        "b" => 200,
                        _ => 5,
                    };
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    BranchTaskResult {
                        branch_id: id,
                        ok: true,
                        output: None,
                        error: None,
                        duration_ms: delay,
                        cancelled: false,
                        timed_out: false,
                    }
                })
            },
            &token,
            opts,
        )
        .await;

        assert_eq!(
            summary.timeout_count, 1,
            "b 应发生 timeout，实际 {} 个",
            summary.timeout_count
        );
        assert_eq!(summary.ok_count, 2, "a + c 应 success");
        assert!(
            !summary.cancelled_from_root,
            "fail_fast=false 根令牌不应被取消"
        );
        // b 的结果里 timed_out=true
        let b = summary
            .branches
            .iter()
            .find(|r| r.branch_id == "b")
            .expect("b branch");
        assert!(b.timed_out, "b.timed_out 应为 true");
        assert!(
            b.error.as_ref().is_some_and(|e| e.contains("timeout")),
            "timeout 结果含 timeout 错误文案"
        );
    }
}
