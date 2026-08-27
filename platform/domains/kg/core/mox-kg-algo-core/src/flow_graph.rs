// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # AI 流程图谱引擎（Rust 版）
//!
//! 与 Node 层 `ai-flow-graph.js` 跨语言对齐的实现。
//! 设计依据：docs/modules/ai-flow-graph-design.md
//!
//! 核心命题：图谱即 AI 引擎的流程基础设施。
//!   - 业务流程（五步流水线/能力矩阵/降级链）建模为 step/keyword/capability/engine 节点与 4 类边
//!   - 算法流程（意图识别）= 图谱上的激活扩散（个性化 PageRank，委托修复后的统一实现）
//!
//! 节点类型：step / keyword / capability / engine
//! 边类型：triggers（词→能力）/ flows_to（流水线）/ delegates_to（委托）/ degrades_to（降级）

use crate::{KnowledgeEdge, KnowledgeGraph, KnowledgeNode};
use std::collections::HashMap;

/// 意图关键词规则：命中关键词即按权重激活对应能力
#[derive(Debug, Clone)]
pub struct IntentRule {
    pub capability: String,
    pub keyword: String,
    pub weight: f64,
}

/// 能力元数据：委托引擎与描述
#[derive(Debug, Clone)]
pub struct CapabilityMeta {
    pub id: String,
    pub engine: String,
    pub description: String,
    pub is_default: bool,
}

/// 激活扩散意图识别结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct IntentResult {
    pub intent: String,
    pub score: f64,
    pub scores: HashMap<String, f64>,
    pub matched_keywords: Vec<String>,
    pub method: &'static str,
    pub iterations_hint: usize,
}

/// 流程图谱统计（自检用）
#[derive(Debug, Clone, serde::Serialize)]
pub struct FlowGraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub step_nodes: usize,
    pub keyword_nodes: usize,
    pub capability_nodes: usize,
    pub engine_nodes: usize,
    pub trigger_edges: usize,
    pub flow_edges: usize,
    pub delegate_edges: usize,
    pub degrade_edges: usize,
}

const PIPELINE: [&str; 5] = ["intent", "route", "execute", "verify", "feedback"];

const STEP_TITLES: [(&str, &str); 5] = [
    ("intent", "意图识别"),
    ("route", "能力路由"),
    ("execute", "引擎执行"),
    ("verify", "质量校验"),
    ("feedback", "指标反馈"),
];

/// AI 流程图谱：业务流程 + 算法流程统一承载于图谱引擎
pub struct AIFlowGraph {
    graph: KnowledgeGraph,
    capabilities: Vec<CapabilityMeta>,
    keywords: Vec<(String, String, f64)>, // (keyword, capability, weight) 去重后
    trigger_edge_count: usize,
}

fn make_node(id: &str, label: &str, node_type: &str) -> KnowledgeNode {
    KnowledgeNode {
        id: id.to_string(),
        label: label.to_string(),
        node_type: node_type.to_string(),
        properties: serde_json::json!({}),
        embedding: None,
        activation: 0.0,
        metadata: HashMap::new(),
    }
}

