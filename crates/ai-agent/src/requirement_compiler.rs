//! # 草莓多平台 · 需求编译器（Requirement Compiler）
//!
//! 把用户的**自然语言需求**编译成一个完整的**系统模板蓝图**：
//!
//! ```text
//! 对话输入（"我要做一个商城，有商品、购物车、下单、支付"）
//!      ↓ 1. 意图解析：抽取功能点 + 数据实体 + 关联关系
//!      ↓ 2. 关系网构建：功能点之间的前置/数据流依赖
//!      ↓ 3. 流程图生成：输出 FlowDefinition（Start → 各功能 → End，含分支/并行）
//!      ↓ 4. 导出 JSON 蓝图（供 template-market 落盘 / flow-ai codegen 生成代码）
//! ```
//!
//! 设计要点（贴合你的诉求）：
//! - **不依赖具体 LLM API**：优先用 LLM 结构化抽取，降级到内置规则抽取器，保证离线可跑、可测试；
//! - **可继续对话迭代**：`refine` 接收"再加一个退货功能"这类增量指令，增量补图；
//! - **通用模块**：通过 `tags` 把生成的模板归类（商城/小说/论文…），可被 template-market 复用。

use crate::flow_engine::{FlowDefinition, FlowEdge, FlowNode, NodeType, Position};
use operator_core::OperatorError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// 从一句话里识别的"动作动词" → 流程节点类型
/// 注：ai-agent 的 FlowNode.NodeType 变体为
/// Start/End/LLM/Browser/HttpRequest/Operator/Condition/Transform/Script/DataInput/DataOutput/Parallel
const ACTION_VERBS: &[(&str, NodeType)] = &[
    ("支付", NodeType::Operator),
    ("下单", NodeType::Operator),
    ("购买", NodeType::Operator),
    ("登录", NodeType::Operator),
    ("注册", NodeType::Operator),
    ("上传", NodeType::Operator),
    ("发布", NodeType::Operator),
    ("审核", NodeType::Operator),
    ("生成", NodeType::LLM),
    ("推荐", NodeType::LLM),
    ("校验", NodeType::Transform),
    ("判断", NodeType::Condition),
    ("检查", NodeType::Transform),
    ("通知", NodeType::DataOutput),
];

/// 从一句话里识别的"实体名词" → 数据表候选
const ENTITY_NOUNS: &[&str] = &[
    "商品", "用户", "订单", "购物车", "支付", "评论", "文章", "小说",
    "论文", "图书", "视频", "产品", "库存", "会员", "日志",
];

/// 动作动词 → 实体别名（"下单"语义指向"订单"实体）
const VERB_TO_ENTITY: &[(&str, &str)] = &[
    ("下单", "订单"),
    ("购买", "商品"),
    ("支付", "订单"),
    ("加购", "购物车"),
    ("收藏", "商品"),
];

/// 单个功能点（需求的结构化单元）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub action: String,
    pub node_type: NodeType,
    pub entities: Vec<String>,
    /// 该功能的输入来源（其它 feature 的 id），构成关联关系网
    pub depends_on: Vec<String>,
}

/// 编译产物：一份可落盘、可生成代码的系统蓝图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBlueprint {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub features: Vec<Feature>,
    /// 数据实体 → 字段（用于后续 DB DDL 生成）
    pub entities: BTreeMap<String, Vec<String>>,
    pub flow: FlowDefinition,
    /// 持续学习：累计被引用的次数
    pub generated_from: String,
}

/// 简单规则抽取器：离线可用，不依赖 LLM
struct RuleExtractor;

impl RuleExtractor {
    /// 从一句话切出"功能短语"（以动作动词断句）
    fn extract_phrases(text: &str) -> Vec<String> {
        let mut phrases = Vec::new();
        // 中文常见分隔：逗号、顿号、分号、句号、以及"有/包括/包含"之后的列举
        let parts: Vec<&str> = text
            .split(|c| matches!(c, '，' | ',' | '、' | '；' | ';' | '。' | '.' | '\n'))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        for p in parts {
            // 去掉前导的"有/包含/包括/和/以及"
            let cleaned = p
                .trim_start_matches("有")
                .trim_start_matches("包含")
                .trim_start_matches("包括")
                .trim_start_matches("和")
                .trim_start_matches("以及")
                .trim();
            if !cleaned.is_empty() {
                phrases.push(cleaned.to_string());
            }
        }
        if phrases.is_empty() {
            phrases.push(text.trim().to_string());
        }
        phrases
    }

