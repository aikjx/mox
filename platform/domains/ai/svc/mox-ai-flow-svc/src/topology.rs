// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 六维实体关系拓扑网
//!
//! 实体：LLM模型 / 工具 / 记忆块 / Skill技能 / 流程图节点 / 业务专家规则
//!
//! 提供三个原生 Agent 不具备的能力：
//! 1. **最短路径检索**：语音指令 → 图上加权检索，命中历史 Skill 模板则跳过完整 ReAct 推理；
//! 2. **权重动态衰减**：高频关联升权、低频自动衰减归档，控制记忆规模不无限膨胀；
//! 3. **实体联动更新**：改一个流程节点，级联标脏绑定的工具/Skill/代码模板/规则。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};

/// 实体六维分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Model,
    Tool,
    Memory,
    Skill,
    FlowNode,
    Rule,
}

/// 关系类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// 流程节点 → 工具
    Binds,
    /// Skill → 记忆块
    Recalls,
    /// 规则 → 流程分支
    Constrains,
    /// 模型 → 任务类型
    Serves,
    /// Skill → 流程节点（模板实例化）
    Implements,
    /// 通用语义相似
    Similar,
}

impl RelationKind {
    /// 关系基础代价（越小越优先走）
    fn base_cost(&self) -> f64 {
        match self {
            RelationKind::Implements => 0.5,
            RelationKind::Binds => 0.8,
            RelationKind::Recalls => 1.0,
            RelationKind::Serves => 1.0,
            RelationKind::Constrains => 1.2,
            RelationKind::Similar => 1.5,
        }
    }
}

/// 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub kind: EntityKind,
    pub label: String,
    /// 语义关键词（用于指令匹配）
    #[serde(default)]
    pub keywords: Vec<String>,
    /// 使用次数
    #[serde(default)]
    pub hits: u64,
    /// 动态权重（衰减后的活跃度，0.0~+）
    #[serde(default = "one")]
    pub weight: f64,
    /// 是否已归档
    #[serde(default)]
    pub archived: bool,
    /// 执行该实体的预估开销（用于最短路径的代价）
    #[serde(default)]
    pub cost_ms: u64,
}

fn one() -> f64 {
    1.0
}

impl Entity {
    pub fn new(id: impl Into<String>, kind: EntityKind, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            label: label.into(),
            keywords: Vec::new(),
            hits: 0,
            weight: 1.0,
            archived: false,
            cost_ms: 0,
        }
    }
    pub fn with_keywords<I: IntoIterator<Item = S>, S: Into<String>>(mut self, kw: I) -> Self {
        self.keywords = kw.into_iter().map(|s| s.into()).collect();
        self
    }
    pub fn with_cost(mut self, ms: u64) -> Self {
        self.cost_ms = ms;
        self
    }
}

/// 关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    /// 关联强度 0..1，越大越紧密
    pub strength: f64,
}

impl Relation {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: RelationKind,
        strength: f64,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            kind,
            strength: strength.clamp(0.001, 1.0),
        }
    }
}

/// 检索命中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub entity_id: String,
    pub kind: EntityKind,
    pub label: String,
    /// 综合得分 = 语义相似 × 动态权重
    pub score: f64,
    pub matched_keywords: Vec<String>,
}

/// 路径检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    /// 命中的入口实体
    pub entry: Option<Match>,
    /// 最低代价执行路径（实体 id 序列）
    pub path: Vec<String>,
    /// 路径总代价
    pub cost: f64,
    /// 是否可跳过完整 ReAct 推理
    pub fast_path: bool,
    /// 说明
    pub rationale: String,
}

/// 级联影响分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactSet {
    pub origin: String,
    /// 按实体类型分组的受影响实体
    pub affected: BTreeMap<String, Vec<String>>,
    pub total: usize,
}

/// 六维关系拓扑图
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

