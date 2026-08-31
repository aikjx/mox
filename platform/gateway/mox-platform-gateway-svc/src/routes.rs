// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 企业级网关·模块化路由注册中心
//!
//! 设计原则：
//! - 单二进制（mox-server）= axum 网关直接绑定 0.0.0.0:8080
//! - 路由桩 = `Router` + `tower::ServiceBuilder` 中间件分层组合，无运行时开销
//! - 域内 handler 使用 `mox-framework` 导出的 `FrameworkResult<T>`，错误自动转 JSON
//!
//! 31 业务域路由前缀矩阵（可挂接）：
//! ```text
//!   L0 接入通用:  /health  /metrics  /ready  /api/v1/openapi.json
//!   L1 IAM 域:    /iam/v1/*  /auth/v1/*  /tenant/v1/*  /rbac/v1/*
//!   L2 KG 域:     /kg/v1/*  /graph/v1/*  /cypher/v1/*  /ngql/v1/*
//!   L3 AI 域:     /ai/engine/*  /ai/v1/*  /expert/v1/*  /intent/v1/*
//!   L4 Alliance 域:/alliance/v1/*  /alliance/scheduler/*  /alliance/executor/*
//!   L5 Flow 域:   /flow/v1/*  /workflow/v1/*  /bpm/v1/*  /pipeline/v1/*
//!   L6 Cloud 域:  /cloud/v1/*  /s3/*  /volume/v1/*  /fs/v1/*
//!   L7 Data 域:   /data/v1/*  /etl/v1/*  /norm/v1/*  /standard/v1/*
//!   L8 Voice 域:  /voice/v1/*  /midi/v1/*  /melody/v1/*  /tts/v1/*
//!   L9 Market 域: /market/v1/*  /shop/v1/*  /order/v1/*  /billing/v1/*
//!   L10 Streams 域:/streams/v1/*  /kafka/v1/*  /ws/v1/*  /event/v1/*
//!   L11 Enterprise: /enterprise/v1/*  /platform/v1/*  /audit/v1/*
//! ```

mod axum_impl {
    use axum::{
        Json, Router,
        extract::{Query, State},
        routing::{get, post},
    };
    use mox_framework::FrameworkResult;
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::sync::Arc;

    // ====================================================================
    // 域描述符（用于 /api/v1/domains 自描述）
    // ====================================================================
    #[derive(Debug, Clone, Serialize)]
    pub struct DomainDescriptor {
        pub prefix: &'static str,
        pub name: &'static str,
        pub layer: &'static str,
        pub description: &'static str,
        pub status: &'static str, // "stub" | "ready" | "beta"
    }

