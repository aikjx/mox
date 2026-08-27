// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! URN 身份规范：企业知识中枢的全局唯一标识。
//!
//! 形如 `urn:kg:<tenant>:<layer>:<kind>:<key>`，例如
//! `urn:kg:default:L5:code:crates/operator-core/src/lib.rs`。
//!
//! 三套来源图（静态关图 / 运行时 AI 知识图 / 六维统一图）各自的 id 命名法互不兼容，
//! URN 是把它们归一到同一身份空间的唯一手段——**同一实体在任何来源都必须解析出同一 URN**，
//! 这是「三图归一」可去重、可合并的第一性前提。

use mox_flow_fusion_svc::{EntityKind, Layer};

pub const URN_PREFIX: &str = "urn:kg";
pub const DEFAULT_TENANT: &str = "default";

/// 实体类型 → 稳定短码（URN 片段，禁止变更，否则破坏历史身份）
pub fn kind_code(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Requirement => "req",
        EntityKind::Feature => "fun",
        EntityKind::Business => "biz",
        EntityKind::Algorithm => "alg",
        EntityKind::Task => "tsk",
        EntityKind::Code => "cod",
        EntityKind::Data => "data",
        EntityKind::Function => "func",
        EntityKind::Interface => "iface",
        EntityKind::Script => "script",
        EntityKind::ScheduleTask => "cron",
        EntityKind::Config => "config",
        EntityKind::Dependency => "dep",
        EntityKind::ThirdParty => "thirdparty",
        EntityKind::Doc => "doc",
        EntityKind::Runtime => "mox_platform_orchestrator_svc",
        EntityKind::DataSchema => "schema",
        EntityKind::DataStore => "store",
        EntityKind::Loop => "loop",
        EntityKind::Graph => "graph",
    }
}

/// 短码 → 实体类型
pub fn parse_kind(code: &str) -> Option<EntityKind> {
    let k = match code {
        "req" => EntityKind::Requirement,
        "fun" => EntityKind::Feature,
        "biz" => EntityKind::Business,
        "alg" => EntityKind::Algorithm,
        "tsk" => EntityKind::Task,
        "cod" => EntityKind::Code,
        "data" => EntityKind::Data,
        "func" => EntityKind::Function,
        "iface" => EntityKind::Interface,
        "script" => EntityKind::Script,
        "cron" => EntityKind::ScheduleTask,
        "config" => EntityKind::Config,
        "dep" => EntityKind::Dependency,
        "thirdparty" => EntityKind::ThirdParty,
        "doc" => EntityKind::Doc,
        "mox_platform_orchestrator_svc" => EntityKind::Runtime,
        "schema" => EntityKind::DataSchema,
        "store" => EntityKind::DataStore,
        "loop" => EntityKind::Loop,
        "graph" => EntityKind::Graph,
        _ => return None,
    };
    Some(k)
}

/// 层短码 → 层（`Layer::code()` 的逆映射）
pub fn parse_layer(code: &str) -> Option<Layer> {
    let l = match code {
        "L1" => Layer::RequirementSemantic,
        "L2" => Layer::PrimitiveMapping,
        "L3" => Layer::TopologyEmergence,
        "L4" => Layer::Orchestration,
        "L5" => Layer::ExecutionRuntime,
        "L6" => Layer::AssetPrecipitation,
        "L7" => Layer::Governance,
        _ => return None,
    };
    Some(l)
}

/// key 规范化：URN 以 `:` 分段，key 内的 `:` 必须转义，反斜杠统一为正斜杠。
pub fn normalize_key(key: &str) -> String {
    key.trim()
        .replace('\\', "/")
        .replace(':', "_")
        .trim_start_matches("./")
        .to_string()
}

/// 构造 URN
pub fn build(tenant: &str, layer: Layer, kind: EntityKind, key: &str) -> String {
    let t = if tenant.trim().is_empty() {
        DEFAULT_TENANT
    } else {
        tenant.trim()
    };
    format!(
        "{}:{}:{}:{}:{}",
        URN_PREFIX,
        t,
        layer.code(),
        kind_code(kind),
        normalize_key(key)
    )
}

