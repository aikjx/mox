// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 10 条 IAM 标准 Policy 定义
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
