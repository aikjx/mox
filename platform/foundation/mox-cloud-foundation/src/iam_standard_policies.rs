// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! T10 M4：10 条 IAM 标准 Policy + Deny 优先 evaluate 引擎
//!
//! 10 条 Policy SID 命名（与 tasks.md A-2 对齐）：
//!
//! | # | SID                | 适用场景                                  |
//! |---|--------------------|------------------------------------------|
//! | P1 | AdminFullAccess    | 管理员 所有 Action / 所有 Resource        |
//! | P2 | BucketOwnerFull    | 桶所有者：对自己桶的全操作                |
//! | P3 | EditorWrite        | 编辑者：写 + 读 + 列（不含权限变更）       |
//! | P4 | ViewerReadOnly     | 只读：Get/Head/List                      |
//! | P5 | GuestListOnly      | 访客：仅列桶                              |
//! | P6 | PublicRead         | 公开匿名：GetObject public-read 前缀      |
//! | P7 | DenyNonMFADelete   | 未通过 MFA → 拒绝所有 Delete 类动作       |
//! | P8 | DenyIPOutOfRange   | 非允许 IP 段 → 拒绝所有                   |
//! | P9 | TagConditionalEdit | 仅 tag=project:X 下允许写                 |
//! | P10| VPCSourceOnly      | 非 VPC/内网来源 IP → 拒绝                 |

use crate::iam::PolicyStatement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 条件键（用于 P7/P8/P9/P10）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalContext {
    pub principal: String,
    pub action: String,
    pub resource: String,
    /// 桶所有者 (用于 P2 BucketOwnerFull 隐式授权)
    pub bucket_owner: Option<String>,
    /// MFA 是否已通过
    pub mfa_authenticated: Option<bool>,
    /// 源 IP
    pub source_ip: Option<String>,
    /// 请求携带标签（k→v）
    pub tags: BTreeMap<String, String>,
    /// 是否来自 VPC / 内网
    pub from_vpc: Option<bool>,
}

/// P1..P10 标准 10 条（sid 命名与注释严格对齐 tasks.md 清单）
pub fn standard_10_policies() -> Vec<PolicyStatement> {
    vec![
        // P1 AdminFullAccess
        PolicyStatement {
            sid: "P1-AdminFullAccess".into(),
            effect: "Allow".into(),
            actions: vec!["*".into()],
            resources: vec!["*".into()],
        },
        // P2 BucketOwnerFull — 资源端以桶为粒度匹配
        PolicyStatement {
            sid: "P2-BucketOwnerFull".into(),
            effect: "Allow".into(),
            actions: vec![
                "s3:*".into(),
                "cloud:*".into(),
            ],
            resources: vec!["arn:cloud:::bucket/OWNER_PREFIX/*".into()],
        },
        // P3 EditorWrite
        PolicyStatement {
            sid: "P3-EditorWrite".into(),
            effect: "Allow".into(),
            actions: vec![
                "s3:PutObject".into(),
                "s3:GetObject".into(),
                "s3:DeleteObject".into(),
                "s3:ListBucket".into(),
                "s3:HeadObject".into(),
                "cloud:Upload".into(),
                "cloud:Download".into(),
            ],
            resources: vec!["arn:cloud:::bucket/*".into()],
        },
        // P4 ViewerReadOnly
        PolicyStatement {
            sid: "P4-ViewerReadOnly".into(),
            effect: "Allow".into(),
            actions: vec![
                "s3:GetObject".into(),
                "s3:ListBucket".into(),
                "s3:HeadObject".into(),
                "cloud:Download".into(),
                "cloud:List".into(),
            ],
            resources: vec!["arn:cloud:::bucket/*".into()],
        },
        // P5 GuestListOnly
        PolicyStatement {
            sid: "P5-GuestListOnly".into(),
            effect: "Allow".into(),
            actions: vec!["s3:ListBucket".into(), "cloud:List".into()],
            resources: vec!["arn:cloud:::bucket/*".into()],
        },
        // P6 PublicRead（匿名）
        PolicyStatement {
            sid: "P6-PublicRead".into(),
            effect: "Allow".into(),
            actions: vec!["s3:GetObject".into(), "cloud:Download".into()],
            resources: vec!["arn:cloud:::bucket/*/public/*".into()],
        },
        // P7 DenyNonMFA：非 MFA 情况下拒绝所有 Delete*
        PolicyStatement {
            sid: "P7-DenyNonMFADelete".into(),
            effect: "Deny".into(),
            actions: vec![
                "s3:DeleteObject".into(),
                "s3:DeleteBucket".into(),
                "s3:DeleteBucketPolicy".into(),
                "cloud:Delete".into(),
            ],
            resources: vec!["*".into()],
        },
        // P8 DenyIPOutOfRange：非允许 IP 段 → 所有 Deny（实际匹配逻辑在 evaluate 条件层）
        PolicyStatement {
            sid: "P8-DenyIPOutOfRange".into(),
            effect: "Deny".into(),
            actions: vec!["*".into()],
            resources: vec!["*".into()],
        },
        // P9 TagConditionalEdit：仅 tag project:alpha 可写
        PolicyStatement {
            sid: "P9-TagConditionalEdit".into(),
            effect: "Allow".into(),
            actions: vec![
                "s3:PutObject".into(),
                "cloud:Upload".into(),
            ],
            resources: vec!["arn:cloud:::bucket/project-alpha/*".into()],
        },
        // P10 VPCSourceOnly：Deny 非 VPC 来源（evaluate 层处理条件）
        PolicyStatement {
            sid: "P10-VPCSourceOnly".into(),
            effect: "Deny".into(),
            actions: vec!["*".into()],
            resources: vec!["*".into()],
        },
    ]
}

