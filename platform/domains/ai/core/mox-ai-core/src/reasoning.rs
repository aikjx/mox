// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # AI 增强推理模块
//!
//! 将 LLM 的语义理解能力与图谱结构结合，实现：
//! - 意图识别（Intent Detection）：将自然语言映射到图谱操作
//! - 语义搜索（Semantic Search）：结合图结构的 LLM 增强搜索
//! - 路径解释（Path Explanation）：用 LLM 解释图谱推理路径
//! - 新知识生成（Knowledge Synthesis）：从对话中提取新节点/边
//! - 因果推理（Causal Reasoning）：AI 辅助因果链路发现
//!
//! ## 与图谱算法的分工
//!
//! | 能力 | 引擎 | 说明 |
//! |-------|------|------|
//! | 确定性路径搜索 | 图谱算法（MoxGraph） | A* / BFS / Hebbian |
//! | 意图分类/知识抽取 | AI 推理层（本模块） | LLM 调用 |
//! | 持久化 | 上层存储 | 图谱 + 对话历史 |

use crate::graph::{GraphEdge, GraphNode, NodeId, RelationId, MoxGraph};
use crate::providers::{AiError, AiProvider, ChatMessage, ChatRequest, ModelConfig};
use serde::{Deserialize, Serialize};

/// 推理能力类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReasoningCapability {
    /// 意图识别：自然语言 → 图谱操作
    IntentDetection,
    /// 语义搜索：基于描述找相关节点
    SemanticSearch,
    /// 路径解释：给推理链生成自然语言描述
    PathExplanation,
    /// 知识抽取：从对话提取新节点/边
    KnowledgeExtraction,
    /// 关系发现：发现图中缺失的隐含关系
    RelationDiscovery,
    /// 因果推理：AI 辅助因果链路推断
    CausalReasoning,
}

/// AI 推理请求
#[derive(Debug, Clone)]
pub struct ReasoningRequest {
    /// 能力类型
    pub capability: ReasoningCapability,
    /// 自然语言输入
    pub query: String,
    /// 关联图谱节点 ID（可选）
    pub focus_nodes: Vec<String>,
    /// 最大结果数
    pub max_results: usize,
}

/// AI 推理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    /// 使用的推理能力
    pub capability: ReasoningCapability,
    /// 自然语言解释
    pub explanation: String,
    /// 提取的新节点（知识抽取用）
    pub new_nodes: Vec<ExtractedNode>,
    /// 提取的新边（知识抽取用）
    pub new_edges: Vec<ExtractedEdge>,
    /// 置信度（0.0~1.0）
    pub confidence: f32,
    /// 推理耗时（ms）
    pub latency_ms: u64,
}

/// 从对话/文本中提取的新节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub description: String,
    pub confidence: f32,
}

/// 从对话/文本中提取的新边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub description: String,
    pub confidence: f32,
}

/// AI 推理器
pub struct AiReasoner {
    provider: Box<dyn AiProvider>,
    model: String,
}

