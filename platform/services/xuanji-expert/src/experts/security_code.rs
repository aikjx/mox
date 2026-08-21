//! 代码安全专家（开发维度）：审查代码安全漏洞、注入风险、敏感数据处理
//!
//! 分析基于 `CodeUnit` 的**预分析真字段**（`hardcoded_secret` / `sql_injection_risk` /
//! `weak_hash` / `n_plus_one`），不再做脆弱的字符串子串匹配。
//! 安全问题一律 `veto=true`（强制否决，治理闸门不可覆盖）。

use crate::expert::{Expert, ExpertOpinion, Risk};
use crate::ir::{Dimension, ExpertId};
use crate::context::ExpertContext;
use flow_ai::model::Severity;

/// 代码安全专家：审查代码层面的安全问题
pub struct SecurityCodeExpert;

impl Expert for SecurityCodeExpert {
    fn id(&self) -> ExpertId {
        "security_code".to_string()
    }

    fn dimension(&self) -> Dimension {
        Dimension::SecurityCode
    }

    fn analyze(&self, ctx: &ExpertContext) -> ExpertOpinion {
        let mut risks = Vec::new();
        let mut score = 1.0;

        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 硬编码密钥/令牌（预分析字段，强否决）
                if unit.hardcoded_secret {
                    risks.push(Risk {
                        severity: Severity::Blocking,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 包含硬编码密钥/令牌", unit.name),
                        remediation: Some("使用环境变量或密钥管理服务".to_string()),
                        veto: true,
                    });
                    score *= 0.2;
                }

                // 2. SQL 注入风险（拼接 SQL，预分析字段，强否决）
                if unit.sql_injection_risk {
                    risks.push(Risk {
                        severity: Severity::Blocking,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 存在 SQL 注入风险（拼接 SQL）", unit.name),
                        remediation: Some("使用参数化查询/预编译语句".to_string()),
                        veto: true,
                    });
                    score *= 0.3;
                }

                // 3. 弱哈希（md5/sha1 用于密码，强否决）
                if unit.weak_hash {
                    risks.push(Risk {
                        severity: Severity::Blocking,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 使用弱哈希（md5/sha1）处理密码", unit.name),
                        remediation: Some("改用 bcrypt/argon2 等自适应哈希".to_string()),
                        veto: true,
                    });
                    score *= 0.3;
                }

                // 4. N+1 查询（性能+潜在资源耗尽，警告级）
                if unit.n_plus_one {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 存在 N+1 查询风险", unit.name),
                        remediation: Some("批量查询/预加载关联数据".to_string()),
                        veto: false,
                    });
                    score *= 0.7;
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::SecurityCode,
            constraints: Vec::new(),
            risks,
            score,
            metrics: Default::default(),
            suggestions: Vec::new(),
            skipped: false,
            skip_reason: None,
        }
    }
}
