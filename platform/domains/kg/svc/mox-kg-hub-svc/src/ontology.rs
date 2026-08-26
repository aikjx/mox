//! 本体归一：把三套来源图各自的类型词汇映射到唯一本体（`EntityKind`/`RelKind`/`Layer`）。
//!
//! - 静态关图 `tools/info-graph`：13 类 `InfoKind` 字符串（`CodeFile`/`Doc`/...）
//! - 运行时 AI 知识图 `graph-algorithms`：`node_type` 是**自由字符串**，需模糊归一
//! - 六维统一图 `primiflow-fusion`：已是本体，直通
//!
//! 归一化是有损的映射决策，因此每条规则都显式写死，不做隐式猜测：
//! 无法识别的来源类型统一落到 `Data`（信息兜底），并由治理层作为"待人工确认"暴露，
//! 而不是静默丢弃——丢数据比归错类更危险。

use mox_flow_fusion_svc::{EntityKind, Layer, RelKind};

/// 静态关图 `InfoKind` 字符串 → 本体实体类型。
///
/// 注意 `CodeFile` → `Code`：本体用六维的 `Code` 承载代码实体，避免同义类重复。
pub fn map_info_kind(s: &str) -> EntityKind {
    match s {
        "Business" => EntityKind::Business,
        "Data" => EntityKind::Data,
        "Function" => EntityKind::Function,
        "Interface" => EntityKind::Interface,
        "CodeFile" => EntityKind::Code,
        "Script" => EntityKind::Script,
        "ScheduleTask" => EntityKind::ScheduleTask,
        "Config" => EntityKind::Config,
        "Dependency" => EntityKind::Dependency,
        "ThirdParty" => EntityKind::ThirdParty,
        "Doc" => EntityKind::Doc,
        "Runtime" => EntityKind::Runtime,
        "Requirement" => EntityKind::Requirement,
        _ => EntityKind::Data,
    }
}

/// 运行时 AI 知识图的自由 `node_type` → 本体实体类型（大小写无关 + 别名容错）。
pub fn map_node_type(s: &str) -> EntityKind {
    let t = s.trim().to_ascii_lowercase();
    match t.as_str() {
        "requirement" | "req" | "需求" => EntityKind::Requirement,
        "feature" | "fun" | "功能" => EntityKind::Feature,
        "business" | "biz" | "业务" | "业务流程" => EntityKind::Business,
        "algorithm" | "alg" | "算法" | "operator" | "算子" => EntityKind::Algorithm,
        "task" | "tsk" | "任务" => EntityKind::Task,
        "code" | "codefile" | "cod" | "代码" => EntityKind::Code,
        "function" | "func" | "函数" | "method" => EntityKind::Function,
        "interface" | "api" | "endpoint" | "接口" | "路由" | "route" => EntityKind::Interface,
        "script" | "脚本" => EntityKind::Script,
        "scheduletask" | "cron" | "定时任务" => EntityKind::ScheduleTask,
        "config" | "配置" => EntityKind::Config,
        "dependency" | "dep" | "依赖" | "crate" | "package" => EntityKind::Dependency,
        "thirdparty" | "external" | "第三方" => EntityKind::ThirdParty,
        "doc" | "document" | "文档" | "markdown" | "wiki" => EntityKind::Doc,
        "runtime" | "service" | "运行时" | "服务" => EntityKind::Runtime,
        "dataschema" | "schema" | "table" | "表" | "模型" => EntityKind::DataSchema,
        "datastore" | "store" | "database" | "db" | "存储" => EntityKind::DataStore,
        "loop" | "闭环" => EntityKind::Loop,
        "graph" | "topology" | "拓扑" | "图" => EntityKind::Graph,
        // 会话/知识条目等 AI 侧产物统一视为数据信息
        _ => EntityKind::Data,
    }
}

/// 关系词汇 → 本体关系类型（兼容静态关图 8 类与 AI 图自由 `relation_type`）。
pub fn map_relation(s: &str) -> RelKind {
    let t = s.trim().to_ascii_lowercase();
    match t.as_str() {
        "bind" | "六维绑定" | "绑定" => RelKind::Bind,
        "dataflow" | "data_flow" | "数据流" => RelKind::DataFlow,
        "loopback" | "loop_back" | "闭环回流" => RelKind::LoopBack,
        "branch" | "分支汇聚" | "分支" => RelKind::Branch,
        "trigger" | "触发" => RelKind::Trigger,
        "call" | "调用" | "calls" => RelKind::Call,
        "readwrite" | "read_write" | "读写" => RelKind::ReadWrite,
        "reference" | "ref" | "引用" | "relates_to" | "related" => RelKind::Reference,
        "dependency" | "depends" | "依赖" => RelKind::Dependency,
        "inheritance" | "implements" | "继承" => RelKind::Inheritance,
        "configref" | "config_ref" | "配置引用" => RelKind::ConfigRef,
        "deploy" | "部署" | "承载" => RelKind::Deploy,
        _ => RelKind::Reference,
    }
}

