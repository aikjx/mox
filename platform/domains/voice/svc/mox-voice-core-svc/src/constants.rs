// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 常量归一化（FR-14 SSOT）—— Python 版 10+ 文件硬编码常量根治
//!
//! 所有"阈值 / 协议版本 / 扣分权重 / 维度优先级 / 热词长度 / S6 学习周期"统一收口于此。
//! 改动只需改一处，`cargo test -p xiaobai-core t14_constants_ssot` 一致性用例保证不漂移。

/// Crate 元数据（与 mox-common-meta CRATE_ID 风格对齐：唯一不可变 UUID）
pub const XIAOBAI_CRATE_ID: &str = "xb9c1f42-a9d8-53b7-9c0a-60e10b6c8f03";
pub const XIAOBAI_ENGINE_NAME: &str = "mox::mox_voice_core_svc::OperatorEngine";
/// voice_proxy JSON 信封协议版本——和 Python `AIS_FR13_V1 = "AIS-FR13/V1.0"` 字符级一致
pub const XIAOBAI_PROTOCOL_VERSION: &str = "AIS-FR13/V1.0";

// ============== 意图路由（PPR）阈值 ==============
/// 歧义判定阈值：top1_score - top2_score ≤ 该值 → 判定歧义 → 联盟裁决 (INTENT_AMBIGUOUS)
/// 与 Python `router.AMBIGUITY_THRESHOLD = 0.1` 数值一致
pub const AMBIGUITY_THRESHOLD: f32 = 0.10;
/// 单句匹配中规则命中的最大候选数（超限直接裁 top N，避免 PPR 扩散耗时）
pub const INTENT_MAX_CANDIDATES: usize = 10;

// ============== FR-5 热词注入 ==============
/// 单条热词最小/最大长度（中文字符数，含中英文）——对应 Python MAX_HOTWORD_LEN = 40
pub const MAX_HOTWORD_LEN: usize = 40;
pub const MIN_HOTWORD_LEN: usize = 1;
/// 热词加权 score 合法范围 [0.0, 100.0]
pub const HOTWORD_SCORE_MIN: f32 = 0.0_f32;
pub const HOTWORD_SCORE_MAX: f32 = 100.0_f32;
/// FR-5 S1 ContextConfig 默认 context_score（sherpa-rs 文档推荐 1.5 偏置量）
pub const DEFAULT_CONTEXT_SCORE: f32 = 1.5_f32;
/// FR-5 S3 post-hoc 模糊替换最大 Levenshtein 差异率（相对热词长度 ≤ 35% 才生效，适配 2/6 字 ASR 误差）
pub const HOTWORD_FUZZY_MAX_RATIO: f32 = 0.35_f32;
/// S3 单 ASR 解码结果应用的热词替换上限（防止噪声放大）
pub const HOTWORD_POSTHOC_MAX_REPLACES: usize = 8;

// ============== voice_proxy 桥 ==============
/// cloud_fallback 下联盟裁决 800ms 超时（与 Python dispatch_intent CLOUD_DEADLINE_MS = 800 对齐）
pub const BRIDGE_CLOUD_DEADLINE_MS: u64 = 800;
/// WebSocket 心跳周期（毫秒）
pub const BRIDGE_PING_INTERVAL_MS: u64 = 15_000;
/// WebSocket 断连最大重试次数（指数退避 1s→2s→4s→8s，4 次后判定 BRIDGE_DISCONNECTED）
pub const BRIDGE_MAX_RETRIES: u32 = 4;

// ============== RBAC / PII ==============
/// file_operator 命中 PII 敏感资源时，强制把 clearance 要求升到该等级
pub const PII_SENSITIVE_FORCE_LEVEL: u8 = 3; // L3 MoxAdmin
/// 常量：敏感前缀表（与 mox-expert/sensitivity.rs SENSITIVE_DOMAINS 一致，冗余一份保证本 crate 不依赖 mox-expert feature gate 即可跑通单测）
pub const SENSITIVE_DOMAINS: &[&str] = &["citizen_", "pii", "id_card", "phone", "bank_card"];
pub const DESENSITIZED_SUFFIXES: &[&str] = &["_safe", "_desensitized", "_masked", "_anon"];

// ============== 专家联盟裁决权重 ==============
/// 14 位专家维度优先级（序号越小越先触发一票否决；对齐 mox-expert/alliance.rs 常量）
pub const EXPERT_DIM_PRIORITY: &[&str] = &[
    "security",      // G1 安全闸门：一票否决
    "permission",    // G2 权限闸门：一票否决
    "resource",      // G3 资源：CPU/内存/RAM
    "performance",   // G4 性能：P99 延迟
    "algorithm",     // G5 算法：收敛/拓扑
    "data",          // G6 数据：精度/漂移
    "architecture",  // G7 架构：耦合/分层
    "code_quality",  // G8 质量：覆盖率/可维护
    "observability",
    "business",
    "documentation",
    "maintainability",
    "testing",
    "performance",
];
/// 冲突裁决扣分权重（MustSerialize 冲突 Parallelize 扣 15 分）
pub const CONFLICT_SEMANTIC_PENALTY: u16 = 15;

// ============== S6 学习模块 ==============
/// S6 每周热词 score 回流 & 模型权重更新周期（7 天 = 604_800_000 ms）
pub const S6_WEEKLY_CYCLE_MS: u64 = 7 * 24 * 60 * 60 * 1_000;
/// S6 单次热词调整最大步长（避免一次学习导致热词权重剧烈跳变）
pub const S6_HOTWORD_MAX_STEP: f32 = 5.0_f32;

// ============== 常量一致性测试（T14）==============
#[cfg(test)]
mod t14_constants_ssot {
    use super::*;

    /// 关键与 Python 对齐的常量值断言——任何修改都会导致单测失败，强制人工评审
    #[test]
    fn protocol_version_matches_python_v1() {
        assert_eq!(XIAOBAI_PROTOCOL_VERSION, "AIS-FR13/V1.0");
    }
    #[test]
    fn ambiguity_threshold_0_10() {
        assert!((AMBIGUITY_THRESHOLD - 0.10_f32).abs() < 1e-6);
    }
    #[test]
    fn hotword_score_bounds_0_100() {
        assert!((HOTWORD_SCORE_MIN - 0.0_f32).abs() < 1e-6);
        assert!((HOTWORD_SCORE_MAX - 100.0_f32).abs() < 1e-6);
    }
    #[test]
    fn cloud_deadline_800ms() {
        assert_eq!(BRIDGE_CLOUD_DEADLINE_MS, 800);
    }
    #[test]
    fn s6_one_week_ms() {
        assert_eq!(S6_WEEKLY_CYCLE_MS, 604_800_000);
    }
}