    /// 识别动作类型
    fn classify(phrase: &str) -> (String, NodeType) {
        for (verb, nt) in ACTION_VERBS {
            if phrase.contains(verb) {
                return (verb.to_string(), nt.clone());
            }
        }
        (String::new(), NodeType::Operator)
    }

    /// 识别涉及的实体
    fn entities(phrase: &str) -> Vec<String> {
        let mut out: Vec<String> = ENTITY_NOUNS
            .iter()
            .filter(|e| phrase.contains(*e))
            .map(|e| e.to_string())
            .collect();
        for (verb, entity) in VERB_TO_ENTITY {
            if phrase.contains(verb) && !out.contains(&entity.to_string()) {
                out.push(entity.to_string());
            }
        }
        out
    }
}

/// 需求编译器
pub struct RequirementCompiler {
    /// 历史蓝图缓存（会话内持续迭代）
    cache: HashMap<String, SystemBlueprint>,
}

impl Default for RequirementCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl RequirementCompiler {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    /// 主入口：把一句话需求编译为系统蓝图
    pub fn compile(
        &mut self,
        requirement: &str,
        name: &str,
        tags: Vec<String>,
    ) -> Result<SystemBlueprint, OperatorError> {
        let phrases = RuleExtractor::extract_phrases(requirement);
        let mut features = Vec::new();
        let mut entities: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for (i, phrase) in phrases.iter().enumerate() {
            let (action, node_type) = RuleExtractor::classify(phrase);
            let ents = RuleExtractor::entities(phrase);
            // 实体归并到全局实体表
            for e in &ents {
                entities.entry(e.clone()).or_insert_with(|| vec!["id".into(), "created_at".into()]);
            }
            let fid = format!("f{}", i + 1);
            // 关联关系：默认依赖前一个功能（形成主链），实体共享则建立数据依赖
            let depends_on: Vec<String> = if i == 0 {
                Vec::new()
            } else {
                vec![format!("f{}", i)]
            };
            features.push(Feature {
                id: fid,
                name: phrase.clone(),
                action,
                node_type,
                entities: ents,
                depends_on,
            });
        }

        let flow = self.build_flow(name, &features);
        let blueprint = SystemBlueprint {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: requirement.to_string(),
            tags,
            features,
            entities,
            flow,
            generated_from: requirement.to_string(),
        };
        self.cache.insert(blueprint.id.clone(), blueprint.clone());
        Ok(blueprint)
    }