/// 实体类型 → 默认所属层（PT-Primi L1-L7）。
///
/// 语义层次而非物理位置：需求在 L1，算法在 L2/L3，任务编排在 L4，
/// 可执行实体在 L5，沉淀资产（文档/数据/配置）在 L6，治理类在 L7。
pub fn default_layer(kind: EntityKind) -> Layer {
    match kind {
        EntityKind::Requirement => Layer::RequirementSemantic,
        EntityKind::Feature | EntityKind::Business => Layer::PrimitiveMapping,
        EntityKind::Algorithm | EntityKind::Loop | EntityKind::Graph => Layer::TopologyEmergence,
        EntityKind::Task | EntityKind::ScheduleTask => Layer::Orchestration,
        EntityKind::Code
        | EntityKind::Function
        | EntityKind::Interface
        | EntityKind::Script
        | EntityKind::Runtime => Layer::ExecutionRuntime,
        EntityKind::Doc
        | EntityKind::Data
        | EntityKind::Config
        | EntityKind::DataSchema
        | EntityKind::DataStore
        | EntityKind::Dependency
        | EntityKind::ThirdParty => Layer::AssetPrecipitation,
    }
}

/// 该实体是否属于「核心实现实体」——治理层偏离检测（GR-E6）只对核心实体要求需求溯源。
/// 文档/配置/依赖/第三方不强制挂需求根，否则会产生大量噪声告警。
pub fn is_core_impl(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::Code
            | EntityKind::Function
            | EntityKind::Interface
            | EntityKind::Script
            | EntityKind::Runtime
            | EntityKind::Business
            | EntityKind::Data
            | EntityKind::Algorithm
            | EntityKind::Task
            | EntityKind::Feature
    )
}

/// 六维顺序（REQ→FUN→BIZ→ALG→TSK→COD），用于溯源链渲染与完备性校验
pub const SIX_DIM_ORDER: [EntityKind; 6] = [
    EntityKind::Requirement,
    EntityKind::Feature,
    EntityKind::Business,
    EntityKind::Algorithm,
    EntityKind::Task,
    EntityKind::Code,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_kind_thirteen_classes_all_mapped() {
        // 静态关图 13 类必须全部有明确归一目标，且 CodeFile 归到 Code
        assert_eq!(map_info_kind("CodeFile"), EntityKind::Code);
        assert_eq!(map_info_kind("Requirement"), EntityKind::Requirement);
        assert_eq!(map_info_kind("ScheduleTask"), EntityKind::ScheduleTask);
        assert_eq!(map_info_kind("ThirdParty"), EntityKind::ThirdParty);
        // 未知类型兜底为 Data，绝不 panic、绝不丢弃
        assert_eq!(map_info_kind("SomethingNew"), EntityKind::Data);
    }

    #[test]
    fn node_type_is_case_and_alias_tolerant() {
        assert_eq!(map_node_type("API"), EntityKind::Interface);
        assert_eq!(map_node_type("  Endpoint "), EntityKind::Interface);
        assert_eq!(map_node_type("算子"), EntityKind::Algorithm);
        assert_eq!(map_node_type("CodeFile"), EntityKind::Code);
        assert_eq!(map_node_type("会话消息"), EntityKind::Data);
    }

    #[test]
    fn relation_alias_maps_to_bind() {
        assert_eq!(map_relation("Bind"), RelKind::Bind);
        assert_eq!(map_relation("六维绑定"), RelKind::Bind);
        assert_eq!(map_relation("relates_to"), RelKind::Reference);
        assert_eq!(map_relation("未知关系"), RelKind::Reference);
    }

    #[test]
    fn six_dim_layers_are_monotonic_by_design() {
        // 六维默认层不得倒挂：REQ(L1) 必须早于 COD(L5)
        let req = default_layer(EntityKind::Requirement).code();
        let cod = default_layer(EntityKind::Code).code();
        assert_eq!(req, "L1");
        assert_eq!(cod, "L5");
    }

    #[test]
    fn docs_and_config_are_not_core_impl() {
        assert!(!is_core_impl(EntityKind::Doc));
        assert!(!is_core_impl(EntityKind::Config));
        assert!(!is_core_impl(EntityKind::Dependency));
        assert!(is_core_impl(EntityKind::Code));
        assert!(is_core_impl(EntityKind::Interface));
    }
}
