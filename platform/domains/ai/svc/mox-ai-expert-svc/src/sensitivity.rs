// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 敏感度判定单一权威源（SSOT）
//!
//! 根治 P1（原 `permission.rs` / `security.rs` 对敏感/生产/脱敏判定三处分叉，
//! 导致 `var:citizen_safe` 这类已脱敏资源被 `contains("citizen")` 误判为泄露）。
//!
//! 设计要点：
//! - 所有"是否敏感 / 是否生产 / 是否已脱敏"判定统一收口于此，专家层只调用本模块。
//! - 判定基于**规范化资源 URI** `<scheme>:<env>/<domain>/<entity>`（对齐 IN-5）。
//!   - 敏感域：`citizen_*` / `pii` / `id_card` / `phone` / `bank_card`（含脱敏后缀 `_safe` 视作已脱敏）。
//!   - 生产环境前缀：`prod` / `production` / `main`。
//! - 关键修正：凡资源名以 `_safe` / `_desensitized` / `_masked` 结尾，视为**已脱敏**，
//!   不再判定为敏感泄露（消除假阳性阻断）。

/// 敏感数据域关键词（子串匹配，作用于 scheme+body 全串）
/// 注意：用 `citizen_`（带下划线）而非裸 `citizen`，避免变量名 `var:citizen`
/// 被误判为敏感域（仅 `db:citizen_info` 等真实公民库命中）。
const SENSITIVE_DOMAINS: &[&str] = &["citizen_", "pii", "id_card", "phone", "bank_card"];

/// 生产环境标识（匹配 `<scheme>:<env>/...` 中的 env 段前缀）
const PRODUCTION_ENVS: &[&str] = &["prod", "production", "main"];

/// 已脱敏资源后缀（命中即视为安全，不再判为敏感泄露）
const DESENSITIZED_SUFFIXES: &[&str] = &["_safe", "_desensitized", "_masked", "_anon"];

/// 判断单个资源 URI 是否**已脱敏**（安全）。
///
/// 规则：资源 URI 去除 scheme 后，若末段（entity）以脱敏后缀结尾，判定为已脱敏。
pub fn is_desensitized(resource: &str) -> bool {
    let entity = resource.rsplit('/').next().unwrap_or(resource);
    DESENSITIZED_SUFFIXES.iter().any(|s| entity.ends_with(s))
}

/// 判断单个资源 URI 是否触碰**敏感数据域**（无论是否已脱敏）。
///
/// 注意：本函数只判"是否敏感域"，**不**判"是否已脱敏"。
/// 调用方需结合 [`is_desensitized`] 决定是否真正构成泄露风险。
/// 检测覆盖 scheme 与 body 全串（如 `pii:user_data` 的 scheme 本身即敏感标识）。
pub fn is_sensitive_domain(resource: &str) -> bool {
    let lowered = resource.to_lowercase();
    SENSITIVE_DOMAINS.iter().any(|d| lowered.contains(d))
}

/// 判断单个资源 URI 是否位于**生产环境**（env 段为 prod/production/main）。
pub fn is_production(resource: &str) -> bool {
    // 形如 scheme:env/domain/entity —— 取 scheme 后的第一段作为 env
    let after_scheme = match resource.split_once(':') {
        Some((_, r)) => r,
        None => return false,
    };
    let env = after_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .trim_matches(':');
    PRODUCTION_ENVS
        .iter()
        .any(|p| env == *p || env.starts_with(p))
}

/// 判断资源是否构成**真实敏感泄露风险**：敏感域 且 未脱敏。
///
/// 这是 `permission` / `security` 专家应使用的**唯一**泄露判定入口，
/// 彻底消除原三处分叉带来的 `var:citizen_safe` 假阳性。
pub fn is_sensitive_leak(resource: &str) -> bool {
    is_sensitive_domain(resource) && !is_desensitized(resource)
}

/// 判断资源是否构成**生产/敏感写风险**：生产环境 或 敏感域（未脱敏）的写操作。
///
/// 用于 `permission` 专家区分"生产/敏感写（否决级）"与"普通外部写（建议级）"。
pub fn is_production_or_sensitive_write(resource: &str) -> bool {
    is_production(resource) || is_sensitive_leak(resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desensitized_not_flagged_as_leak() {
        // 根治 P1 的核心用例：已脱敏变量不得被误判泄露
        assert!(is_desensitized("var:citizen_safe"));
        assert!(is_desensitized("db:prod/citizen_info_desensitized"));
        assert!(!is_sensitive_leak("var:citizen_safe"));
        assert!(!is_sensitive_leak("pii:user_data_masked"));
    }

    #[test]
    fn raw_sensitive_flagged_as_leak() {
        assert!(is_sensitive_leak("db:citizen_info"));
        assert!(is_sensitive_leak("pii:user_data"));
        assert!(is_sensitive_leak("var:id_card"));
    }

    #[test]
    fn production_env_detection() {
        assert!(is_production("db:prod/orders"));
        assert!(is_production("db:production/orders"));
        assert!(is_production("db:main/anything"));
        assert!(!is_production("db:test/orders"));
        assert!(!is_production("db:dev/citizen_info"));
    }

    #[test]
    fn production_or_sensitive_write() {
        // 生产环境即便非敏感域也触发
        assert!(is_production_or_sensitive_write("db:prod/orders"));
        // 敏感域未脱敏触发
        assert!(is_production_or_sensitive_write("db:citizen_info"));
        // 测试环境 + 已脱敏变量不触发
        assert!(!is_production_or_sensitive_write("db:test/citizen_safe"));
    }
}