impl AiReasoner {
    pub fn new(provider: Box<dyn AiProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
        }
    }

    /// 执行推理（async）
    pub async fn reason(&self, req: &ReasoningRequest) -> Result<ReasoningResult, AiError> {
        let start = std::time::Instant::now();
        let prompt = build_reasoning_prompt(req);
        let messages = vec![
            ChatMessage::system(REASONING_SYSTEM_PROMPT),
            ChatMessage::user(prompt),
        ];
        let chat_req = ChatRequest {
            messages,
            config: ModelConfig {
                model: self.model.clone(),
                max_tokens: 2048,
                temperature: 0.3,
                ..Default::default()
            },
        };
        let response = self.provider.chat(&chat_req).await?;
        let latency_ms = start.elapsed().as_millis() as u64;
        parse_reasoning_result(&response.content, req.capability, latency_ms)
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

/// 构建推理提示词
fn build_reasoning_prompt(req: &ReasoningRequest) -> String {
    let capability_desc = match req.capability {
        ReasoningCapability::IntentDetection => "意图识别",
        ReasoningCapability::SemanticSearch => "语义搜索",
        ReasoningCapability::PathExplanation => "路径解释",
        ReasoningCapability::KnowledgeExtraction => "知识抽取",
        ReasoningCapability::RelationDiscovery => "关系发现",
        ReasoningCapability::CausalReasoning => "因果推理",
    };

    let focus = if req.focus_nodes.is_empty() {
        "无特定节点".into()
    } else {
        req.focus_nodes.join(", ")
    };

    format!(
        "## 任务类型：{}\n\
         ## 用户查询：{}\n\
         ## 聚焦节点：{}\n\
         ## 最多结果：{}\n\n\
         请按以下格式输出（JSON）：\n\
         {{\n\
         \"explanation\": \"自然语言解释\",\n\
         \"new_nodes\": [{{\"id\": \"节点ID\", \"label\": \"标签\", \"type\": \"类型\", \"description\": \"描述\", \"confidence\": 0.9}}],\n\
         \"new_edges\": [{{\"from\": \"起始节点\", \"to\": \"目标节点\", \"relation\": \"关系类型\", \"description\": \"描述\", \"confidence\": 0.85}}],\n\
         \"confidence\": 0.9\n\
         }}\n\
         如果不需要抽取节点/边，返回空数组。注意：confidence 为 0.0~1.0。",
        capability_desc, req.query, focus, req.max_results
    )
}

/// 解析推理结果（简单 JSON 解析）
fn parse_reasoning_result(
    response: &str,
    capability: ReasoningCapability,
    latency_ms: u64,
) -> Result<ReasoningResult, AiError> {
    // 尝试提取 JSON 代码块
    let json_str = response
        .trim()
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| response.trim().strip_prefix("```").and_then(|s| s.strip_suffix("```")))
        .unwrap_or(response.trim())
        .trim();

    let json: serde_json::Value = serde_json::from_str(json_str)
        .or_else(|_| serde_json::from_str(response))
        .map_err(|e| {
            AiError::Other(format!("JSON解析失败: {} | 原始响应: {}", e, response))
        })?;

    let explanation = json
        .get("explanation")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let confidence = json
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;

    let new_nodes: Vec<ExtractedNode> = json
        .get("new_nodes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(ExtractedNode {
                        id: v.get("id")?.as_str()?.to_string(),
                        label: v.get("label")?.as_str()?.to_string(),
                        node_type: v.get("type")?.as_str()?.to_string(),
                        description: v.get("description")?.as_str()?.to_string(),
                        confidence: v.get("confidence")?.as_f64().unwrap_or(0.5) as f32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let new_edges: Vec<ExtractedEdge> = json
        .get("new_edges")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(ExtractedEdge {
                        from: v.get("from")?.as_str()?.to_string(),
                        to: v.get("to")?.as_str()?.to_string(),
                        relation: v.get("relation")?.as_str()?.to_string(),
                        description: v.get("description")?.as_str()?.to_string(),
                        confidence: v.get("confidence")?.as_f64().unwrap_or(0.5) as f32,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ReasoningResult {
        capability,
        explanation,
        new_nodes,
        new_edges,
        confidence,
        latency_ms,
    })
}

/// 图谱感知推理器 — 同时利用图结构和 AI
pub struct GraphAwareReasoner {
    ai: AiReasoner,
}

impl GraphAwareReasoner {
    pub fn new(provider: Box<dyn AiProvider>, model: impl Into<String>) -> Self {
        Self {
            ai: AiReasoner::new(provider, model),
        }
    }

    /// 语义搜索：基于描述找最相关的图谱节点（async）
    pub async fn semantic_search(
        &self,
        graph: &MoxGraph,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<SemanticMatch>, AiError> {
        let req = ReasoningRequest {
            capability: ReasoningCapability::SemanticSearch,
            query: query.into(),
            focus_nodes: graph.nodes.keys().map(|k| k.0.clone()).collect(),
            max_results,
        };
        let result = self.ai.reason(&req).await?;

        // 将 AI 返回的节点描述与图谱匹配
        let matches: Vec<SemanticMatch> = result
            .new_nodes
            .iter()
            .filter_map(|n| {
                graph
                    .nodes
                    .get(&NodeId(n.id.clone()))
                    .map(|gn| SemanticMatch {
                        node: gn.clone(),
                        match_reason: result.explanation.clone(),
                        confidence: n.confidence,
                    })
            })
            .collect();

        Ok(matches)
    }

    /// 从对话中提取新知识并注入图谱（async）
    pub async fn extract_and_inject(
        &self,
        graph: &mut MoxGraph,
        user_input: &str,
    ) -> Result<ExtractionSummary, AiError> {
        let req = ReasoningRequest {
            capability: ReasoningCapability::KnowledgeExtraction,
            query: user_input.into(),
            focus_nodes: vec![],
            max_results: 20,
        };
        let result = self.ai.reason(&req).await?;

        let mut injected_nodes = 0;
        let mut injected_edges = 0;

        for node in &result.new_nodes {
            if node.confidence < 0.7 {
                continue;
            }
            let id = NodeId(node.id.clone());
            if !graph.nodes.contains_key(&id) {
                graph.add_node(GraphNode::new(id, node.label.clone()));
                injected_nodes += 1;
            }
        }

        for edge in &result.new_edges {
            if edge.confidence < 0.7 {
                continue;
            }
            let from = NodeId(edge.from.clone());
            let to = NodeId(edge.to.clone());
            let id = RelationId(format!("{:?}-{:?}-{}", from, to, edge.relation));
            if !graph.edges.contains_key(&id) {
                graph.add_edge(GraphEdge::new(from, to, edge.relation.clone()));
                injected_edges += 1;
            }
        }

        Ok(ExtractionSummary {
            explanation: result.explanation,
            nodes_injected: injected_nodes,
            edges_injected: injected_edges,
            total_extracted: result.new_nodes.len() + result.new_edges.len(),
            confidence: result.confidence,
        })
    }

    /// AI 辅助因果推理：给定起点，推理可能因果路径（async）
    pub async fn causal_analysis(
        &self,
        graph: &MoxGraph,
        start_id: &str,
        target_hint: Option<&str>,
    ) -> Result<CausalAnalysisResult, AiError> {
        let node_desc = graph
            .nodes
            .get(&NodeId(start_id.into()))
            .map(|n| format!("[{}] {}", n.id, n.label))
            .unwrap_or_else(|| start_id.into());

        let req = ReasoningRequest {
            capability: ReasoningCapability::CausalReasoning,
            query: format!(
                "分析节点 {} 的因果链{}",
                node_desc,
                target_hint
                    .map(|t| format!("，目标：{}", t))
                    .unwrap_or_default()
            ),
            focus_nodes: vec![start_id.into()],
            max_results: 10,
        };

        let result = self.ai.reason(&req).await?;

        Ok(CausalAnalysisResult {
            start_node: start_id.into(),
            explanation: result.explanation,
            discovered_nodes: result.new_nodes,
            discovered_edges: result.new_edges,
            graph_path: None, // 图谱算法路径由调用侧填充
            confidence: result.confidence,
        })
    }

    pub fn reasoner(&self) -> &AiReasoner {
        &self.ai
    }
}

/// 语义搜索匹配结果
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    pub node: GraphNode,
    pub match_reason: String,
    pub confidence: f32,
}

/// 抽取摘要
#[derive(Debug, Clone)]
pub struct ExtractionSummary {
    pub explanation: String,
    pub nodes_injected: usize,
    pub edges_injected: usize,
    pub total_extracted: usize,
    pub confidence: f32,
}

/// 因果分析结果
#[derive(Debug, Clone)]
pub struct CausalAnalysisResult {
    pub start_node: String,
    pub explanation: String,
    pub discovered_nodes: Vec<ExtractedNode>,
    pub discovered_edges: Vec<ExtractedEdge>,
    pub graph_path: Option<Vec<String>>, // 图谱算法发现的路径
    pub confidence: f32,
}

/// 推理系统提示词
const REASONING_SYSTEM_PROMPT: &str = r#"\
你是一个专业的知识图谱推理助手。你的任务是：

1. **意图识别**：将用户自然语言转换为图谱操作意图
2. **知识抽取**：从对话中提取结构化的节点和关系
3. **关系发现**：发现图中隐含但未明确表达的关联
4. **因果推理**：分析事件之间的因果链路

输出要求：
- 始终返回结构化 JSON（见用户请求格式）
- new_nodes 中的 id 必须是可以直接作为图谱节点 ID 的字符串
- new_edges 中的 from/to 必须是已存在或新创建的节点 ID
- confidence 反映你对抽取结果的信心（0.0~1.0），低于 0.6 的结果请省略
- explanation 使用简洁的中文描述
- 严格遵循 JSON 格式，不要添加额外的解释文字"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reasoning_result_valid_json() {
        let response = r#"{
            "explanation": "测试解释",
            "new_nodes": [{"id": "n1", "label": "节点1", "type": "TypeA", "description": "desc", "confidence": 0.9}],
            "new_edges": [{"from": "n1", "to": "n2", "relation": "rel", "description": "edge desc", "confidence": 0.8}],
            "confidence": 0.85
        }"#;
        let result =
            parse_reasoning_result(response, ReasoningCapability::KnowledgeExtraction, 100)
                .unwrap();
        assert_eq!(result.explanation, "测试解释");
        assert_eq!(result.new_nodes.len(), 1);
        assert_eq!(result.new_edges.len(), 1);
        assert!((result.confidence - 0.85).abs() < 0.001);
        assert_eq!(result.latency_ms, 100);
    }

    #[test]
    fn test_parse_reasoning_result_code_block() {
        let response = "```json\n{\"explanation\": \"code block\", \"new_nodes\": [], \"new_edges\": [], \"confidence\": 0.5}\n```";
        let result =
            parse_reasoning_result(response, ReasoningCapability::IntentDetection, 0).unwrap();
        assert_eq!(result.explanation, "code block");
    }

    #[test]
    fn test_parse_reasoning_result_empty_arrays() {
        let response = r#"{"explanation": "no nodes", "new_nodes": [], "new_edges": [], "confidence": 0.3}"#;
        let result =
            parse_reasoning_result(response, ReasoningCapability::SemanticSearch, 50).unwrap();
        assert!(result.new_nodes.is_empty());
        assert!(result.new_edges.is_empty());
    }

    #[test]
    fn test_build_reasoning_prompt() {
        let req = ReasoningRequest {
            capability: ReasoningCapability::CausalReasoning,
            query: "分析订单延迟原因".into(),
            focus_nodes: vec!["order:001".into()],
            max_results: 5,
        };
        let prompt = build_reasoning_prompt(&req);
        assert!(prompt.contains("因果推理"));
        assert!(prompt.contains("分析订单延迟原因"));
        assert!(prompt.contains("order:001"));
    }

    #[test]
    fn test_reasoning_capability_serialization() {
        let cap = ReasoningCapability::KnowledgeExtraction;
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: ReasoningCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, deserialized);
    }
}
