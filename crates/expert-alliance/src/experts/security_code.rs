//! 代码安全专家（开发维度）：审查代码安全漏洞、注入风险、敏感数据处理

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
        let constraints = Vec::new();
        let mut risks = Vec::new();
        let suggestions = Vec::new();
        let mut score = 1.0;

        // 分析代码IR
        if let Some(code_ir) = &ctx.code_ir {
            for unit in &code_ir.units {
                // 1. 检查SQL注入风险
                if contains_sql_injection_pattern(&unit.source_code) {
                    risks.push(Risk {
                        severity: Severity::Blocking,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 存在SQL注入风险", unit.name),
                        remediation: Some("使用参数化查询".to_string()),
                        veto: true, // 安全问题强制否决
                    });
                    score *= 0.3;
                }

                // 2. 检查XSS风险
                if contains_xss_pattern(&unit.source_code) {
                    risks.push(Risk {
                        severity: Severity::Warning,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 存在XSS风险", unit.name),
                        remediation: Some("对用户输入进行转义".to_string()),
                        veto: true,
                    });
                    score *= 0.5;
                }

                // 3. 检查硬编码密钥
                if contains_hardcoded_secrets(&unit.source_code) {
                    risks.push(Risk {
                        severity: Severity::Blocking,
                        nodes: vec![unit.id.clone()],
                        dimension: Dimension::SecurityCode,
                        message: format!("模块 {} 包含硬编码密钥", unit.name),
                        remediation: Some("使用环境变量或密钥管理服务".to_string()),
                        veto: true,
                    });
                    score *= 0.2;
                }

                // 4. 检查不安全依赖
                for dep in &unit.dependencies {
                    if is_unsafe_dependency(&dep) {
                        risks.push(Risk {
                            severity: Severity::Warning,
                            nodes: vec![unit.id.clone()],
                            dimension: Dimension::SecurityCode,
                            message: format!("依赖 {} 存在已知漏洞，建议升级", dep),
                            remediation: Some("更新到安全版本".to_string()),
                            veto: false,
                        });
                        score *= 0.7;
                    }
                }
            }
        }

        ExpertOpinion {
            expert: self.id(),
            dimension: Dimension::SecurityCode,
            constraints,
            risks,
            score,
            metrics: Default::default(),
            suggestions,
            skipped: false,
            skip_reason: None,
        }
    }
}

/// 检查SQL注入模式
fn contains_sql_injection_pattern(code: &str) -> bool {
    code.contains("format!") && code.contains("SELECT")
        || code.contains("execute(") && code.contains("+")
}

/// 检查XSS模式
fn contains_xss_pattern(code: &str) -> bool {
    code.contains("innerHTML") && !code.contains("sanitize")
        || code.contains("document.write")
}

/// 检查硬编码密钥
fn contains_hardcoded_secrets(code: &str) -> bool {
    code.contains("password = \"")
        || code.contains("api_key = \"")
        || code.contains("secret = \"")
}

/// 检查不安全依赖
fn is_unsafe_dependency(dep: &str) -> bool {
    dep.contains("openssl-0.9") || dep.contains("serde_yaml-0.8")
}
