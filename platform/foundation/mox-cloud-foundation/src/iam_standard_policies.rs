// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! T10 M4：10 条 IAM 标准 Policy + Deny 优先 evaluate 引擎

mod types;
mod policies;
mod evaluator;

pub use types::EvalContext;
pub use policies::{standard_10_policies, STANDARD_10_SIDS};
pub use evaluator::{evaluate_policies, find_by_sid};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iam::PolicyStatement;

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