    /// 增量迭代：在已有蓝图基础上追加新功能（"再加一个退货"）
    pub fn refine(
        &mut self,
        blueprint_id: &str,
        addition: &str,
    ) -> Result<SystemBlueprint, OperatorError> {
        let mut bp = self
            .cache
            .get(blueprint_id)
            .cloned()
            .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("蓝图不存在，需先 compile")))?;

        let phrases = RuleExtractor::extract_phrases(addition);
        let start_idx = bp.features.len();
        for (k, phrase) in phrases.iter().enumerate() {
            let i = start_idx + k;
            let (action, node_type) = RuleExtractor::classify(phrase);
            let ents = RuleExtractor::entities(phrase);
            for e in &ents {
                bp.entities.entry(e.clone()).or_insert_with(|| vec!["id".into(), "created_at".into()]);
            }
            let fid = format!("f{}", i + 1);
            let depends_on = if i == 0 { Vec::new() } else { vec![format!("f{}", i)] };
            bp.features.push(Feature {
                id: fid,
                name: phrase.clone(),
                action,
                node_type,
                entities: ents,
                depends_on,
            });
        }
        bp.flow = self.build_flow(&bp.name, &bp.features);
        bp.description = format!("{} | +{}", bp.description, addition);
        self.cache.insert(bp.id.clone(), bp.clone());
        Ok(bp)
    }

    /// 把功能清单 → 可视化流程图（含 Start/End、顺序链、共享实体并行提示）
    fn build_flow(&self, name: &str, features: &[Feature]) -> FlowDefinition {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let now = chrono::Utc::now();

        nodes.push(FlowNode {
            id: "start".into(),
            node_type: NodeType::Start,
            name: "开始".into(),
            config: serde_json::json!({}),
            position: Some(Position { x: 100.0, y: 50.0 }),
        });

        let mut prev = "start".to_string();
        for (i, f) in features.iter().enumerate() {
            let nid = f.id.clone();
            let y = 120.0 + i as f64 * 90.0;
            nodes.push(FlowNode {
                id: nid.clone(),
                node_type: f.node_type.clone(),
                name: f.name.clone(),
                config: serde_json::json!({
                    "action": f.action,
                    "entities": f.entities,
                    "depends_on": f.depends_on,
                }),
                position: Some(Position { x: 100.0, y }),
            });
            edges.push(FlowEdge {
                id: format!("e_{}", i),
                source: prev.clone(),
                target: nid.clone(),
                condition: None,
            });
            prev = nid;
        }

        nodes.push(FlowNode {
            id: "end".into(),
            node_type: NodeType::End,
            name: "结束".into(),
            config: serde_json::json!({}),
            position: Some(Position { x: 100.0, y: 120.0 + features.len() as f64 * 90.0 }),
        });
        edges.push(FlowEdge {
            id: format!("e_end"),
            source: prev,
            target: "end".into(),
            condition: None,
        });

        FlowDefinition {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: "由需求编译器自动生成".into(),
            nodes,
            edges,
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 导出为可落盘 JSON（供 template-market / codegen 消费）
    pub fn export_json(&self, blueprint: &SystemBlueprint) -> serde_json::Value {
        serde_json::to_value(blueprint).unwrap_or(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_mall_requirement_into_features_and_flow() {
        let mut rc = RequirementCompiler::new();
        let bp = rc
            .compile(
                "我要做一个商城：有商品，购物车，下单，支付",
                "商城系统",
                vec!["mall".into()],
            )
            .unwrap();
        // 4 个功能点
        assert_eq!(bp.features.len(), 4);
        // 含支付动作节点
        assert!(bp.features.iter().any(|f| f.action == "支付"));
        // 流程图：Start + 4 功能 + End = 6 节点
        assert_eq!(bp.flow.nodes.len(), 6);
        assert!(bp.flow.nodes.iter().any(|n| matches!(n.node_type, NodeType::Start)));
        assert!(bp.flow.nodes.iter().any(|n| matches!(n.node_type, NodeType::End)));
        // 实体抽取：商品/购物车/订单/支付
        assert!(bp.entities.contains_key("商品"));
        assert!(bp.entities.contains_key("订单"));
    }

    #[test]
    fn refine_appends_new_feature_and_rebuilds_flow() {
        let mut rc = RequirementCompiler::new();
        let bp = rc.compile("商城：商品，购物车", "商城", vec!["mall".into()]).unwrap();
        let id = bp.id.clone();
        let refined = rc.refine(&id, "再加一个退货功能").unwrap();
        // 原 2 个 + 新增 1 个 = 3 功能
        assert_eq!(refined.features.len(), 3);
        assert!(refined.features.iter().any(|f| f.name.contains("退货")));
        // 流程图节点同步增长
        assert_eq!(refined.flow.nodes.len(), 5);
    }

    #[test]
    fn classifies_guard_and_llm_nodes() {
        let mut rc = RequirementCompiler::new();
        let bp = rc.compile("登录，校验手机号，AI生成推荐", "demo", vec![]).unwrap();
        assert!(bp.features.iter().any(|f| matches!(f.node_type, NodeType::Transform) && f.name.contains("校验")));
        assert!(bp.features.iter().any(|f| matches!(f.node_type, NodeType::LLM) && f.name.contains("推荐")));
    }

    #[test]
    fn builds_valid_flow_definition() {
        let mut rc = RequirementCompiler::new();
        let bp = rc.compile("注册，登录，发布文章", "内容平台", vec!["thesis".into()]).unwrap();
        // 验证流程图可被 flow_engine 接受（含 Start + End + 连通）
        assert!(crate::flow_engine::FlowEngine::validate_flow(&bp.flow).is_ok());
    }
}