impl TopologyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, e: Entity) -> &mut Self {
        self.entities.push(e);
        self
    }

    pub fn add_relation(&mut self, r: Relation) -> &mut Self {
        self.relations.push(r);
        self
    }

    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.entities.iter().position(|e| e.id == id)
    }

    pub fn active_count(&self) -> usize {
        self.entities.iter().filter(|e| !e.archived).count()
    }

    /// 从流程图自动导入实体与关系（流程节点 ↔ 工具 ↔ 规则）
    pub fn ingest_flow(&mut self, flow: &crate::model::FlowGraph) {
        for n in &flow.nodes {
            let eid = format!("flow:{}:{}", flow.id, n.id);
            if self.entity(&eid).is_none() {
                let mut e = Entity::new(eid.clone(), EntityKind::FlowNode, n.name.clone())
                    .with_cost(n.duration_ms);
                e.keywords = tokenize(&n.name);
                e.keywords.extend(n.tags.clone());
                self.add_entity(e);
            }
            if let Some(tool) = n.tool {
                let tid = format!("tool:{:?}", tool).to_lowercase();
                if self.entity(&tid).is_none() {
                    self.add_entity(Entity::new(
                        tid.clone(),
                        EntityKind::Tool,
                        format!("{:?}", tool),
                    ));
                }
                self.add_relation(Relation::new(eid.clone(), tid, RelationKind::Binds, 0.9));
            }
        }
        for r in &flow.rules {
            let rid = format!("rule:{}", r.id);
            if self.entity(&rid).is_none() {
                let mut e = Entity::new(rid.clone(), EntityKind::Rule, r.description.clone());
                e.keywords = tokenize(&r.description);
                self.add_entity(e);
            }
            for n in &flow.nodes {
                if n.accesses.iter().any(|a| {
                    r.resource_prefixes
                        .iter()
                        .any(|p| a.resource.starts_with(p.as_str()))
                }) {
                    self.add_relation(Relation::new(
                        rid.clone(),
                        format!("flow:{}:{}", flow.id, n.id),
                        RelationKind::Constrains,
                        1.0,
                    ));
                }
            }
        }
    }

    /// 语义检索：把自然语言/语音指令匹配到实体
    pub fn search(&self, query: &str, top_k: usize) -> Vec<Match> {
        let q = tokenize(query);
        let qset: HashSet<&str> = q.iter().map(|s| s.as_str()).collect();
        let mut scored: Vec<Match> = self
            .entities
            .iter()
            .filter(|e| !e.archived)
            .filter_map(|e| {
                let matched: Vec<String> = e
                    .keywords
                    .iter()
                    .filter(|k| qset.contains(k.as_str()) || query.contains(k.as_str()))
                    .cloned()
                    .collect();
                if matched.is_empty() {
                    return None;
                }
                // Jaccard-ish 相似度 × 动态权重
                let denom = (e.keywords.len() + q.len()).max(1) as f64;
                let sim = 2.0 * matched.len() as f64 / denom;
                Some(Match {
                    entity_id: e.id.clone(),
                    kind: e.kind,
                    label: e.label.clone(),
                    score: sim * e.weight,
                    matched_keywords: matched,
                })
            })
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(top_k);
        scored
    }

    /// 加权最短路径（Dijkstra）
    ///
    /// 边代价 = 关系基础代价 / 强度 + 目标实体归一化执行开销 / (1 + 动态权重)
    /// → 强关联、高频复用、低开销的路径被优先选中。
    pub fn shortest_path(&self, from: &str, to: &str) -> Option<(Vec<String>, f64)> {
        let (s, t) = (self.index_of(from)?, self.index_of(to)?);
        let n = self.entities.len();
        let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for r in &self.relations {
            let (Some(u), Some(v)) = (self.index_of(&r.from), self.index_of(&r.to)) else {
                continue;
            };
            if self.entities[v].archived {
                continue;
            }
            adj[u].push((v, self.edge_cost(r, v)));
        }

        let mut dist = vec![f64::INFINITY; n];
        let mut prev = vec![usize::MAX; n];
        dist[s] = 0.0;
        let mut heap: BinaryHeap<(OrderedF64, usize)> = BinaryHeap::new();
        heap.push((OrderedF64(-0.0), s));
        while let Some((OrderedF64(negd), u)) = heap.pop() {
            let d = -negd;
            if d > dist[u] + 1e-12 {
                continue;
            }
            if u == t {
                break;
            }
            for &(v, w) in &adj[u] {
                let nd = d + w;
                if nd < dist[v] - 1e-12 {
                    dist[v] = nd;
                    prev[v] = u;
                    heap.push((OrderedF64(-nd), v));
                }
            }
        }
        if dist[t].is_infinite() {
            return None;
        }
        let mut path = vec![t];
        let mut cur = t;
        while prev[cur] != usize::MAX {
            cur = prev[cur];
            path.push(cur);
        }
        path.reverse();
        Some((
            path.into_iter()
                .map(|i| self.entities[i].id.clone())
                .collect(),
            dist[t],
        ))
    }

    fn edge_cost(&self, r: &Relation, target: usize) -> f64 {
        let e = &self.entities[target];
        let structural = r.kind.base_cost() / r.strength;
        let exec = (e.cost_ms as f64 / 1000.0) / (1.0 + e.weight);
        structural + exec
    }

    /// 指令路由：优先走图谱复用路径，失败才回退完整推理
    ///
    /// `fast_path_threshold`：命中得分阈值，超过即认为可复用历史 Skill。
    pub fn route(&self, query: &str, fast_path_threshold: f64) -> RoutePlan {
        let hits = self.search(query, 5);
        let Some(best) = hits.first().cloned() else {
            return RoutePlan {
                entry: None,
                path: Vec::new(),
                cost: f64::INFINITY,
                fast_path: false,
                rationale: "图谱无匹配实体，回退完整 ReAct 推理".into(),
            };
        };

        // 优先找 Skill 类实体作为可复用模板
        let skill = hits
            .iter()
            .find(|m| m.kind == EntityKind::Skill)
            .cloned()
            .unwrap_or_else(|| best.clone());

        // 从 Skill 出发找到其实现的流程终点，形成可执行路径
        let targets: Vec<&Entity> = self
            .entities
            .iter()
            .filter(|e| e.kind == EntityKind::FlowNode && !e.archived)
            .collect();
        let mut best_path: Option<(Vec<String>, f64)> = None;
        for tgt in targets {
            if let Some((p, c)) = self.shortest_path(&skill.entity_id, &tgt.id) {
                if best_path.as_ref().map(|(_, bc)| c > *bc).unwrap_or(false) {
                    // 取代价最大的终点 = 最完整的流程链
                    continue;
                }
                if best_path.is_none() || p.len() > best_path.as_ref().unwrap().0.len() {
                    best_path = Some((p, c));
                }
            }
        }

        let (path, cost) = best_path.unwrap_or_else(|| (vec![skill.entity_id.clone()], 0.0));
        let fast = skill.kind == EntityKind::Skill && skill.score >= fast_path_threshold;
        RoutePlan {
            rationale: if fast {
                format!(
                    "命中历史技能 `{}`（得分 {:.2} ≥ 阈值 {:.2}），走图谱最短路径，跳过完整推理",
                    skill.label, skill.score, fast_path_threshold
                )
            } else {
                format!(
                    "最佳匹配 `{}` 得分 {:.2} 低于阈值 {:.2}，需完整 ReAct 推理兜底",
                    skill.label, skill.score, fast_path_threshold
                )
            },
            entry: Some(skill),
            path,
            cost,
            fast_path: fast,
        }
    }

    /// 记录一次使用，提升权重
    pub fn record_hit(&mut self, id: &str) {
        if let Some(e) = self.entity_mut(id) {
            e.hits += 1;
            // 对数增长，避免单点权重爆炸
            e.weight = (e.weight + 1.0 / (1.0 + e.weight)).min(10.0);
            e.archived = false;
        }
    }

    /// 权重动态衰减 + 自动归档
    ///
    /// `decay`：每轮衰减系数 (0,1)；`archive_below`：低于该权重自动归档。
    /// 返回本轮归档数量。
    pub fn decay(&mut self, decay: f64, archive_below: f64) -> usize {
        let d = decay.clamp(0.0, 1.0);
        let mut archived = 0;
        for e in self.entities.iter_mut() {
            if e.archived {
                continue;
            }
            e.weight *= d;
            if e.weight < archive_below {
                e.archived = true;
                archived += 1;
            }
        }
        archived
    }

    /// 级联影响分析：修改某实体后哪些实体需要同步更新
    pub fn impact_of(&self, origin: &str) -> ImpactSet {
        let mut seen: HashSet<String> = HashSet::new();
        let mut stack = vec![origin.to_string()];
        seen.insert(origin.to_string());
        // 双向传播：流程节点改了，其绑定工具、约束规则、实现它的 Skill 都要标脏
        let mut fwd: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut bwd: HashMap<&str, Vec<&str>> = HashMap::new();
        for r in &self.relations {
            fwd.entry(r.from.as_str()).or_default().push(r.to.as_str());
            bwd.entry(r.to.as_str()).or_default().push(r.from.as_str());
        }
        while let Some(cur) = stack.pop() {
            for m in [fwd.get(cur.as_str()), bwd.get(cur.as_str())]
                .into_iter()
                .flatten()
            {
                for nxt in m.iter() {
                    if seen.insert(nxt.to_string()) {
                        stack.push(nxt.to_string());
                    }
                }
            }
        }
        seen.remove(origin);
        let mut affected: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for id in &seen {
            if let Some(e) = self.entity(id) {
                affected
                    .entry(format!("{:?}", e.kind))
                    .or_default()
                    .push(id.clone());
            }
        }
        for v in affected.values_mut() {
            v.sort();
        }
        ImpactSet {
            origin: origin.to_string(),
            total: seen.len(),
            affected,
        }
    }
}

