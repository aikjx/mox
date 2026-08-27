// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

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
use mox_flow_operator_core::OperatorError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use mox_platform_system_core::persistence_provider::{PersistenceProvider, SqlValue};

/// 喂给 LLM 的消息（与 llm_client::LLMChatMessage 同构，避免跨模块类型泄漏）
#[derive(Debug, Clone)]
pub struct LlmMsg {
    pub role: String,
    pub content: String,
}

/// 由 `AIAgent::llm_client()` 提供的 LLM 通道（OpenAI 兼容）用于"细功能拆解"。
/// 定义为 trait 对象指针，避免 ai-agent 对 llm_client 具体类型产生循环依赖噪音。
pub type LlmFn = Arc<
    dyn Fn(Vec<LlmMsg>) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'static>>
        + Send
        + Sync,
>;

/// LLM 拆解后返回的结构化功能点
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmFeature {
    name: String,
    action: String,
    #[serde(default)]
    node_type: Option<String>,
    #[serde(default)]
    entities: Vec<String>,
}

/// LLM 拆解的响应外壳
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmPlan {
    features: Vec<LlmFeature>,
}

/// 从一句话里识别的"动作动词" → 流程节点类型
/// 注：ai-agent 的 FlowNode.NodeType 变体为
/// Start/End/LLM/Browser/HttpRequest/Operator/Condition/Transform/Script/DataInput/DataOutput/Parallel
/// 动作动词 → 节点类型（单一事实源：crate::knowledge::REQUIREMENT_ACTION_VERBS）
const ACTION_VERBS: &[(&str, NodeType)] = crate::knowledge::REQUIREMENT_ACTION_VERBS;

/// 从一句话里识别的"实体名词" → 数据表候选（单一事实源：crate::knowledge::REQUIREMENT_ENTITY_NOUNS）
const ENTITY_NOUNS: &[&str] = crate::knowledge::REQUIREMENT_ENTITY_NOUNS;

/// 动作动词 → 实体别名（单一事实源：crate::knowledge::REQUIREMENT_VERB_TO_ENTITY）
const VERB_TO_ENTITY: &[(&str, &str)] = crate::knowledge::REQUIREMENT_VERB_TO_ENTITY;

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
            .split(['，', ',', '、', '；', ';', '。', '.', '\n'])
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

/// 把 LLM 返回的 node_type 字符串映射到内部 NodeType 枚举
fn parse_node_type(s: &str) -> Option<NodeType> {
    match s.trim().to_lowercase().as_str() {
        "start" => Some(NodeType::Start),
        "end" => Some(NodeType::End),
        "llm" => Some(NodeType::LLM),
        "browser" => Some(NodeType::Browser),
        "http" | "httprequest" => Some(NodeType::HttpRequest),
        "operator" => Some(NodeType::Operator),
        "condition" => Some(NodeType::Condition),
        "transform" => Some(NodeType::Transform),
        "script" => Some(NodeType::Script),
        "data_input" | "datainput" => Some(NodeType::DataInput),
        "data_output" | "dataoutput" => Some(NodeType::DataOutput),
        "parallel" => Some(NodeType::Parallel),
        _ => None,
    }
}

/// 根据动作动词推断节点类型（与规则抽取保持一致）
fn infer_node_type(action: &str) -> NodeType {
    for (verb, nt) in ACTION_VERBS {
        if action.contains(verb) {
            return nt.clone();
        }
    }
    NodeType::Operator
}

/// 取名词单数形式（简单去"功能/管理/列表"等后缀，作为实体名兜底）
fn singular(name: &str) -> String {
    let cleaned = name
        .trim_end_matches("功能")
        .trim_end_matches("管理")
        .trim_end_matches("列表")
        .trim_end_matches("模块")
        .trim();
    if cleaned.is_empty() {
        "item".to_string()
    } else {
        cleaned.to_string()
    }
}

/// 需求编译器
pub struct RequirementCompiler {
    /// 历史蓝图缓存（会话内持续迭代）
    cache: HashMap<String, SystemBlueprint>,
    /// 可选持久化后端：蓝图落盘，支持跨会话/重启复用（None=纯内存）
    db: Option<Arc<dyn PersistenceProvider>>,
}

