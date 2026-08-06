//! Step 10 实时联动：把会话累积的流程图推送到 expert-alliance HTTP 服务。
//!
//! feature = "live" 时编译（引入 reqwest）。默认不编译，bridge 仍零第三方 HTTP 依赖。
//! 流程：bridge 后台定期把录制好的 FlowGraph POST 给 `expert-alliance` 的 `/api/ingest`，
//! 前端 `alliance.html` 轮询 `/api/live` 拿到带高亮的 VizBundle，实时联动：
//! 关键路径金黄 / 冲突标红 / 复用青色 / 算法验证卡。

#![cfg(feature = "live")]

use crate::state::BridgeState;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize)]
struct IngestBody<'a> {
    session: &'a str,
    flow: &'a flow_ai::model::FlowGraph,
}

/// 周期性把会话图推送到 expert-alliance 服务。在后台 tokio 任务里调用。
pub async fn push_loop(state: Arc<BridgeState>, session: String, endpoint: String) {
    let client = reqwest::Client::new();
    loop {
        if let Some(g) = state.recorder.snapshot(&session) {
            let body = IngestBody { session: &session, flow: &g };
            if let Err(e) = client
                .post(&format!("{endpoint}/api/ingest"))
                .json(&body)
                .send()
                .await
            {
                eprintln!("[bridge-live] push failed: {e}");
            }
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}