impl AIFlowGraph {
    /// 从意图规则与能力矩阵构建流程图谱
    ///
    /// 两阶段构建（先全部节点、后全部边）：KnowledgeGraph::add_edge 要求两端节点已存在，
    /// 单阶段边建边加会因目标节点未建而静默失败（T8 实测发现边数 4/31）。
    pub fn build(rules: &[IntentRule], capabilities: &[CapabilityMeta]) -> Self {
        let mut graph = KnowledgeGraph::new();

        // ===== 阶段一：全部节点 =====

        // ① step 节点（业务流程骨架）
        for (i, step) in PIPELINE.iter().enumerate() {
            let title = STEP_TITLES
                .iter()
                .find(|(s, _)| s == step)
                .map(|(_, t)| *t)
                .unwrap_or(step);
            graph.add_node(make_node(
                &format!("step:{}", step),
                &format!("{}. {}", i + 1, title),
                "step",
            ));
        }

        // ② keyword 节点（去重：一词一节点）
        let mut seen_kw: HashMap<String, bool> = HashMap::new();
        let mut keywords: Vec<(String, String, f64)> = Vec::new();
        for rule in rules {
            let kw_id = format!("kw:{}", rule.keyword);
            if !seen_kw.contains_key(&kw_id) {
                seen_kw.insert(kw_id.clone(), true);
                let mut node = make_node(&kw_id, &rule.keyword, "keyword");
                node.activation = rule.weight;
                graph.add_node(node);
                keywords.push((rule.keyword.clone(), rule.capability.clone(), rule.weight));
            }
        }

        // ③ capability 节点
        for cap in capabilities {
            graph.add_node(make_node(&format!("cap:{}", cap.id), &cap.id, "capability"));
        }

        // ④ engine 节点（去重）
        let mut engines: Vec<String> = capabilities
            .iter()
            .map(|c| c.engine.clone())
            .filter(|e| !e.is_empty())
            .collect();
        engines.sort();
        engines.dedup();
        for eng in &engines {
            graph.add_node(make_node(&format!("eng:{}", eng), eng, "engine"));
        }

        // ===== 阶段二：全部边（节点已齐，不会静默失败） =====

        let mut trigger_edge_count = 0usize;

        // flows_to：流水线顺序
        for w in PIPELINE.windows(2) {
            graph
                .add_edge(KnowledgeEdge {
                    source: format!("step:{}", w[0]),
                    target: format!("step:{}", w[1]),
                    weight: 1.0,
                    relation_type: "flows_to".to_string(),
                    properties: serde_json::json!({}),
                })
                .expect("flows_to 边构建失败：step 节点缺失");
        }

        // triggers：关键词→能力
        for rule in rules {
            graph
                .add_edge(KnowledgeEdge {
                    source: format!("kw:{}", rule.keyword),
                    target: format!("cap:{}", rule.capability),
                    weight: rule.weight,
                    relation_type: "triggers".to_string(),
                    properties: serde_json::json!({}),
                })
                .expect("triggers 边构建失败：kw/cap 节点缺失");
            trigger_edge_count += 1;
        }

        // delegates_to / degrades_to
        for cap in capabilities {
            if !cap.engine.is_empty() {
                graph
                    .add_edge(KnowledgeEdge {
                        source: format!("cap:{}", cap.id),
                        target: format!("eng:{}", cap.engine),
                        weight: 1.0,
                        relation_type: "delegates_to".to_string(),
                        properties: serde_json::json!({}),
                    })
                    .expect("delegates_to 边构建失败：cap/eng 节点缺失");
            }
            if !cap.is_default {
                graph
                    .add_edge(KnowledgeEdge {
                        source: format!("cap:{}", cap.id),
                        target: "cap:chat".to_string(),
                        weight: 0.5,
                        relation_type: "degrades_to".to_string(),
                        properties: serde_json::json!({}),
                    })
                    .expect("degrades_to 边构建失败：cap:chat 节点缺失");
            }
        }

        Self {
            graph,
            capabilities: capabilities.to_vec(),
            keywords,
            trigger_edge_count,
        }
    }

