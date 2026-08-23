//! PrimiFlow 多维度融合归一化一体化架构层
//!
//! 把两份企业级规范熔铸为**一个**平台：
//! - [`unified`]：归一化统一图模型（GR-STD 12 类 ∪ PT-Primi 六维 + L1-L7 层 + κ‑τ 原语坐标），
//!   并提供三重治理闸门——守恒残差全局闸门(R07)、六维零孤儿绑定(A4)、GR-STD 8 闸门。
//! - [`envelope`]：`PTEnvelope` 归一化跨层消息（PT-Primi §4 接口契约）。
//! - [`registry`]：能力融合 Registry（R06），把 13 crate 能力 + 6 张数据表融合进统一图。
//! - [`platform`]：`PrimiPlatform` 一体化编排，把主链路八模块与统一图、全局闸门织成闭环。
//! - [`config`]：企业级运行时配置（12-factor：文件 + 环境变量 + 校验）。
//! - [`observability`]：结构化日志与请求追踪（对接 Loki/ELK）。
//! - [`server`]：企业级 REST 服务层（Bearer 鉴权 / CORS / 六维溯源查询 / PT-DOC 自生成）。

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
pub const CRATE_ID: &str = "primiflow-fusion";

/// 璇玑系统 Crate 结构化元数据。
#[derive(Debug, Clone, Copy)]
pub struct CrateMeta {
    pub uuid: &'static str,
    pub ais_layers: &'static [&'static str],
    pub owner_project: &'static str,
    pub capabilities: &'static [&'static str],
    pub data_tables_read: &'static [&'static str],
    pub data_tables_write: &'static [&'static str],
}

pub const CRATE_META: CrateMeta = CrateMeta {
    uuid: "7e4ad6f5-b814-47c8-c3e4-f5a6b7c8d9e0",
    ais_layers: &["L2-Gateway", "L3-Service", "L1-Ingress", "L6-Kernel"],
    owner_project: "proj-xuanji-core",
    capabilities: &[
        "GR-STD ∪ PT-Primi 统一图模型 (UnifiedGraph)",
        "三重治理闸门 (R07/A4/GR-STD 8 闸门)",
        "PTEnvelope 跨层归一化消息",
        "13 crate × 6 table 融合 Registry",
        "PrimiPlatform 一体化编排闭环",
        "12-factor 企业级运行时配置",
        "Loki/ELK 结构化日志与追踪",
        "PT-DOC 平台文档自生成",
    ],
    data_tables_read: &["registry.bin", "unified_graph.bin", "config.env"],
    data_tables_write: &["registry.bin"],
};

pub mod config;
pub mod envelope;
pub mod observability;
pub mod platform;
pub mod ptdoc;
pub mod registry;
pub mod server;
pub mod sixdim;
pub mod unified;

/// 便捷再导出：跨层信封
pub use envelope::PTEnvelope;
/// 便捷再导出：一体化平台
pub use platform::{PlatformReport, PrimiPlatform};
pub use ptdoc::{Ptdoc, PtdocSet};
/// 便捷再导出：融合入口、能力清单与六维绑定注册表
pub use registry::{fuse_all, CRATE_NAMES};
pub use sixdim::{now_ms, RegistryStats, SixDimBinding, SixDimRegistry};
/// 便捷再导出：统一图核心类型
pub use unified::{
    EntityKind, Layer, PlatformGate, PrimitiveCoords, RelKind, UnifiedEdge, UnifiedGraph,
    UnifiedNode,
};
