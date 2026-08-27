// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Bucket Policy JSON 解析 + evaluate（Principal/Effect/Action/Resource 核心子集）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BucketPolicyStatement {
    #[serde(rename = "Sid", default)]
    pub sid: String,
    #[serde(rename = "Effect")]
    pub effect: String, // "Allow" / "Deny"
    #[serde(rename = "Principal")]
    pub principal: PrincipalValue,
    #[serde(rename = "Action")]
    pub action: StringListOrOne,
    #[serde(rename = "Resource")]
    pub resource: StringListOrOne,
    #[serde(rename = "Condition", default)]
    pub condition: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(untagged)]
pub enum PrincipalValue {
    #[default]
    Empty,
    String(String),
    Map(HashMap<String, StringListOrOne>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(untagged)]
pub enum StringListOrOne {
    #[default]
    None,
    One(String),
    List(Vec<String>),
}

impl StringListOrOne {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            StringListOrOne::None => Vec::new(),
            StringListOrOne::One(s) => vec![s.clone()],
            StringListOrOne::List(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BucketPolicy {
    #[serde(rename = "Version", default)]
    pub version: String,
    #[serde(rename = "Statement")]
    pub statement: Vec<BucketPolicyStatement>,
}

impl BucketPolicy {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// 评估 policy：给定 principal_id/action/resource → 返回 true(allow) / false(implicit deny)。
    /// Deny 优先于 Allow（AWS 评估逻辑核心子集）。
    pub fn evaluate(&self, principal_id: &str, action: &str, resource: &str) -> bool {
        let mut allowed = false;
        for st in &self.statement {
            // Principal 匹配
            let p_match = match &st.principal {
                PrincipalValue::Empty => false,
                PrincipalValue::String(s) => s == "*" || s == principal_id,
                PrincipalValue::Map(m) => {
                    // AWS: {"AWS": "..."} 或 {"CanonicalUser": "..."}
                    m.values().any(|lv| {
                        let lv_vec = lv.to_vec();
                        lv_vec.iter().any(|v| v == "*" || v == principal_id)
                    })
                }
            };
            if !p_match {
                continue;
            }

            let actions = st.action.to_vec();
            let resources = st.resource.to_vec();
            let a_match = actions.iter().any(|a| {
                if a == "*" {
                    return true;
                }
                // 支持 s3:GetObject / s3:* 匹配
                let prefix = a.trim_end_matches('*');
                action == a || (a.ends_with('*') && action.starts_with(prefix))
            });
            let r_match = resources.iter().any(|r| {
                if r == "*" {
                    return true;
                }
                // arn:aws:s3:::bucket/* 匹配
                let prefix = r.trim_end_matches('*');
                resource == r || (r.ends_with('*') && resource.starts_with(prefix))
            });
            if a_match && r_match {
                if st.effect == "Deny" {
                    return false;
                }
                if st.effect == "Allow" {
                    allowed = true;
                }
            }
        }
        allowed
    }
}