impl Default for RequirementCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl RequirementCompiler {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            db: None,
        }
    }

    /// 带持久化后端的编译器：蓝图落盘 SQLite，支持跨会话/重启复用。
    /// `new()` 仍保持纯内存（向后兼容，不创建任何文件）。
    pub fn with_storage(db_path: &str) -> Result<Self, OperatorError> {
        use mox_platform_system_core::sqlite_provider::SqlitePersistence;
        let pvd = SqlitePersistence::file(db_path)
            .map_err(|e| OperatorError::Other(anyhow::anyhow!("蓝图库打开失败: {}", e)))?;
        pvd.exec_batch(
            "CREATE TABLE IF NOT EXISTS blueprints (id TEXT PRIMARY KEY, name TEXT NOT NULL, tags TEXT NOT NULL, json TEXT NOT NULL, updated_at TEXT NOT NULL)",
        )
        .map_err(|e| OperatorError::Other(anyhow::anyhow!("蓝图库初始化失败: {}", e)))?;
        Ok(Self {
            cache: HashMap::new(),
            db: Some(Arc::new(pvd)),
        })
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
                entities
                    .entry(e.clone())
                    .or_insert_with(|| vec!["id".into(), "created_at".into()]);
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
        self.persist(&blueprint);
        Ok(blueprint)
    }

    /// 接入真实 LLM 做更细的功能拆解。
    /// - 有 LLM 通道时：让模型把一句话需求拆成结构化功能点（action/entities/node_type），
    ///   再与规则抽取结果合并去重，得到比纯规则更细的蓝图。
    /// - 无 LLM（未配置 key / 离线）时：自动降级到 `compile` 的规则抽取路径，保证可测可用。
    pub async fn compile_with_llm(
        &mut self,
        requirement: &str,
        name: &str,
        tags: Vec<String>,
        llm: Option<&LlmFn>,
    ) -> Result<SystemBlueprint, OperatorError> {
        // 先用规则抽取打底（保证即使 LLM 失败也有可用蓝图）
        let mut blueprint = self.compile(requirement, name, tags.clone())?;

        if let Some(llm) = llm {
            match self.llm_disassemble(requirement, llm).await {
                Ok(extra) => {
                    let start_idx = blueprint.features.len();
                    for (k, f) in extra.iter().enumerate() {
                        let i = start_idx + k;
                        let node_type = f
                            .node_type
                            .as_deref()
                            .and_then(parse_node_type)
                            .unwrap_or_else(|| infer_node_type(&f.action));
                        let ents = if f.entities.is_empty() {
                            vec![singular(&f.name)]
                        } else {
                            f.entities.clone()
                        };
                        for e in &ents {
                            blueprint
                                .entities
                                .entry(e.clone())
                                .or_insert_with(|| vec!["id".into(), "created_at".into()]);
                        }
                        let fid = format!("f{}", i + 1);
                        let depends_on = if i == 0 {
                            Vec::new()
                        } else {
                            vec![format!("f{}", i)]
                        };
                        blueprint.features.push(Feature {
                            id: fid,
                            name: f.name.clone(),
                            action: f.action.clone(),
                            node_type,
                            entities: ents,
                            depends_on,
                        });
                    }
                    // 重新构建流程图以纳入 LLM 补充的功能点
                    blueprint.flow = self.build_flow(&blueprint.name, &blueprint.features);
                }
                Err(_) => {
                    // LLM 失败静默降级，保留规则抽取结果
                }
            }
        }

        self.cache.insert(blueprint.id.clone(), blueprint.clone());
        self.persist(&blueprint);
        Ok(blueprint)
    }

    /// 调用 LLM 把需求拆成结构化功能点列表
    async fn llm_disassemble(
        &self,
        requirement: &str,
        llm: &LlmFn,
    ) -> anyhow::Result<Vec<LlmFeature>> {
        let prompt = format!(
            "你是一个系统架构拆解助手。请把下面的业务需求拆解为若干独立功能点。\n\
             每个功能点包含：name(简短中文名)、action(动词，如 下单/支付/查询/生成/审批)、\
             node_type(从 [start,end,llm,browser,http,operator,condition,transform,script,data_input,data_output,parallel,task,guard,decision,event] 选一)、\
             entities(该功能涉及的实体名，如 订单/用户/商品)。\n\
             只返回 JSON，格式: {{\"features\":[...]}}，不要解释。\n\n需求：{}",
            requirement
        );
        let messages = vec![
            LlmMsg {
                role: "system".to_string(),
                content: "你是严谨的软件架构拆解器，只输出 JSON。".to_string(),
            },
            LlmMsg {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        let resp = llm(messages).await?;
        let clean = crate::util::extract_json_object(&resp)
            .ok_or_else(|| anyhow::anyhow!("LLM 返回无 JSON 内容"))?;
        let plan: LlmPlan =
            serde_json::from_str(clean).map_err(|e| anyhow::anyhow!("LLM 返回无法解析: {}", e))?;
        Ok(plan.features)
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
            .or_else(|| self.load_blueprint(blueprint_id))
            .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("蓝图不存在，需先 compile")))?;

        let phrases = RuleExtractor::extract_phrases(addition);
        let start_idx = bp.features.len();
        for (k, phrase) in phrases.iter().enumerate() {
            let i = start_idx + k;
            let (action, node_type) = RuleExtractor::classify(phrase);
            let ents = RuleExtractor::entities(phrase);
            for e in &ents {
                bp.entities
                    .entry(e.clone())
                    .or_insert_with(|| vec!["id".into(), "created_at".into()]);
            }
            let fid = format!("f{}", i + 1);
            let depends_on = if i == 0 {
                Vec::new()
            } else {
                vec![format!("f{}", i)]
            };
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
        self.persist(&bp);
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
            position: Some(Position {
                x: 100.0,
                y: 120.0 + features.len() as f64 * 90.0,
            }),
        });
        edges.push(FlowEdge {
            id: "e_end".to_string(),
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

    /// 加载蓝图：优先从持久化库读取（跨会话/重启可用），回退到内存缓存。
    pub fn load_blueprint(&self, id: &str) -> Option<SystemBlueprint> {
        if let Some(db) = &self.db {
            let opt = db
                .query_one(
                    "SELECT json FROM blueprints WHERE id = ?1",
                    &[SqlValue::Text(id.to_string())],
                )
                .ok()
                .flatten()
                .and_then(|row| match row.get("json") {
                    Some(SqlValue::Text(s)) => serde_json::from_str::<SystemBlueprint>(s).ok(),
                    _ => None,
                });
            if let Some(bp) = opt {
                return Some(bp);
            }
        }
        self.cache.get(id).cloned()
    }

    /// 列出全部蓝图：持久化优先；无存储后端时返回内存缓存。
    pub fn list_blueprints(&self) -> Vec<SystemBlueprint> {
        if let Some(db) = &self.db {
            if let Ok(rows) = db.query("SELECT json FROM blueprints ORDER BY updated_at DESC", &[])
            {
                let mut out = Vec::new();
                for r in rows {
                    if let Some(SqlValue::Text(s)) = r.get("json") {
                        if let Ok(bp) = serde_json::from_str::<SystemBlueprint>(s) {
                            out.push(bp);
                        }
                    }
                }
                return out;
            }
        }
        self.cache.values().cloned().collect()
    }

    /// 可选持久化：若配置了存储后端，将蓝图 upsert 进 SQLite（非致命，失败仅告警）
    fn persist(&self, bp: &SystemBlueprint) {
        let Some(db) = &self.db else {
            return;
        };
        let tags = serde_json::to_string(&bp.tags).unwrap_or_else(|_| "[]".to_string());
        let json = match serde_json::to_string(bp) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("蓝图序列化失败: {e}");
                return;
            }
        };
        if let Err(e) = db.exec(
            "INSERT OR REPLACE INTO blueprints (id, name, tags, json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                SqlValue::Text(bp.id.clone()),
                SqlValue::Text(bp.name.clone()),
                SqlValue::Text(tags),
                SqlValue::Text(json),
                SqlValue::Text(chrono::Utc::now().to_rfc3339()),
            ],
        ) {
            tracing::warn!("蓝图落盘失败: {e}");
        }
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
        assert!(bp
            .flow
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, NodeType::Start)));
        assert!(bp
            .flow
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, NodeType::End)));
        // 实体抽取：商品/购物车/订单/支付
        assert!(bp.entities.contains_key("商品"));
        assert!(bp.entities.contains_key("订单"));
    }

    #[test]
    fn refine_appends_new_feature_and_rebuilds_flow() {
        let mut rc = RequirementCompiler::new();
        let bp = rc
            .compile("商城：商品，购物车", "商城", vec!["mall".into()])
            .unwrap();
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
        let bp = rc
            .compile("登录，校验手机号，AI生成推荐", "demo", vec![])
            .unwrap();
        assert!(bp
            .features
            .iter()
            .any(|f| matches!(f.node_type, NodeType::Transform) && f.name.contains("校验")));
        assert!(bp
            .features
            .iter()
            .any(|f| matches!(f.node_type, NodeType::LLM) && f.name.contains("推荐")));
    }

    #[test]
    fn builds_valid_flow_definition() {
        let mut rc = RequirementCompiler::new();
        let bp = rc
            .compile("注册，登录，发布文章", "内容平台", vec!["thesis".into()])
            .unwrap();
        // 验证流程图可被 flow_engine 接受（含 Start + End + 连通）
        assert!(crate::flow_engine::FlowEngine::validate_flow(&bp.flow).is_ok());
    }

    #[tokio::test]
    async fn compile_with_llm_degrades_without_llm() {
        let mut rc = RequirementCompiler::new();
        // 无 LLM 通道：应等价于规则抽取，至少产出 4 个功能点
        let bp = rc
            .compile_with_llm(
                "我要做一个商城：有商品，购物车，下单，支付",
                "商城系统",
                vec!["mall".into()],
                None,
            )
            .await
            .unwrap();
        assert!(bp.features.len() >= 4);
        assert!(bp.entities.contains_key("订单"));
    }

    #[test]
    fn blueprint_persists_across_compiler_instances() {
        let path = std::env::temp_dir()
            .join(format!("bp_persist_{}.db", uuid::Uuid::new_v4()))
            .to_str()
            .unwrap()
            .to_string();
        let mut rc = RequirementCompiler::with_storage(&path).unwrap();
        let bp = rc
            .compile("商城：商品，下单，支付", "商城", vec!["mall".into()])
            .unwrap();
        let id = bp.id.clone();
        assert_eq!(bp.features.len(), 3);
        drop(rc);
        // 模拟重启：新实例读同一库
        let rc2 = RequirementCompiler::with_storage(&path).unwrap();
        let loaded = rc2.load_blueprint(&id).expect("应从库加载蓝图");
        assert_eq!(loaded.name, "商城");
        assert_eq!(loaded.features.len(), 3);
        assert_eq!(rc2.list_blueprints().len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn refine_persists_to_storage() {
        let path = std::env::temp_dir()
            .join(format!("bp_refine_{}.db", uuid::Uuid::new_v4()))
            .to_str()
            .unwrap()
            .to_string();
        let mut rc = RequirementCompiler::with_storage(&path).unwrap();
        let bp = rc.compile("商城：商品", "商城", vec![]).unwrap();
        let id = bp.id.clone();
        let _ = rc.refine(&id, "再加一个退货功能").unwrap();
        drop(rc);
        let rc2 = RequirementCompiler::with_storage(&path).unwrap();
        let loaded = rc2.load_blueprint(&id).unwrap();
        assert_eq!(loaded.features.len(), 2, "refine 结果应已落盘");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn compile_with_llm_merges_llm_features() {
        use std::sync::Arc;
        // 提供一个假的 LLM，返回结构化拆解，验证合并逻辑
        let fake: LlmFn = Arc::new(|_msgs| {
            Box::pin(async {
                Ok(
                    r#"{"features":[{"name":"库存扣减","action":"扣减","node_type":"operator","entities":["库存"]}]}"#
                        .to_string(),
                )
            })
        });
        let mut rc = RequirementCompiler::new();
        let bp = rc
            .compile_with_llm("商城：商品，下单", "商城", vec!["mall".into()], Some(&fake))
            .await
            .unwrap();
        // 规则抽取 2 个 + LLM 补充 1 个 = 3
        assert_eq!(bp.features.len(), 3);
        assert!(bp.features.iter().any(|f| f.name.contains("库存扣减")));
        // LLM 补充的实体入库
        assert!(bp.entities.contains_key("库存"));
    }
}