    /// 默认流程图谱：与 Node 层 INTENT_KEYWORDS 核心子集对齐
    pub fn default_config() -> Self {
        let rules = vec![
            // graph
            IntentRule {
                capability: "graph".into(),
                keyword: "图谱".into(),
                weight: 3.0,
            },
            IntentRule {
                capability: "graph".into(),
                keyword: "PageRank".into(),
                weight: 3.0,
            },
            IntentRule {
                capability: "graph".into(),
                keyword: "中心性".into(),
                weight: 2.5,
            },
            IntentRule {
                capability: "graph".into(),
                keyword: "社区结构".into(),
                weight: 2.5,
            },
            IntentRule {
                capability: "graph".into(),
                keyword: "节点关系".into(),
                weight: 2.0,
            },
            // reasoning
            IntentRule {
                capability: "reasoning".into(),
                keyword: "深度推理".into(),
                weight: 3.0,
            },
            IntentRule {
                capability: "reasoning".into(),
                keyword: "逐步分析".into(),
                weight: 2.5,
            },
            IntentRule {
                capability: "reasoning".into(),
                keyword: "逻辑推理".into(),
                weight: 2.5,
            },
            IntentRule {
                capability: "reasoning".into(),
                keyword: "证明".into(),
                weight: 2.0,
            },
            // expert
            IntentRule {
                capability: "expert".into(),
                keyword: "专家".into(),
                weight: 3.0,
            },
            IntentRule {
                capability: "expert".into(),
                keyword: "会诊".into(),
                weight: 3.0,
            },
            IntentRule {
                capability: "expert".into(),
                keyword: "多角度".into(),
                weight: 2.0,
            },
            // memory
            IntentRule {
                capability: "memory".into(),
                keyword: "记忆".into(),
                weight: 2.5,
            },
            IntentRule {
                capability: "memory".into(),
                keyword: "历史记录".into(),
                weight: 2.0,
            },
            // workflow
            IntentRule {
                capability: "workflow".into(),
                keyword: "工作流".into(),
                weight: 3.0,
            },
            IntentRule {
                capability: "workflow".into(),
                keyword: "流程编排".into(),
                weight: 2.5,
            },
        ];
        let capabilities = vec![
            CapabilityMeta {
                id: "expert".into(),
                engine: "expert-alliance-engine".into(),
                description: "专家联盟会诊".into(),
                is_default: false,
            },
            CapabilityMeta {
                id: "reasoning".into(),
                engine: "ultimate-ai-engine".into(),
                description: "终极深度推理".into(),
                is_default: false,
            },
            CapabilityMeta {
                id: "memory".into(),
                engine: "ai-engine".into(),
                description: "记忆检索".into(),
                is_default: false,
            },
            CapabilityMeta {
                id: "graph".into(),
                engine: "ai-engine".into(),
                description: "图谱分析".into(),
                is_default: false,
            },
            CapabilityMeta {
                id: "workflow".into(),
                engine: "ai-engine".into(),
                description: "工作流执行".into(),
                is_default: false,
            },
            CapabilityMeta {
                id: "chat".into(),
                engine: "llm-gateway".into(),
                description: "通用对话（默认）".into(),
                is_default: true,
            },
        ];
        Self::build(&rules, &capabilities)
    }

