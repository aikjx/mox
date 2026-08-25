//! AIS-SPEC-9001：企业级统一契约头 —— 模块名 live.rs\n//! AIS-REV-1：自描述接口 · 幂等 · 可观测 · 零外部副作用（网络/IO 仅限封装函数）\n//! AIS-REV-2：公开项 pub fn/pub struct 必须具备 /// 文档注释与错误语义说明\n//! AIS-REV-3：遵循 MOX-AIS-通用 标准，禁止占位实现宏遗留\n\n//! Step 10 实时联动：把会话累积的流程图推送到 mox-expert HTTP 服务。
//!
//! feature = "live" 时编译（引入 reqwest）。默认不编译，bridge 仍零第三方 HTTP 依赖。
//! 流程：bridge 后台定期把录制好的 FlowGraph POST 给 `mox-expert` 的 `/api/ingest`，
//! 前端 `mox.html` 轮询 `/api/live` 拿到带高亮的 VizBundle，实时联动：
//! 关键路径金黄 / 冲突标红 / 复用青色 / 算法验证卡。

#![cfg(feature = "live")]

use crate::state::BridgeState;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize)]
// 说明：struct IngestBody —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
struct IngestBody<'a> {
    session: &'a str,
    flow: &'a flow_ai::model::FlowGraph,
}

/// 周期性把会话图推送到 mox-expert 服务。在后台 tokio 任务里调用。
/// 企业级降级：reqwest 超时 / 远端 5xx / DNS 失败全部转为 warn + continue，
/// 绝不因为联动链路故障而阻塞桥接主循环。
pub async fn push_loop(state: Arc<BridgeState>, session: String, endpoint: String) {
    use reqwest::ClientBuilder;
    // 快速失败：连接 1s / 请求 3s（避免 hang 住整个桥的并发位）
    let client = match ClientBuilder::new()
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[bridge-live] ClientBuilder 失败（降级：禁用 live push）：{e}");
            // 即便 client 构建失败也不 return —— 改为空循环，保持任务存活以便运维能通过日志发现并重启
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }
    };
    let mut consec_errs = 0u32;
    loop {
        // 连续失败退避，避免打日志 / 打网络
        let sleep_ms = if consec_errs == 0 {
            1500u64
        } else {
            std::cmp::min(1500u64 * (consec_errs as u64).saturating_mul(2), 30_000)
        };

        if let Some(g) = state.recorder.snapshot(&session) {
            let body = IngestBody {
                session: &session,
                flow: &g,
            };
            let send_res = tokio::time::timeout(
                Duration::from_secs(4),
                client
                    .post(&format!("{endpoint}/api/ingest"))
                    .json(&body)
                    .send(),
            )
            .await;
            match send_res {
                Ok(Ok(resp)) => {
                    if resp.status().is_success() {
                        consec_errs = 0;
                    } else {
                        consec_errs = consec_errs.saturating_add(1);
                        eprintln!(
                            "[bridge-live] 远端非 2xx（backoff={sleep_ms}ms, consec={consec_errs}）：status={}",
                            resp.status()
                        );
                    }
                }
                Ok(Err(e)) => {
                    consec_errs = consec_errs.saturating_add(1);
                    eprintln!(
                        "[bridge-live] push 发送失败（backoff={sleep_ms}ms, consec={consec_errs}）：{e}"
                    );
                }
                Err(_elapsed) => {
                    consec_errs = consec_errs.saturating_add(1);
                    eprintln!(
                        "[bridge-live] push 超时（backoff={sleep_ms}ms, consec={consec_errs}）"
                    );
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }
}