/// 便于索引的常量视图
pub const STANDARD_10_SIDS: [&str; 10] = [
    "P1-AdminFullAccess",
    "P2-BucketOwnerFull",
    "P3-EditorWrite",
    "P4-ViewerReadOnly",
    "P5-GuestListOnly",
    "P6-PublicRead",
    "P7-DenyNonMFADelete",
    "P8-DenyIPOutOfRange",
    "P9-TagConditionalEdit",
    "P10-VPCSourceOnly",
];

/// Action / Resource 匹配（统一前缀通配 "*" 规则）
fn action_matches(pattern: &str, action: &str) -> bool {
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
fn glob_match(pattern: &str, text: &str) -> bool {
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

fn resource_matches(pattern: &str, resource: &str, ctx: &EvalContext) -> bool {
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
fn source_ip_in_trusted_range(ip: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(
        sid: &str,
        action: &str,
        resource: &str,
        extra: impl Fn(&mut EvalContext),
    ) -> bool {
        // 取单条 policy 做单元评估
        let all = standard_10_policies();
        let one = all.iter().find(|p| p.sid == sid).cloned().unwrap();
        let mut ctx = EvalContext {
            principal: "u".into(),
            action: action.into(),
            resource: resource.into(),
            ..Default::default()
        };
        extra(&mut ctx);
        evaluate_policies(&[one], &ctx)
    }

    // --- P1 AdminFullAccess ---
    #[test]
    fn p1_admin_allows_any() {
        assert!(eval("P1-AdminFullAccess", "anything", "anywhere", |_| {}));
    }

    // --- P2 BucketOwnerFull ---
    #[test]
    fn p2_owner_owns_my_bucket() {
        let r = eval(
            "P2-BucketOwnerFull",
            "s3:PutObject",
            "arn:cloud:::bucket/alice/secret.txt",
            |c| c.bucket_owner = Some("alice".into()),
        );
        assert!(r);
    }
    #[test]
    fn p2_not_owner_rejects() {
        let r = eval(
            "P2-BucketOwnerFull",
            "s3:PutObject",
            "arn:cloud:::bucket/bob/secret.txt",
            |c| c.bucket_owner = Some("alice".into()),
        );
        // P2 模式是 /OWNER_PREFIX/*，bob 桶不匹配，不命中 Allow → false
        assert!(!r);
    }

    // --- P3 EditorWrite ---
    #[test]
    fn p3_editor_write_allowed() {
        assert!(eval("P3-EditorWrite", "s3:PutObject", "arn:cloud:::bucket/x/a", |_| {}));
    }
    #[test]
    fn p3_editor_permission_action_mismatch() {
        assert!(!eval("P3-EditorWrite", "s3:PutBucketPolicy", "arn:cloud:::bucket/x", |_| {}));
    }

    // --- P4 ViewerRO ---
    #[test]
    fn p4_viewer_get_ok() {
        assert!(eval("P4-ViewerReadOnly", "s3:GetObject", "arn:cloud:::bucket/x/a", |_| {}));
    }
    #[test]
    fn p4_viewer_put_rejected() {
        assert!(!eval("P4-ViewerReadOnly", "s3:PutObject", "arn:cloud:::bucket/x/a", |_| {}));
    }

    // --- P5 GuestListOnly ---
    #[test]
    fn p5_list_ok() {
        assert!(eval("P5-GuestListOnly", "s3:ListBucket", "arn:cloud:::bucket/x", |_| {}));
    }
    #[test]
    fn p5_get_rejected() {
        assert!(!eval("P5-GuestListOnly", "s3:GetObject", "arn:cloud:::bucket/x/a", |_| {}));
    }

    // --- P6 PublicRead ---
    #[test]
    fn p6_public_prefix_match() {
        assert!(eval(
            "P6-PublicRead",
            "s3:GetObject",
            "arn:cloud:::bucket/x/public/logo.png",
            |_| {}
        ));
    }
    #[test]
    fn p6_private_not_match() {
        assert!(!eval(
            "P6-PublicRead",
            "s3:GetObject",
            "arn:cloud:::bucket/x/private/a.txt",
            |_| {}
        ));
    }

    // --- P7 DenyNonMFA ---
    #[test]
    fn p7_no_mfa_deny_delete() {
        let r = eval("P7-DenyNonMFADelete", "s3:DeleteObject", "arn:cloud:::bucket/x/a", |c| {
            c.mfa_authenticated = Some(false);
        });
        assert!(!r);
    }
    #[test]
    fn p7_mfa_true_skip_deny_allow_from_elsewhere() {
        // 单独 P7 本身不 Allow，所以单条 policy 下即使 MFA=true → 无 Allow → false
        let r = eval("P7-DenyNonMFADelete", "s3:DeleteObject", "arn:cloud:::bucket/x/a", |c| {
            c.mfa_authenticated = Some(true);
        });
        assert!(!r, "P7 only denies; without Allow → false");
        // P3 + P7 组合：MFA=true → Allow；MFA=false → Deny（Deny 优先）
        let all = standard_10_policies();
        let p3 = find_by_sid(&all, "P3-EditorWrite").unwrap().clone();
        let p7 = find_by_sid(&all, "P7-DenyNonMFADelete").unwrap().clone();
        let mut ctx = EvalContext {
            principal: "e".into(),
            action: "s3:DeleteObject".into(),
            resource: "arn:cloud:::bucket/x/a".into(),
            mfa_authenticated: Some(false),
            ..Default::default()
        };
        assert!(!evaluate_policies(&[p3.clone(), p7.clone()], &ctx));
        ctx.mfa_authenticated = Some(true);
        assert!(evaluate_policies(&[p3, p7], &ctx));
    }

    // --- P8 DenyIP ---
    #[test]
    fn p8_trusted_cidr_skip_deny() {
        let all = standard_10_policies();
        let p1 = find_by_sid(&all, "P1-AdminFullAccess").unwrap().clone();
        let p8 = find_by_sid(&all, "P8-DenyIPOutOfRange").unwrap().clone();
        let mut ctx = EvalContext {
            principal: "a".into(),
            action: "s3:GetObject".into(),
            resource: "arn:cloud:::b/k".into(),
            source_ip: Some("10.0.0.1".into()),
            ..Default::default()
        };
        assert!(evaluate_policies(&[p1.clone(), p8.clone()], &ctx));
        ctx.source_ip = Some("8.8.8.8".into());
        assert!(!evaluate_policies(&[p1, p8], &ctx));
    }

    // --- P9 TagConditionalEdit ---
    #[test]
    fn p9_tag_matched_allows_write() {
        let r = eval("P9-TagConditionalEdit", "s3:PutObject", "arn:cloud:::bucket/project-alpha/x", |c| {
            c.tags.insert("project".into(), "alpha".into());
        });
        assert!(r);
    }
    #[test]
    fn p9_tag_missing_blocks() {
        let r = eval("P9-TagConditionalEdit", "s3:PutObject", "arn:cloud:::bucket/project-alpha/x", |_| {});
        assert!(!r);
    }

    // --- P10 VPCOnly ---
    #[test]
    fn p10_vpc_true_skips_deny() {
        let all = standard_10_policies();
        let p1 = find_by_sid(&all, "P1-AdminFullAccess").unwrap().clone();
        let p10 = find_by_sid(&all, "P10-VPCSourceOnly").unwrap().clone();
        let mut ctx = EvalContext {
            principal: "s".into(),
            action: "x".into(),
            resource: "y".into(),
            from_vpc: Some(true),
            ..Default::default()
        };
        assert!(evaluate_policies(&[p1.clone(), p10.clone()], &ctx));
        ctx.from_vpc = Some(false);
        assert!(!evaluate_policies(&[p1, p10], &ctx));
    }

    // --- Deny 优先总体规则：Allow + Deny（无条件）同时命中 → Deny 赢 ---
    #[test]
    fn deny_overrides_allow() {
        let allow = PolicyStatement {
            sid: "AllowDelete".into(),
            effect: "Allow".into(),
            actions: vec!["s3:DeleteObject".into()],
            resources: vec!["*".into()],
        };
        let deny = PolicyStatement {
            sid: "NeverDelete".into(),
            effect: "Deny".into(),
            actions: vec!["s3:DeleteObject".into()],
            resources: vec!["*".into()],
        };
        let ctx = EvalContext {
            principal: "u".into(),
            action: "s3:DeleteObject".into(),
            resource: "arn:cloud:::b/k".into(),
            ..Default::default()
        };
        // 顺序不影响 Deny 优先（evaluate 先跑 Deny？不：我们按顺序遍历，但每条 Deny 命中立即 return false）
        assert!(!evaluate_policies(&[allow.clone(), deny.clone()], &ctx));
        assert!(!evaluate_policies(&[deny, allow.clone()], &ctx));
        // 单独 allow → true
        assert!(evaluate_policies(&[allow], &ctx));
    }

    // --- 隐含 Deny：完全不匹配任何 Allow → false ---
    #[test]
    fn implicit_deny_when_no_allow_matches() {
        let allow = PolicyStatement {
            sid: "OnlyList".into(),
            effect: "Allow".into(),
            actions: vec!["s3:ListBucket".into()],
            resources: vec!["*".into()],
        };
        let ctx = EvalContext {
            principal: "u".into(),
            action: "s3:PutObject".into(),
            resource: "x".into(),
            ..Default::default()
        };
        assert!(!evaluate_policies(&[allow], &ctx));
    }

    // --- Prefix 匹配验证 ---
    #[test]
    fn action_wildcard_prefix_matches() {
        let p = PolicyStatement {
            sid: "S3Star".into(),
            effect: "Allow".into(),
            actions: vec!["s3:*".into()],
            resources: vec!["*".into()],
        };
        for a in ["s3:PutObject", "s3:GetObject", "s3:ListBucketVersions"] {
            let ctx = EvalContext {
                principal: "u".into(),
                action: a.into(),
                resource: "x".into(),
                ..Default::default()
            };
            assert!(evaluate_policies(&[p.clone()], &ctx), "fail: {a}");
        }
    }

    #[test]
    fn resource_wildcard_prefix_matches() {
        let p = PolicyStatement {
            sid: "R".into(),
            effect: "Allow".into(),
            actions: vec!["*".into()],
            resources: vec!["arn:cloud:::bucket/shared/*".into()],
        };
        let ctx1 = EvalContext {
            principal: "u".into(),
            action: "a".into(),
            resource: "arn:cloud:::bucket/shared/docs/readme.md".into(),
            ..Default::default()
        };
        assert!(evaluate_policies(&[p.clone()], &ctx1));
        let ctx2 = EvalContext {
            principal: "u".into(),
            action: "a".into(),
            resource: "arn:cloud:::bucket/personal/a".into(),
            ..Default::default()
        };
        assert!(!evaluate_policies(&[p], &ctx2));
    }
}