    /// 激活扩散意图识别（F8，与 Node 层算法对齐）
    ///
    /// a_i = (1-d)·p_i + d·(Σ_{j→i} a_j·W(j,i)/outW(j) + dangling_mass/n)
    /// 命中关键词构成个性化向量 p，在流程图谱上跑个性化 PageRank，
    /// 取 capability 节点中激活值最高者（平局取字典序最小，确定性）。
    pub fn detect_intent_by_spread(&self, question: &str) -> IntentResult {
        let text = question;

        // ① 命中检测
        let hits: Vec<&(String, String, f64)> = self
            .keywords
            .iter()
            .filter(|(kw, _, _)| text.contains(kw.as_str()))
            .collect();

        if hits.is_empty() {
            let mut scores = HashMap::new();
            for cap in &self.capabilities {
                scores.insert(cap.id.clone(), 0.0);
            }
            return IntentResult {
                intent: "chat".to_string(),
                score: 0.0,
                scores,
                matched_keywords: vec![],
                method: "spread",
                iterations_hint: 0,
            };
        }

        // ② 个性化向量（命中关键词按权重归一）
        let mut personalization: HashMap<String, f64> = HashMap::new();
        for (kw, _, w) in &hits {
            *personalization.entry(format!("kw:{}", kw)).or_insert(0.0) += *w;
        }

        // ③ 激活扩散：委托修复后的个性化 PageRank 单源实现
        let activation = self.graph.pagerank_personalized(&personalization, 50);

        // ④ 能力排序（确定性：平局取字典序最小）
        let mut cap_scores: Vec<(String, f64)> = self
            .capabilities
            .iter()
            .map(|cap| {
                let sc = activation
                    .get(&format!("cap:{}", cap.id))
                    .copied()
                    .unwrap_or(0.0);
                (cap.id.clone(), sc)
            })
            .collect();
        cap_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));

        let (best, best_score) = cap_scores[0].clone();
        let scores: HashMap<String, f64> = cap_scores.into_iter().collect();
        let matched: Vec<String> = hits.iter().map(|(kw, _, _)| kw.clone()).collect();

        IntentResult {
            intent: best,
            score: best_score,
            scores,
            matched_keywords: matched,
            method: "spread",
            iterations_hint: 50,
        }
    }

    /// 流程图谱统计（自检：节点/边数量守恒）
    pub fn stats(&self) -> FlowGraphStats {
        let mut engines: Vec<String> = self
            .capabilities
            .iter()
            .map(|c| c.engine.clone())
            .filter(|e| !e.is_empty())
            .collect();
        engines.sort();
        engines.dedup();

        FlowGraphStats {
            node_count: self.graph.node_count(),
            edge_count: self.graph.edge_count(),
            step_nodes: PIPELINE.len(),
            keyword_nodes: self.keywords.len(),
            capability_nodes: self.capabilities.len(),
            engine_nodes: engines.len(),
            trigger_edges: self.trigger_edge_count,
            flow_edges: PIPELINE.len() - 1,
            delegate_edges: self
                .capabilities
                .iter()
                .filter(|c| !c.engine.is_empty())
                .count(),
            degrade_edges: self.capabilities.iter().filter(|c| !c.is_default).count(),
        }
    }

    /// 借用内部图谱（供上层做进一步图计算）
    pub fn graph(&self) -> &KnowledgeGraph {
        &self.graph
    }
}