fn tokenize(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            if ch.is_ascii() {
                buf.push(ch.to_ascii_lowercase());
            } else {
                // 中文按字切分 + 相邻二元组，提升短指令召回
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
                out.push(ch.to_string());
            }
        } else if !buf.is_empty() {
            out.push(std::mem::take(&mut buf));
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    // 中文二元组
    let cjk: Vec<&String> = out
        .iter()
        .filter(|t| t.chars().count() == 1 && !t.is_ascii())
        .collect();
    let bigrams: Vec<String> = cjk
        .windows(2)
        .map(|w| format!("{}{}", w[0], w[1]))
        .collect();
    out.extend(bigrams);
    out.sort();
    out.dedup();
    out
}

/// f64 的全序包装（用于 BinaryHeap）
#[derive(PartialEq)]
struct OrderedF64(f64);
impl Eq for OrderedF64 {}
impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> TopologyGraph {
        let mut g = TopologyGraph::new();
        g.add_entity(
            Entity::new("skill:report", EntityKind::Skill, "月度报表生成").with_keywords([
                "报表",
                "月度",
                "月度报表",
                "生成",
            ]),
        );
        g.add_entity(Entity::new("flow:n1", EntityKind::FlowNode, "读取Excel").with_cost(300));
        g.add_entity(Entity::new("flow:n2", EntityKind::FlowNode, "汇总输出").with_cost(100));
        g.add_entity(Entity::new("tool:file", EntityKind::Tool, "File"));
        g.add_entity(Entity::new("mem:last", EntityKind::Memory, "上次执行记录"));
        g.add_relation(Relation::new(
            "skill:report",
            "flow:n1",
            RelationKind::Implements,
            1.0,
        ));
        g.add_relation(Relation::new(
            "flow:n1",
            "flow:n2",
            RelationKind::Implements,
            1.0,
        ));
        g.add_relation(Relation::new(
            "flow:n1",
            "tool:file",
            RelationKind::Binds,
            0.9,
        ));
        g.add_relation(Relation::new(
            "skill:report",
            "mem:last",
            RelationKind::Recalls,
            0.7,
        ));
        g
    }

    #[test]
    fn search_finds_skill() {
        let g = demo();
        let hits = g.search("帮我生成月度报表", 3);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].entity_id, "skill:report");
    }

    #[test]
    fn shortest_path_reaches_flow_tail() {
        let g = demo();
        let (path, cost) = g.shortest_path("skill:report", "flow:n2").unwrap();
        assert_eq!(path, vec!["skill:report", "flow:n1", "flow:n2"]);
        assert!(cost > 0.0);
    }

    #[test]
    fn route_takes_fast_path_on_strong_hit() {
        let g = demo();
        let plan = g.route("生成月度报表", 0.1);
        assert!(plan.fast_path, "{}", plan.rationale);
        assert!(plan.path.contains(&"flow:n1".to_string()));
    }

    #[test]
    fn route_falls_back_when_no_match() {
        let g = demo();
        let plan = g.route("给我讲个笑话", 0.1);
        assert!(!plan.fast_path);
        assert!(plan.entry.is_none());
    }

    #[test]
    fn decay_archives_cold_entities() {
        let mut g = demo();
        g.record_hit("skill:report");
        let before = g.active_count();
        // 多轮衰减，只有被反复命中的实体存活
        for _ in 0..10 {
            g.record_hit("skill:report");
            g.decay(0.5, 0.1);
        }
        assert!(g.active_count() < before, "冷实体应被归档");
        assert!(!g.entity("skill:report").unwrap().archived, "热实体应保留");
    }

    #[test]
    fn impact_propagates_both_directions() {
        let g = demo();
        let imp = g.impact_of("flow:n1");
        // 上游 Skill、下游节点、绑定工具都应被标脏
        let all: Vec<String> = imp.affected.values().flatten().cloned().collect();
        assert!(all.contains(&"skill:report".to_string()));
        assert!(all.contains(&"tool:file".to_string()));
        assert!(all.contains(&"flow:n2".to_string()));
    }

    #[test]
    fn ingest_flow_builds_bindings() {
        use crate::model::{FlowGraph, FlowNode, ToolKind};
        let mut f = FlowGraph::new("f1", "测试流程");
        f.add_node(FlowNode::task("a", "浏览器抓取", ToolKind::Browser, 100));
        let mut g = TopologyGraph::new();
        g.ingest_flow(&f);
        assert!(g.entity("flow:f1:a").is_some());
        assert!(g.entity("tool:browser").is_some());
        assert!(g
            .relations
            .iter()
            .any(|r| r.kind == RelationKind::Binds && r.to == "tool:browser"));
    }
}
