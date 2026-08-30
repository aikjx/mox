// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Deny 优先策略评估引擎

use crate::iam::PolicyStatement;
use super::types::EvalContext;

/// Action / Resource 匹配（统一前缀通配 "*" 规则）
pub(crate) fn action_matches(pattern: &str, action: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    // 支持通配 s3:* 匹配 s3:PutObject
    if let Some(prefix) = pattern.strip_suffix('*') {
        return action.starts_with(prefix);
    }
    pattern == action
}

/// 资源路径 glob 匹配（IAM 语义）：'*' 跨段匹配任意字符（含 '/'）。
/// 例：
///   "arn:cloud:::bucket/*" 匹配 "arn:cloud:::bucket/x/a"
///   "arn:cloud:::bucket/*/public/*" 匹配 "arn:cloud:::bucket/x/public/logo.png"
///   "arn:cloud:::bucket/*/public/*" 不匹配 "arn:cloud:::bucket/x/private/a.txt"
pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (pn, tn) = (p.len(), t.len());
    let mut pi = 0;
    let mut ti = 0;
    let mut star_p: Option<usize> = None;
    let mut star_t: Option<usize> = None;
    while ti < tn {
        if pi < pn && p[pi] == '*' {
            star_p = Some(pi);
            pi += 1;
            star_t = Some(ti);
        } else if pi < pn && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if star_p.is_some() {
            // 有 * 未闭合 → 回溯让 * 再吞一个（含 '/' 跨段，AWS IAM 语义）
            let sp = star_p.unwrap();
            pi = sp + 1;
            star_t = star_t.map(|s| s + 1);
            ti = star_t.unwrap_or(ti);
        } else {
            return false;
        }
    }
    while pi < pn && p[pi] == '*' {
        pi += 1;
    }
    pi == pn
}

pub(crate) fn resource_matches(pattern: &str, resource: &str, ctx: &EvalContext) -> bool {
    if pattern == "*" {
        return true;
    }
    // P2 OWNER_PREFIX 展开
    let real = if let Some(owner) = ctx.bucket_owner.as_deref() {
        pattern.replace("OWNER_PREFIX", owner)
    } else {
        pattern.to_string()
    };
    if real.contains('*') {
        return glob_match(&real, resource);
    }
    real == resource
}

/// 判定 P8：源 IP 是否在允许网段（简单实现：CIDR 前缀匹配 10.0./192.168./172.{16-31}.）
pub(crate) fn source_ip_in_trusted_range(ip: &str) -> bool {
    if ip.starts_with("10.") || ip.starts_with("192.168.") {
        return true;
    }
    if let Some(rest) = ip.strip_prefix("172.") {
        if let Some(o2) = rest.split('.').next() {
            if let Ok(n) = o2.parse::<u8>() {
                return (16..=31).contains(&n);
            }
        }
    }
    false
}

/// Deny 优先策略评估器：
///
/// 1. 先跑所有 Deny 规则：命中且上下文条件匹配 → 立即 false（Deny 短路）
/// 2. 再跑所有 Allow 规则：命中且条件匹配 → 累计 allow=true
/// 3. 最终未显式 Allow → false（隐含 Deny）
pub fn evaluate_policies(
    policies: &[PolicyStatement],
    ctx: &EvalContext,
) -> bool {
    let mut explicitly_allowed = false;
    for p in policies {
        let a_match = p.actions.iter().any(|a| action_matches(a, &ctx.action));
        let r_match = resource_matches(&p.resources.join(";"), &ctx.resource, ctx)
            || p.resources.iter().any(|r| resource_matches(r, &ctx.resource, ctx));
        if !(a_match && r_match) {
            continue;
        }
        let effect = p.effect.as_str();
        match effect {
            "Deny" => {
                // 条件门：P7/P8/P10 只在对应上下文成立时 Deny
                match p.sid.as_str() {
                    "P7-DenyNonMFADelete" => {
                        // MFA=false 或 None → Deny；MFA=true → 跳过
                        if !ctx.mfa_authenticated.unwrap_or(false) {
                            return false;
                        }
                        continue; // MFA OK，不触发这条 Deny
                    }
                    "P8-DenyIPOutOfRange" => match ctx.source_ip.as_deref() {
                        Some(ip) if !source_ip_in_trusted_range(ip) => return false,
                        None => return false, // 没 IP → 视为不可信
                        _ => continue,
                    },
                    "P10-VPCSourceOnly" => match ctx.from_vpc {
                        Some(true) => continue,
                        _ => return false,
                    },
                    _ => {
                        // 普通 Deny 无特殊条件，直接短路
                        return false;
                    }
                }
            }
            "Allow" => {
                // Allow 侧条件：P9 需要 tag project=alpha
                if p.sid == "P9-TagConditionalEdit" {
                    match ctx.tags.get("project") {
                        Some(v) if v == "alpha" => {}
                        _ => continue, // 不满足，跳过这条 Allow
                    }
                }
                explicitly_allowed = true;
            }
            _ => {
                // 未知 effect → 视为 Deny（保守）
                return false;
            }
        }
    }
    explicitly_allowed
}

/// 便捷：按 SID 过滤 policy
pub fn find_by_sid<'a>(policies: &'a [PolicyStatement], sid: &str) -> Option<&'a PolicyStatement> {
    policies.iter().find(|p| p.sid == sid)
}