// ==================== 公式测试（与 Node 层 T1-T8 同套测试图） ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KnowledgeEdge, KnowledgeGraph, KnowledgeNode};
    use std::collections::HashMap;

    fn node(id: &str) -> KnowledgeNode {
        KnowledgeNode {
            id: id.to_string(),
            label: id.to_string(),
            node_type: "test".to_string(),
            properties: serde_json::json!({}),
            embedding: None,
            activation: 0.0,
            metadata: HashMap::new(),
        }
    }

    fn edge(s: &str, t: &str) -> KnowledgeEdge {
        KnowledgeEdge {
            source: s.to_string(),
            target: t.to_string(),
            weight: 1.0,
            relation_type: "test".to_string(),
            properties: serde_json::json!({}),
        }
    }

    fn build_graph(nodes: &[&str], edges: &[(&str, &str)]) -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();
        for id in nodes {
            g.add_node(node(id));
        }
        for (s, t) in edges {
            g.add_edge(edge(s, t)).unwrap();
        }
        g
    }

    const EPS: f64 = 1e-6;

    // ---------- T1 星型：F2 度中心性（RAW 边）/ F4 介数 + F5 紧密（双向边，无向语义） ----------
    #[test]
    fn t1_star_graph_formulas() {
        let raw: Vec<(&str, &str)> = vec![("c", "s1"), ("c", "s2"), ("c", "s3"), ("c", "s4")];

        // F2 度中心性：RAW 边图（无向度语义：每条边对两端各计 1）
        //   c 度=4 → 4/4=1.0；叶=1 → 0.25（修复 R-D4 前 c=0.5）
        let g_raw = build_graph(&["c", "s1", "s2", "s3", "s4"], &raw);
        let deg = g_raw.degree_centrality();
        assert!(
            (deg["c"] - 1.0).abs() < EPS,
            "度中心性 c 应=1.0，实测 {}",
            deg["c"]
        );
        assert!(
            (deg["s1"] - 0.25).abs() < EPS,
            "度中心性 s1 应=0.25，实测 {}",
            deg["s1"]
        );

        // F4/F5：双向边图（DiGraph 上表达无向语义）
        let bidi: Vec<(&str, &str)> = raw
            .iter()
            .flat_map(|(a, b)| vec![(*a, *b), (*b, *a)])
            .collect();
        let g_bidi = build_graph(&["c", "s1", "s2", "s3", "s4"], &bidi);

        // F4 介数中心性（Brandes 有向版，双向边=无向语义，路径计两次）：
        //   c 未归一化 12，有向归一化 (5-1)(5-2)=12 → c=1.0；叶=0
        let btw = g_bidi.betweenness_centrality();
        assert!(
            (btw["c"] - 1.0).abs() < EPS,
            "介数 c 应=1.0，实测 {}",
            btw["c"]
        );
        assert!(btw["s1"].abs() < EPS, "介数 s1 应=0，实测 {}", btw["s1"]);

        // F5 紧密中心性（harmonic）：c=1.0，叶=0.625
        let cls = g_bidi.closeness_centrality();
        assert!(
            (cls["c"] - 1.0).abs() < EPS,
            "紧密 c 应=1.0，实测 {}",
            cls["c"]
        );
        assert!(
            (cls["s1"] - 0.625).abs() < EPS,
            "紧密 s1 应=0.625，实测 {}",
            cls["s1"]
        );
    }

    // ---------- T2 链（有向）：F3 PageRank / F4 介数 / F5 紧密 ----------
    #[test]
    fn t2_chain_graph_formulas() {
        let g = build_graph(
            &["a", "b", "c", "d", "e"],
            &[("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")],
        );

        // F3 PageRank：ΣPR=1（修复 R-D2 悬挂回传后守恒）；e 最高（汇聚末端）
        let pr = g.pagerank(100);
        let sum: f64 = pr.values().sum();
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "ΣPR 应=1（悬挂回传守恒），实测 {}",
            sum
        );
        let mut sorted: Vec<(String, f64)> = pr.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        assert_eq!(sorted[0].0, "e", "PageRank 最高应为 e（汇聚末端）");

        // F4 介数：b=3/12=0.25；c=4/12=1/3；端点 a、e=0
        let btw = g.betweenness_centrality();
        assert!(
            (btw["b"] - 0.25).abs() < EPS,
            "介数 b 应=0.25，实测 {}",
            btw["b"]
        );
        assert!(
            (btw["c"] - 1.0 / 3.0).abs() < EPS,
            "介数 c 应=1/3，实测 {}",
            btw["c"]
        );
        assert!(btw["a"].abs() < EPS && btw["e"].abs() < EPS, "端点介数应=0");

        // F5 紧密（harmonic）：a=25/48；e=0（不可达任何节点）
        let cls = g.closeness_centrality();
        assert!(
            (cls["a"] - 25.0 / 48.0).abs() < EPS,
            "紧密 a 应=25/48，实测 {}",
            cls["a"]
        );
        assert!(cls["e"].abs() < EPS, "紧密 e 应=0，实测 {}", cls["e"]);
    }

    // ---------- T4 双环：F3 对称不动点 ----------
    #[test]
    fn t4_two_cycle_pagerank() {
        let g = build_graph(&["a", "b"], &[("a", "b"), ("b", "a")]);
        let pr = g.pagerank(100);
        assert!(
            (pr["a"] - 0.5).abs() < EPS,
            "PR(a) 应=0.5，实测 {}",
            pr["a"]
        );
        assert!(
            (pr["b"] - 0.5).abs() < EPS,
            "PR(b) 应=0.5，实测 {}",
            pr["b"]
        );
    }

    // ---------- T3 双团+桥：F6 CNM 社区检测 ----------
    #[test]
    fn t3_two_cliques_communities() {
        let raw: Vec<(&str, &str)> = vec![
            ("a", "b"),
            ("a", "c"),
            ("b", "c"),
            ("d", "e"),
            ("d", "f"),
            ("e", "f"),
            ("b", "d"),
        ];
        let bidi: Vec<(&str, &str)> = raw
            .iter()
            .flat_map(|(a, b)| vec![(*a, *b), (*b, *a)])
            .collect();
        let g = build_graph(&["a", "b", "c", "d", "e", "f"], &bidi);

        // CNM：恰好 2 社区 {a,b,c} + {d,e,f}（LPA 会标签吞并为 1 个）
        let comms = g.detect_communities(50);
        assert_eq!(
            comms.len(),
            2,
            "双团+桥应得 2 社区，实测 {} 个",
            comms.len()
        );
        let mut sets: Vec<Vec<String>> = comms
            .iter()
            .map(|c| {
                let mut v = c.nodes.clone();
                v.sort();
                v
            })
            .collect();
        sets.sort();
        assert_eq!(sets[0], vec!["a", "b", "c"], "社区1 应为 {{a,b,c}}");
        assert_eq!(sets[1], vec!["d", "e", "f"], "社区2 应为 {{d,e,f}}");
    }

    // ---------- T5 孤立图：社区=节点数，中心性全 0 ----------
    #[test]
    fn t5_isolated_graph() {
        let g = build_graph(&["x", "y", "z"], &[]);
        let comms = g.detect_communities(10);
        assert_eq!(comms.len(), 3, "孤立图应 3 社区");
        let deg = g.degree_centrality();
        assert!(deg.values().all(|&v| v.abs() < EPS));
        let btw = g.betweenness_centrality();
        assert!(btw.values().all(|&v| v.abs() < EPS));
    }

    // ---------- T6 有向星型：介数中心性=0（无路径经过中心） ----------
    #[test]
    fn t6_directed_star_betweenness() {
        let g = build_graph(
            &["c", "s1", "s2", "s3", "s4"],
            &[("c", "s1"), ("c", "s2"), ("c", "s3"), ("c", "s4")],
        );
        let btw = g.betweenness_centrality();
        assert!(
            btw.values().all(|&v| v.abs() < EPS),
            "有向星型（中心→叶）介数应全 0"
        );
    }

    // ---------- T7 激活扩散意图识别（F8） ----------
    #[test]
    fn t7_intent_detection() {
        let fg = AIFlowGraph::default_config();

        let cases = [
            ("请分析这个图谱的PageRank与社区结构", "graph"),
            ("请深度推理这个问题并逐步分析", "reasoning"),
            ("组织专家联盟会诊这个问题", "expert"),
            ("你好，今天天气怎么样", "chat"),
        ];
        for (q, expected) in cases {
            let r = fg.detect_intent_by_spread(q);
            assert_eq!(
                r.intent, expected,
                "问题 {:?} 应路由 {}，实测 {}（命中 {:?}）",
                q, expected, r.intent, r.matched_keywords
            );
        }
    }

    // ---------- T8 流程图谱自检：结构守恒 + 决策一致性 ----------
    #[test]
    fn t8_flow_graph_integrity() {
        let fg = AIFlowGraph::default_config();
        let stats = fg.stats();

        // 节点守恒：总数 = step + keyword + capability + engine
        let expected_nodes =
            stats.step_nodes + stats.keyword_nodes + stats.capability_nodes + stats.engine_nodes;
        assert_eq!(stats.node_count, expected_nodes, "节点数守恒");

        // 边守恒：总数 = triggers + flows_to + delegates_to + degrades_to
        let expected_edges =
            stats.trigger_edges + stats.flow_edges + stats.delegate_edges + stats.degrade_edges;
        assert_eq!(stats.edge_count, expected_edges, "边数守恒");

        // 流水线：5 步 4 边
        assert_eq!(stats.step_nodes, 5);
        assert_eq!(stats.flow_edges, 4);

        // 意图决策与关键词打分一致性（Top-1 一致）
        let cases = [
            ("请分析这个图谱的PageRank", "graph"),
            ("深度推理", "reasoning"),
            ("专家会诊", "expert"),
        ];
        for (q, expected) in cases {
            let r = fg.detect_intent_by_spread(q);
            assert_eq!(r.intent, expected);
            assert!(r.score > 0.0, "激活值应为正");
        }
    }
}