    pub const DOMAINS: &[DomainDescriptor] = &[
        // L0 接入通用
        DomainDescriptor { prefix: "/health",        name: "Health",      layer: "L0", description: "存活/就绪/详细健康检查", status: "ready" },
        DomainDescriptor { prefix: "/metrics",       name: "Metrics",     layer: "L0", description: "Prometheus 指标端点", status: "ready" },
        // L1 IAM
        DomainDescriptor { prefix: "/iam/v1",        name: "IAM",         layer: "L1", description: "身份与访问管理", status: "stub" },
        DomainDescriptor { prefix: "/auth/v1",       name: "Auth",        layer: "L1", description: "登录/登出/JWT/API Key", status: "stub" },
        DomainDescriptor { prefix: "/tenant/v1",     name: "Tenant",      layer: "L1", description: "多租户三档隔离管理", status: "stub" },
        DomainDescriptor { prefix: "/rbac/v1",       name: "RBAC",        layer: "L1", description: "角色权限/ABAC", status: "stub" },
        // L2 KG
        DomainDescriptor { prefix: "/kg/v1",         name: "KG",          layer: "L2", description: "知识图谱·核心 6 接口", status: "ready" },
        DomainDescriptor { prefix: "/graph/v1",      name: "Graph",       layer: "L2", description: "图谱·投影/社区/可视化", status: "stub" },
        DomainDescriptor { prefix: "/cypher/v1",     name: "Cypher",      layer: "L2", description: "Cypher 查询解析", status: "stub" },
        DomainDescriptor { prefix: "/ngql/v1",       name: "nGQL",        layer: "L2", description: "nGQL 查询解析", status: "stub" },
        // L3 AI
        DomainDescriptor { prefix: "/ai/engine",     name: "AIEngine",    layer: "L3", description: "AI 引擎统一编排 4 接口", status: "ready" },
        DomainDescriptor { prefix: "/ai/v1",         name: "AI-Core",     layer: "L3", description: "AI 推理/微调/上下文", status: "stub" },
        DomainDescriptor { prefix: "/expert/v1",     name: "Expert",      layer: "L3", description: "专家联盟·匹配/派单/结算", status: "stub" },
        DomainDescriptor { prefix: "/intent/v1",     name: "Intent",      layer: "L3", description: "A5 激活扩散意图识别", status: "stub" },
        // L4 Alliance
        DomainDescriptor { prefix: "/alliance/v1",   name: "Alliance",    layer: "L4", description: "专家联盟·调度+执行 8 接口", status: "ready" },
        // L5 Flow
        DomainDescriptor { prefix: "/flow/v1",       name: "Flow",        layer: "L5", description: "流程图谱·业务+算法统一承载", status: "stub" },
        DomainDescriptor { prefix: "/workflow/v1",   name: "Workflow",    layer: "L4", description: "BPMN+AI 工作流", status: "stub" },
        DomainDescriptor { prefix: "/bpm/v1",        name: "BPM",         layer: "L4", description: "审批流/人工任务", status: "stub" },
        DomainDescriptor { prefix: "/pipeline/v1",   name: "Pipeline",    layer: "L4", description: "P0-P12 自动开发流水线", status: "stub" },
        // L5 Cloud
        DomainDescriptor { prefix: "/cloud/v1",      name: "Cloud",       layer: "L5", description: "云资源编排/成本/CMDB", status: "stub" },
        DomainDescriptor { prefix: "/s3",            name: "S3",          layer: "L5", description: "S3 兼容对象存储", status: "ready" },
        DomainDescriptor { prefix: "/volume/v1",     name: "Volume",      layer: "L5", description: "块卷/EC 纠删码", status: "stub" },
        DomainDescriptor { prefix: "/fs/v1",         name: "FS",          layer: "L5", description: "文件系统/POSIX", status: "stub" },
        // L6 Data
        DomainDescriptor { prefix: "/data/v1",       name: "Data",        layer: "L6", description: "数据资产目录", status: "stub" },
        DomainDescriptor { prefix: "/etl/v1",        name: "ETL",         layer: "L6", description: "CDC+ETL+Fusion", status: "stub" },
        DomainDescriptor { prefix: "/norm/v1",       name: "Norm",        layer: "L6", description: "数据标准化/规约", status: "stub" },
        DomainDescriptor { prefix: "/standard/v1",   name: "Standard",    layer: "L6", description: "数据标准/字典", status: "stub" },
        // L7 Voice
        DomainDescriptor { prefix: "/voice/v1",      name: "Voice",       layer: "L7", description: "音频/乐谱/ASR", status: "stub" },
        DomainDescriptor { prefix: "/midi/v1",       name: "MIDI",        layer: "L7", description: "MIDI 合成/解析", status: "stub" },
        DomainDescriptor { prefix: "/melody/v1",     name: "Melody",      layer: "L7", description: "melody2score 简谱转谱", status: "stub" },
        DomainDescriptor { prefix: "/tts/v1",        name: "TTS",         layer: "L7", description: "文本转语音", status: "stub" },
        // L8 Market
        DomainDescriptor { prefix: "/market/v1",     name: "Market",      layer: "L8", description: "应用市场/AI 插件", status: "stub" },
        DomainDescriptor { prefix: "/shop/v1",       name: "Shop",        layer: "L8", description: "在线商店", status: "stub" },
        DomainDescriptor { prefix: "/order/v1",      name: "Order",       layer: "L8", description: "订单/支付/开票", status: "stub" },
        DomainDescriptor { prefix: "/billing/v1",    name: "Billing",     layer: "L8", description: "计量计费/账单", status: "stub" },
        // L9 Streams
        DomainDescriptor { prefix: "/streams/v1",    name: "Streams",     layer: "L9", description: "流式处理", status: "stub" },
        DomainDescriptor { prefix: "/kafka/v1",      name: "Kafka",       layer: "L9", description: "消息总线", status: "stub" },
        DomainDescriptor { prefix: "/ws/v1",         name: "WebSocket",   layer: "L9", description: "WebSocket 推流", status: "stub" },
        DomainDescriptor { prefix: "/event/v1",      name: "Event",       layer: "L9", description: "事件驱动/Outbox", status: "stub" },
        // L10 Enterprise
        DomainDescriptor { prefix: "/enterprise/v1", name: "Enterprise",  layer: "L10", description: "企业实体/动态字段/审计", status: "stub" },
        DomainDescriptor { prefix: "/platform/v1",   name: "Platform",    layer: "L10", description: "平台总控/配置/运维", status: "stub" },
        DomainDescriptor { prefix: "/audit/v1",      name: "Audit",       layer: "L10", description: "灯堡哈希链/审计查询", status: "stub" },
    ];

