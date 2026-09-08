// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 企业级网关·域描述符注册中心
//!
//! 本模块仅保留域描述符 `DOMAINS`，供 `/api/v1/domains` 自描述和
//! `/status` 健康检查使用。旧版无状态 `build_gateway_router()` /
//! `serve_axum_gateway()` 及 KG/AI stub 路由器已移除——主入口为
//! `lib.rs::build_gateway_router(state)`，真实域路由由各 service-svc 提供。
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

use serde::Serialize;

// ====================================================================
// 域描述符（用于 /api/v1/domains 自描述）
// ====================================================================
#[derive(Debug, Clone, Serialize)]
pub struct DomainDescriptor {
    pub prefix: &'static str,
    pub name: &'static str,
    pub layer: &'static str,
    pub group: &'static str,  // 能力组：platform/knowledge/ai/orchestration/storage/data/media/commerce/streaming
    pub description: &'static str,
    pub status: &'static str, // "stub" | "ready" | "beta"
}

pub const DOMAINS: &[DomainDescriptor] = &[
    // L0 接入通用
    DomainDescriptor { prefix: "/health",        name: "Health",      layer: "L0", group: "platform", description: "存活/就绪/详细健康检查", status: "ready" },
    DomainDescriptor { prefix: "/metrics",       name: "Metrics",     layer: "L0", group: "platform", description: "Prometheus 指标端点", status: "ready" },
    // L1 IAM（真实 SQLite 仓储，读接口就绪，写接口已接入 IamRepository）
    DomainDescriptor { prefix: "/iam/v1",        name: "IAM",         layer: "L1", group: "platform", description: "身份与访问管理（用户/角色/权限/部门/数据权限/岗位），子域能力，已在 System 域实现", status: "ready" },
    DomainDescriptor { prefix: "/auth/v1",       name: "Auth",        layer: "L1", group: "platform", description: "认证鉴权（登录/Token/刷新/API Key），子域能力，已在 System/Security 域实现", status: "ready" },
    DomainDescriptor { prefix: "/tenant/v1",     name: "Tenant",      layer: "L1", group: "platform", description: "多租户管理（租户CRUD/切换/全表隔离）6 接口，子域能力，已在 System 域实现", status: "ready" },
    DomainDescriptor { prefix: "/rbac/v1",       name: "RBAC",        layer: "L1", group: "platform", description: "角色/权限/当前用户（IAM 真实仓储）3 接口", status: "ready" },
    DomainDescriptor { prefix: "/api/system",    name: "System",      layer: "L1", group: "platform", description: "系统管理（部门/角色/用户/菜单/权限/操作日志/登录日志）46 接口，IamRepository SQLite 真实 CRUD", status: "ready" },
    DomainDescriptor { prefix: "/api/security",  name: "Security",    layer: "L1", group: "platform", description: "安全管理（API Key 全生命周期/安全状态/审计日志）5 接口，SQLite 持久化 + auth 中间件联动", status: "ready" },
    // L2 KG
    DomainDescriptor { prefix: "/kg/v1",         name: "KG",          layer: "L2", group: "knowledge", description: "知识图谱·核心 6 接口", status: "ready" },
    DomainDescriptor { prefix: "/graph/v1",      name: "Graph",       layer: "L2", group: "knowledge", description: "图谱·投影/社区/可视化（与 kg 同源算法）3 接口", status: "ready" },
    DomainDescriptor { prefix: "/api/kb",        name: "KB",          layer: "L2", group: "knowledge", description: "云盘知识库·文档/分析/挂图/检索（mox-kb-svc 100% 自研）", status: "ready" },
    DomainDescriptor { prefix: "/cypher/v1",     name: "Cypher",      layer: "L2", group: "knowledge", description: "Cypher 查询语言（图查询解析/执行），子域能力，已在 Graph/KG 域实现", status: "ready" },
    DomainDescriptor { prefix: "/ngql/v1",       name: "nGQL",        layer: "L2", group: "knowledge", description: "nGQL 查询语言（Nebula 图查询），子域能力，已在 Graph/KG 域实现", status: "ready" },
    // L3 AI
    DomainDescriptor { prefix: "/ai/engine",     name: "AIEngine",    layer: "L3", group: "ai", description: "AI 引擎统一编排 4 接口", status: "ready" },
    DomainDescriptor { prefix: "/ai/v1",         name: "AI-Core",     layer: "L3", group: "ai", description: "AI 核心引擎（模型/推理/提示词/微调），子域能力，已在 AIEngine 域实现", status: "ready" },
    DomainDescriptor { prefix: "/api/experts",   name: "Expert",      layer: "L3", group: "ai", description: "专家智能体集群·注册/协作/调度/图谱/编排/会话 48 接口", status: "ready" },
    DomainDescriptor { prefix: "/intent/v1",     name: "Intent",      layer: "L3", group: "ai", description: "意图识别（NLU/槽位/对话状态/A5 激活扩散），子域能力，已在 AIEngine 域实现", status: "ready" },
    // L4 Alliance（真实 scheduler-core 进程内实现）
    DomainDescriptor { prefix: "/api/alliance",  name: "Alliance",    layer: "L4", group: "ai", description: "专家联盟·调度+执行 20 接口", status: "ready" },
    // L5 Flow
    DomainDescriptor { prefix: "/flow/v1",       name: "Flow",        layer: "L5", group: "orchestration", description: "流程引擎（BPMN/状态机/审批流/流程图谱），子域能力，已在 Alliance 域实现", status: "ready" },
    DomainDescriptor { prefix: "/workflow/v1",   name: "Workflow",    layer: "L4", group: "orchestration", description: "工作流编排（DAG/任务依赖/重试/BPMN+AI），子域能力，已在 Alliance 域实现", status: "ready" },
    DomainDescriptor { prefix: "/bpm/v1",        name: "BPM",         layer: "L4", group: "orchestration", description: "业务流程管理（流程定义/实例/人工任务/审批），子域能力，已在 Alliance 域实现", status: "ready" },
    DomainDescriptor { prefix: "/pipeline/v1",   name: "Pipeline",    layer: "L4", group: "orchestration", description: "流水线（CI/CD/数据管道/AI 管道/P0-P12），子域能力，已在 Alliance 域实现", status: "ready" },
    // L5 Cloud
    DomainDescriptor { prefix: "/cloud/v1",      name: "Cloud",       layer: "L5", group: "storage", description: "对象存储（本地磁盘/S3 兼容语义）6 接口", status: "ready" },
    DomainDescriptor { prefix: "/s3",            name: "S3",          layer: "L5", group: "storage", description: "S3 对象存储（兼容 AWS S3 API/签名/分块上传），子域能力，已在 Cloud 域实现", status: "ready" },
    DomainDescriptor { prefix: "/volume/v1",     name: "Volume",      layer: "L5", group: "storage", description: "卷存储（持久化卷/挂载/快照/EC 纠删码），子域能力，已在 Cloud 域实现", status: "ready" },
    DomainDescriptor { prefix: "/fs/v1",         name: "FS",          layer: "L5", group: "storage", description: "文件存储（本地/NFS/POSIX/分布式文件），子域能力，已在 Cloud 域实现", status: "ready" },
    DomainDescriptor { prefix: "/api/monitor",   name: "Monitor",     layer: "L5", group: "platform", description: "监控运维（指标详情/质量/业务统计/告警汇总/节点日志/链路追踪/告警规则）12 接口，IAM+RuntimeMetrics 真实数据，business_timeseries 待接入历史存储", status: "ready" },
    // L6 Data
    DomainDescriptor { prefix: "/data/v1",       name: "Data",        layer: "L6", group: "data", description: "数据管理（数据集/版本/血缘/资产目录），子域能力，已在 KB 域实现", status: "ready" },
    DomainDescriptor { prefix: "/etl/v1",        name: "ETL",         layer: "L6", group: "data", description: "数据抽取转换加载（CDC/管道/清洗/转换/Fusion），子域能力，已在 KB 域实现", status: "ready" },
    DomainDescriptor { prefix: "/norm/v1",       name: "Norm",        layer: "L6", group: "data", description: "数据规范化（标准化/去重/质量/规约），子域能力，已在 KB 域实现", status: "ready" },
    DomainDescriptor { prefix: "/standard/v1",   name: "Standard",    layer: "L6", group: "data", description: "数据标准（元数据/字典/规范/标准），子域能力，已在 KB 域实现", status: "ready" },
    // L7 Voice
    DomainDescriptor { prefix: "/voice/v1",      name: "Voice",       layer: "L7", group: "media", description: "音频/乐谱/ASR（桥接 melody2score :8012）3 接口", status: "ready" },
    DomainDescriptor { prefix: "/midi/v1",       name: "MIDI",        layer: "L7", group: "media", description: "MIDI 处理（解析/生成/序列化/合成），子域能力，已在 Voice/Melody 域实现", status: "ready" },
    DomainDescriptor { prefix: "/melody/v1",     name: "Melody",      layer: "L7", group: "media", description: "melody2score 转谱桥接（:8012）7 接口", status: "ready" },
    DomainDescriptor { prefix: "/tts/v1",        name: "TTS",         layer: "L7", group: "media", description: "语音合成（文本转语音/多音色/SSML），子域能力，已在 Voice 域实现", status: "ready" },
    // L8 Market
    DomainDescriptor { prefix: "/market/v1",     name: "Market",      layer: "L8", group: "commerce", description: "市场管理（应用市场/AI 插件/商品），子域能力，已在 Market 域实现", status: "ready" },
    DomainDescriptor { prefix: "/shop/v1",       name: "Shop",        layer: "L8", group: "commerce", description: "店铺管理（店铺/商品/库存/在线商店），子域能力，已在 Market 域实现", status: "ready" },
    DomainDescriptor { prefix: "/order/v1",      name: "Order",       layer: "L8", group: "commerce", description: "订单管理（订单/支付/退款/开票），子域能力，已在 Market 域实现", status: "ready" },
    DomainDescriptor { prefix: "/billing/v1",    name: "Billing",     layer: "L8", group: "commerce", description: "计费管理（计量计费/账单/结算/发票），子域能力，已在 Market 域实现", status: "ready" },
    // L9 Streams
    DomainDescriptor { prefix: "/streams/v1",    name: "Streams",     layer: "L9", group: "streaming", description: "流式处理（实时流/窗口/聚合/流处理），子域能力，已在 Melody 域实现", status: "ready" },
    DomainDescriptor { prefix: "/kafka/v1",      name: "Kafka",       layer: "L9", group: "streaming", description: "Kafka 集成（生产者/消费者/主题/消息总线），子域能力，已在 Melody 域实现", status: "ready" },
    DomainDescriptor { prefix: "/ws/v1",         name: "WebSocket",   layer: "L9", group: "streaming", description: "WebSocket 通信（实时推送/双向通信/推流），子域能力，已在 Alliance 域实现", status: "ready" },
    DomainDescriptor { prefix: "/event/v1",      name: "Event",       layer: "L9", group: "streaming", description: "事件驱动（事件总线/订阅/发布/Outbox），子域能力，已在 Alliance 域实现", status: "ready" },
    // L10 Enterprise
    DomainDescriptor { prefix: "/enterprise/v1", name: "Enterprise",  layer: "L10", group: "platform", description: "企业级管理（组织/分权/动态字段），子域能力，已在 System 域实现", status: "ready" },
    DomainDescriptor { prefix: "/platform/v1",   name: "Platform",    layer: "L10", group: "platform", description: "平台治理（配置/版本/升级/模块），子域能力，已在 System 域实现", status: "ready" },
    DomainDescriptor { prefix: "/audit/v1",      name: "Audit",       layer: "L10", group: "platform", description: "审计日志（操作/登录/安全审计/导出），子域能力，已在 Security/System 域实现", status: "ready" },
];
