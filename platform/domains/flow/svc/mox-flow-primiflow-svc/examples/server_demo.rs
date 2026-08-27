// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! PrimiFlow API 服务示例：真正启动 HTTP 服务，对外暴露 κ‑τ 引擎。
//!
//! 运行：
//! ```bash
//! cargo run -p primiflow --example server_demo
//! ```
//! 然后用 curl / 浏览器访问契约端点，例如：
//! ```bash
//! curl -X POST localhost:3000/api/projects \
//!   -H 'content-type: application/json' \
//!   -d '{"name":"经营分析","description":"每天抓取销售数据，清洗对账后生成图表报告。对接 PostgreSQL。"}'
//! curl 'localhost:3000/api/projects'            # 项目审计清单（含 κ/τ/守恒/绑定/Q）
//! curl 'localhost:3000/api/projects/<id>'       # 单个项目详情
//! curl 'localhost:3000/api/assets?q=report'     # 检索知识库资产
//! ```
//!
//! 服务启动时自动从 `primiflow.db` 重放知识库与六维溯源主图，重启后拓扑荷 Q 连续复用。

use std::path::PathBuf;

use mox_flow_primiflow_svc::persistence::Persistence;
use mox_flow_primiflow_svc::server::{new_state, serve, API_CONTRACT};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from("./primiflow_runtime");
    std::fs::create_dir_all(&out_dir).ok();
    let db_path = out_dir.join("primiflow.db");
    let store =
        Persistence::sqlite(db_path.to_str().unwrap()).unwrap_or_else(|_| Persistence::memory());

    let state = new_state(out_dir, store);

    println!("PrimiFlow API 契约：");
    for (m, p, d) in API_CONTRACT {
        println!("  {m:5} {p:40} {d}");
    }

    serve(state, "0.0.0.0:3000").await?;
    Ok(())
}
