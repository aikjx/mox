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

pub const CRATE_ID: &str = "75238345-b48b-534b-818b-8d9abe083a41";
pub const ENGINE_NAME: &str = "xuanji::primiflow_fusion";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
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