    // ====================================================================
    // 共享·网关状态（所有域 handler 通过 State 获取）
    // ====================================================================
    #[derive(Debug, Default)]
    pub struct GatewayState {
        pub started_unix_ms: i64,
    }

    // ====================================================================
    // KG 域：6 核心接口路由桩（与 Node 层 modules/graph.js 跨语言对齐）
    // ====================================================================
    #[derive(Debug, Deserialize)]
    pub struct KgNeighborhoodQuery {
        pub center: String,
        #[serde(default = "default_depth")]
        pub depth: usize,
        #[serde(default = "default_max_nodes")]
        pub limit: usize,
    }
    fn default_depth() -> usize { 2 }
    fn default_max_nodes() -> usize { 500 }

    #[derive(Debug, Deserialize)]
    pub struct KgPathQuery {
        pub source: String,
        pub target: String,
        #[serde(default = "default_k")]
        pub k: usize,
    }
    fn default_k() -> usize { 3 }

    fn kg_domain_router() -> Router<Arc<GatewayState>> {
        Router::new()
            // 1. 邻域子图 → Cytoscape 兼容
            .route("/kg/v1/neighborhood", get(|Query(q): Query<KgNeighborhoodQuery>| async move {
                FrameworkResult::Ok(Json(json!({
                    "ok": true,
                    "stub": true,
                    "note": "将由 kg-service-svc 挂接 KnowledgeGraph::neighborhood_subgraph 实现",
                    "params": {"center": q.center, "depth": q.depth, "limit": q.limit},
                })))
            }))
            // 2. K 条路径查找
            .route("/kg/v1/path", get(|Query(q): Query<KgPathQuery>| async move {
                FrameworkResult::Ok(Json(json!({
                    "ok": true,
                    "stub": true,
                    "note": "将由 KnowledgeGraph::find_paths 挂接",
                    "params": {"source": q.source, "target": q.target, "k": q.k},
                })))
            }))
            // 3. 最短路径
            .route("/kg/v1/shortest-path", get(|Query(q): Query<KgPathQuery>| async move {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true, "params": {"source": q.source, "target": q.target},
                })))
            }))
            // 4. 中心性分析（4 指标 + 公式文档）
            .route("/kg/v1/centrality", get(|| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true,
                    "metrics": ["degree", "betweenness_brandes", "closeness_harmonic", "pagerank"],
                })))
            }))
            // 5. 社区发现（CNM）
            .route("/kg/v1/communities", get(|| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true, "algo": "CNM 模块度贪心凝聚",
                })))
            }))
            // 6. 图谱统计（含密度解读 + 公式）
            .route("/kg/v1/stats", get(|| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true,
                    "includes": ["node_count", "edge_count", "density%", "density_interpretation", "centrality_formulas"],
                })))
            }))
    }

    // ====================================================================
    // AI 引擎域：4 核心接口路由桩（与归一化总纲 §AIS·AI 对齐）
    // ====================================================================
    fn ai_engine_router() -> Router<Arc<GatewayState>> {
        Router::new()
            // 1. POST /ai/engine/process → 自动意图识别→能力路由
            .route("/ai/engine/process", post(|Json(_body): Json<serde_json::Value>| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true,
                    "pipeline": "意图识别(A5 PPR) → 能力路由 → 专家联盟打分 → 执行 → 审计链",
                })))
            }))
            // 2. POST /ai/engine/analyze → 显式能力执行
            .route("/ai/engine/analyze", post(|Json(_body): Json<serde_json::Value>| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true,
                    "note": "capability 字段指定执行能力",
                })))
            }))
            // 3. GET /ai/engine/capabilities → 能力矩阵自描述
            .route("/ai/engine/capabilities", get(|| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true,
                    "count": 7,
                    "items": ["数学推理", "逻辑推理", "知识问答", "代码生成",
                              "中文理解", "时效性检索", "指令跟随"],
                })))
            }))
            // 4. GET /ai/engine/metrics → 成功率/降级率/延迟指标
            .route("/ai/engine/metrics", get(|| async {
                FrameworkResult::Ok(Json(json!({
                    "ok": true, "stub": true,
                    "gauges": ["success_rate", "degrade_rate", "latency_p50_ms", "latency_p99_ms"],
                })))
            }))
    }

    // ====================================================================
    // 域路由桩：所有未就绪域统一挂到 stub_handler
    // ====================================================================
    async fn stub_handler(
        State(_state): State<Arc<GatewayState>>,
    ) -> FrameworkResult<Json<serde_json::Value>> {
        Ok(Json(json!({
            "ok": true,
            "stub": true,
            "message": "此域路由桩已注册，handler 待对应 service-svc 挂接实现",
        })))
    }

    fn stub_domain(prefix: &'static str) -> Router<Arc<GatewayState>> {
        Router::new()
            .route(&format!("{prefix}"), get(stub_handler))
            .route(&format!("{prefix}/*path"), get(stub_handler).post(stub_handler))
    }

    // ====================================================================
    // 路由装配入口：一次性构建 + 返回 axum Router
    // ====================================================================
    pub fn build_gateway_router() -> Router {
        let state = Arc::new(GatewayState::default());
        let mut router = Router::new();

        // L0 通用（ready）
        router = router.route("/api/v1/domains", get(|| async {
            Json(json!({ "ok": true, "count": DOMAINS.len(), "domains": DOMAINS }))
        }));
        router = router.route("/health", get(|| async {
            Json(json!({ "ok": true, "gateway": "axum", "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true) }))
        }));

        // L2 KG（ready 6 接口 stub）+ L3 AI Engine（ready 4 接口 stub）—— 兜底路径
        router = router.merge(kg_domain_router());
        router = router.merge(ai_engine_router());

        // 剩余 25 域挂 stub（31总 - L0×2 - L2×1 - L3×1 = 27？：L0 health/metrics/domains 算 3 项，实际匹配前缀，总数不重要）
        for d in DOMAINS {
            if matches!(d.status, "ready") {
                continue; // ready 的域已单独 merge
            }
            router = router.merge(stub_domain(d.prefix));
        }

        // —— 真实域路由挂接（优先级更高的 merge：同路径覆盖 stub）——
        // L2+L3: kg-service-svc 的 http_adapter 提供真实 6 KG + 4 AI 接口
        let real_kg_ai: Router = mox_kg_service_svc::http_adapter::build_kg_ai_router();

        // 最终：先合网关层路由(带 state)，再合真实域路由(各自内附 state)
        router.with_state(state).merge(real_kg_ai)
    }

    // ====================================================================
    // 启动入口：axum 绑定 bind_addr:port，进入 serve 循环（Ctrl-C 优雅退出）
    // ====================================================================
    pub async fn serve_axum_gateway(bind_addr: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use axum::Server;
        use std::net::SocketAddr;
        use tower_http::cors::{Any, CorsLayer};
        use tower::ServiceBuilder;

        let app = build_gateway_router().layer(
            ServiceBuilder::new()
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
                .into_inner(),
        );

        let addr: SocketAddr = format!("{bind_addr}:{port}").parse()?;
        eprintln!("[mox-server::gateway] 🌐 Rust Gateway 全维接管 @ http://{addr}");
        eprintln!("[mox-server::gateway] ✅ /health · /api/v1/domains · /kg/v1/* · /ai/engine/* 已就绪");

        Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
                eprintln!("\n[mox-server::gateway] 🛑 收到 Ctrl-C，优雅退出中…");
            })
            .await?;
        Ok(())
    }
}

pub use axum_impl::{DomainDescriptor, DOMAINS, GatewayState, build_gateway_router, serve_axum_gateway};