/// 用默认租户构造 URN
pub fn build_default(layer: Layer, kind: EntityKind, key: &str) -> String {
    build(DEFAULT_TENANT, layer, kind, key)
}

/// 解析结果
#[derive(Debug, Clone, PartialEq)]
pub struct Urn {
    pub tenant: String,
    pub layer: Layer,
    pub kind: EntityKind,
    pub key: String,
}

/// 解析 URN，非法返回 None
pub fn parse(s: &str) -> Option<Urn> {
    // urn:kg:<tenant>:<layer>:<kind>:<key>，key 允许含 `/` 与 `.`，但不含 `:`
    let parts: Vec<&str> = s.splitn(6, ':').collect();
    if parts.len() != 6 {
        return None;
    }
    if parts[0] != "urn" || parts[1] != "kg" {
        return None;
    }
    Some(Urn {
        tenant: parts[2].to_string(),
        layer: parse_layer(parts[3])?,
        kind: parse_kind(parts[4])?,
        key: parts[5].to_string(),
    })
}

/// 是否已是合法 URN（用于接入时判断"需不需要再归一"）
pub fn is_urn(s: &str) -> bool {
    parse(s).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_then_parse_roundtrip() {
        let u = build_default(
            Layer::ExecutionRuntime,
            EntityKind::Code,
            "crates/operator-core/src/lib.rs",
        );
        assert_eq!(u, "urn:kg:default:L5:cod:crates/operator-core/src/lib.rs");
        let p = parse(&u).expect("must parse");
        assert_eq!(p.tenant, "default");
        assert_eq!(p.layer, Layer::ExecutionRuntime);
        assert_eq!(p.kind, EntityKind::Code);
        assert_eq!(p.key, "crates/operator-core/src/lib.rs");
    }

    #[test]
    fn windows_path_and_colon_are_normalized() {
        let u = build_default(Layer::AssetPrecipitation, EntityKind::Doc, r".\docs\a:b.md");
        assert_eq!(u, "urn:kg:default:L6:doc:docs/a_b.md");
        assert!(is_urn(&u));
    }

    #[test]
    fn all_twenty_kind_codes_roundtrip() {
        let kinds = [
            EntityKind::Requirement,
            EntityKind::Feature,
            EntityKind::Business,
            EntityKind::Algorithm,
            EntityKind::Task,
            EntityKind::Code,
            EntityKind::Data,
            EntityKind::Function,
            EntityKind::Interface,
            EntityKind::Script,
            EntityKind::ScheduleTask,
            EntityKind::Config,
            EntityKind::Dependency,
            EntityKind::ThirdParty,
            EntityKind::Doc,
            EntityKind::Runtime,
            EntityKind::DataSchema,
            EntityKind::DataStore,
            EntityKind::Loop,
            EntityKind::Graph,
        ];
        // 20 类必须全覆盖且短码互不重复
        let mut seen = std::collections::HashSet::new();
        for k in kinds {
            let c = kind_code(k);
            assert!(seen.insert(c), "短码重复: {c}");
            assert_eq!(parse_kind(c), Some(k));
        }
        assert_eq!(seen.len(), 20);
    }

    #[test]
    fn all_seven_layers_roundtrip() {
        for l in [
            Layer::RequirementSemantic,
            Layer::PrimitiveMapping,
            Layer::TopologyEmergence,
            Layer::Orchestration,
            Layer::ExecutionRuntime,
            Layer::AssetPrecipitation,
            Layer::Governance,
        ] {
            assert_eq!(parse_layer(l.code()), Some(l));
        }
    }

    #[test]
    fn illegal_urn_rejected() {
        assert!(!is_urn("CodeFile:crates/a/src/lib.rs"));
        assert!(!is_urn("urn:kg:default:L9:cod:x"));
        assert!(!is_urn("urn:kg:default:L5:nope:x"));
        assert!(!is_urn("urn:other:default:L5:cod:x"));
    }
}
