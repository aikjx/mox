// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 项目图谱引擎
//!
//! 基于 `mox_kg_core` 的图存储，封装项目需求图谱的领域操作：
//! - 项目 / 需求 / 任务 / 里程碑 / 人员 / 问题 / 文档 / 标签 的 CRUD
//! - 关系建立与解除
//! - 项目进度自动计算
//! - 需求 / 任务依赖链分析
//! - 影响范围分析（修改一个需求会影响哪些任务 / 里程碑）
//! - 人员负载分析
//! - 关键路径识别

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use serde::Serialize;
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{debug, info};

use mox_kg_core::{Edge, QueryResult, TraverseDirection, Vertex};

use crate::schema::*;

// ─── 引擎 ────────────────────────────────────────────────────────────────────

pub struct ProjectGraphEngine {
    /// 内部使用内存图谱存储（P1），P2 可替换为持久化存储
    inner: Arc<RwLock<InnerGraph>>,
}

struct InnerGraph {
    vertices: HashMap<String, Vertex>,
    // 出边索引: source_id -> Vec<Edge>
    out_edges: HashMap<String, Vec<Edge>>,
    // 入边索引: target_id -> Vec<Edge>
    in_edges: HashMap<String, Vec<Edge>>,
}

impl Default for InnerGraph {
    fn default() -> Self {
        Self {
            vertices: HashMap::new(),
            out_edges: HashMap::new(),
            in_edges: HashMap::new(),
        }
    }
}

impl ProjectGraphEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InnerGraph::default())),
        }
    }

    // ─── 项目操作 ────────────────────────────────────────────────────────

    pub async fn create_project(&self, props: ProjectProps) -> String {
        let id = format!("project:{}", props.code.clone());
        let vertex = Vertex::new(&id, entity_types::PROJECT, json!(props));
        self.add_vertex(vertex).await;
        info!("[project-graph] 创建项目: {} ({})", props.name, id);
        id
    }

    pub async fn get_project(&self, project_id: &str) -> Option<(Vertex, ProjectProps)> {
        let v = self.get_vertex(project_id).await?;
        let props: ProjectProps = serde_json::from_value(v.properties.clone()).ok()?;
        Some((v, props))
    }

    pub async fn update_project(&self, project_id: &str, props: ProjectProps) -> bool {
        self.update_vertex_properties(project_id, json!(props)).await
    }

    pub async fn list_projects(&self) -> Vec<(Vertex, ProjectProps)> {
        let g = self.inner.read().await;
        g.vertices
            .values()
            .filter(|v| v.vertex_type == entity_types::PROJECT)
            .filter_map(|v| {
                let props: ProjectProps = serde_json::from_value(v.properties.clone()).ok()?;
                Some((v.clone(), props))
            })
            .collect()
    }

    // ─── 需求操作 ────────────────────────────────────────────────────────

    pub async fn create_requirement(&self, project_id: &str, props: RequirementProps) -> String {
        let id = format!("req:{}", uuid_simple());
        let vertex = Vertex::new(&id, entity_types::REQUIREMENT, json!(props));
        self.add_vertex(vertex).await;
        // 建立 project -[contains]-> requirement 关系
        self.add_edge(Edge::new(
            edge_types::CONTAINS,
            project_id,
            &id,
            json!({"role": "requirement"}),
        ))
        .await;
        // 建立 requirement -[belongs_to]-> project 关系
        self.add_edge(Edge::new(
            edge_types::BELONGS_TO,
            &id,
            project_id,
            json!({}),
        ))
        .await;
        info!("[project-graph] 创建需求: {} -> {}", props.title, project_id);
        id
    }

    pub async fn get_requirement(&self, req_id: &str) -> Option<(Vertex, RequirementProps)> {
        let v = self.get_vertex(req_id).await?;
        let props: RequirementProps = serde_json::from_value(v.properties.clone()).ok()?;
        Some((v, props))
    }

    pub async fn update_requirement(&self, req_id: &str, props: RequirementProps) -> bool {
        self.update_vertex_properties(req_id, json!(props)).await
    }

    pub async fn list_requirements(&self, project_id: &str) -> Vec<(Vertex, RequirementProps)> {
        let edges = self.get_out_edges(project_id, Some(edge_types::CONTAINS)).await;
        let mut result = Vec::new();
        for edge in edges {
            if let Some((v, props)) = self.get_requirement(&edge.target).await {
                result.push((v, props));
            }
        }
        result
    }

    // ─── 任务操作 ────────────────────────────────────────────────────────

    pub async fn create_task(&self, parent_id: &str, parent_type: &str, props: TaskProps) -> String {
        let id = format!("task:{}", uuid_simple());
        let vertex = Vertex::new(&id, entity_types::TASK, json!(props));
        self.add_vertex(vertex).await;

        match parent_type {
            "requirement" => {
                // 需求 -[拆解为]-> 任务
                self.add_edge(Edge::new(
                    edge_types::DECOMPOSES_INTO,
                    parent_id,
                    &id,
                    json!({}),
                ))
                .await;
            }
            "project" => {
                // 项目 -[包含]-> 任务
                self.add_edge(Edge::new(
                    edge_types::CONTAINS,
                    parent_id,
                    &id,
                    json!({"role": "task"}),
                ))
                .await;
            }
            _ => {}
        }

        // 任务 -[属于]-> 项目（如果 parent 是需求，需要先找到项目）
        if parent_type == "requirement" {
            if let Some(project_id) = self.find_project_of_entity(parent_id).await {
                self.add_edge(Edge::new(
                    edge_types::BELONGS_TO,
                    &id,
                    &project_id,
                    json!({}),
                ))
                .await;
            }
        } else if parent_type == "project" {
            self.add_edge(Edge::new(
                edge_types::BELONGS_TO,
                &id,
                parent_id,
                json!({}),
            ))
            .await;
        }

        info!("[project-graph] 创建任务: {} (parent: {})", props.title, parent_id);
        id
    }

    pub async fn get_task(&self, task_id: &str) -> Option<(Vertex, TaskProps)> {
        let v = self.get_vertex(task_id).await?;
        let props: TaskProps = serde_json::from_value(v.properties.clone()).ok()?;
        Some((v, props))
    }

    pub async fn update_task(&self, task_id: &str, props: TaskProps) -> bool {
        let result = self.update_vertex_properties(task_id, json!(props)).await;
        if result {
            // 触发项目进度重算
            if let Some(project_id) = self.find_project_of_entity(task_id).await {
                self.recalc_project_progress(&project_id).await;
            }
        }
        result
    }

    pub async fn list_tasks_of_requirement(&self, req_id: &str) -> Vec<(Vertex, TaskProps)> {
        let edges = self.get_out_edges(req_id, Some(edge_types::DECOMPOSES_INTO)).await;
        let mut result = Vec::new();
        for edge in edges {
            if let Some((v, props)) = self.get_task(&edge.target).await {
                result.push((v, props));
            }
        }
        result
    }

    pub async fn list_tasks_of_project(&self, project_id: &str) -> Vec<(Vertex, TaskProps)> {
        // 方式一：直接 contains 的任务
        let mut task_ids = HashSet::new();
        let contains_edges = self.get_out_edges(project_id, Some(edge_types::CONTAINS)).await;
        for e in &contains_edges {
            // 需要判断目标是不是 task 类型
            if e.target.starts_with("task:") {
                task_ids.insert(e.target.clone());
            }
        }
        // 方式二：通过需求间接关联的任务
        let reqs = self.list_requirements(project_id).await;
        for (req_v, _) in reqs {
            let decomp_edges = self
                .get_out_edges(&req_v.id, Some(edge_types::DECOMPOSES_INTO))
                .await;
            for e in decomp_edges {
                task_ids.insert(e.target);
            }
        }

        let mut result = Vec::new();
        for tid in task_ids {
            if let Some((v, props)) = self.get_task(&tid).await {
                result.push((v, props));
            }
        }
        result
    }

    // ─── 人员操作 ────────────────────────────────────────────────────────

    pub async fn create_person(&self, props: PersonProps) -> String {
        let id = format!("person:{}", slugify(&props.name));
        let vertex = Vertex::new(&id, entity_types::PERSON, json!(props));
        self.add_vertex(vertex).await;
        id
    }

    pub async fn get_person(&self, person_id: &str) -> Option<(Vertex, PersonProps)> {
        let v = self.get_vertex(person_id).await?;
        let props: PersonProps = serde_json::from_value(v.properties.clone()).ok()?;
        Some((v, props))
    }

    pub async fn assign_task(&self, task_id: &str, person_id: &str) -> bool {
        // 先更新任务属性
        if let Some((_, mut props)) = self.get_task(task_id).await {
            props.assignee_id = Some(person_id.to_string());
            self.update_task(task_id, props).await;
        }
        // 建立关系
        self.add_edge(Edge::new(
            edge_types::ASSIGNED_TO,
            task_id,
            person_id,
            json!({}),
        ))
        .await;
        true
    }

    pub async fn list_person_tasks(&self, person_id: &str, status_filter: Option<TaskStatus>) -> Vec<(Vertex, TaskProps)> {
        let edges = self.get_in_edges(person_id, Some(edge_types::ASSIGNED_TO)).await;
        let mut result = Vec::new();
        for edge in edges {
            if let Some((v, props)) = self.get_task(&edge.source).await {
                if let Some(s) = status_filter {
                    if props.status != s {
                        continue;
                    }
                }
                result.push((v, props));
            }
        }
        result
    }

    // ─── 里程碑操作 ──────────────────────────────────────────────────────

    pub async fn create_milestone(&self, project_id: &str, props: MilestoneProps) -> String {
        let id = format!("milestone:{}", uuid_simple());
        let vertex = Vertex::new(&id, entity_types::MILESTONE, json!(props));
        self.add_vertex(vertex).await;
        self.add_edge(Edge::new(
            edge_types::CONTAINS,
            project_id,
            &id,
            json!({"role": "milestone"}),
        ))
        .await;
        id
    }

    pub async fn get_milestone(&self, ms_id: &str) -> Option<(Vertex, MilestoneProps)> {
        let v = self.get_vertex(ms_id).await?;
        let props: MilestoneProps = serde_json::from_value(v.properties.clone()).ok()?;
        Some((v, props))
    }

    pub async fn link_requirement_to_milestone(&self, ms_id: &str, req_id: &str) {
        self.add_edge(Edge::new(
            edge_types::TRACKS,
            ms_id,
            req_id,
            json!({"type": "requirement"}),
        ))
        .await;
    }

    // ─── 问题 / 风险 ─────────────────────────────────────────────────────

    pub async fn create_issue(&self, project_id: &str, props: IssueProps) -> String {
        let id = format!("issue:{}", uuid_simple());
        let vertex = Vertex::new(&id, entity_types::ISSUE, json!(props));
        self.add_vertex(vertex).await;
        self.add_edge(Edge::new(
            edge_types::BELONGS_TO,
            &id,
            project_id,
            json!({}),
        ))
        .await;
        id
    }

    pub async fn link_issue_to(&self, issue_id: &str, target_id: &str) {
        self.add_edge(Edge::new(
            edge_types::RELATED_TO,
            issue_id,
            target_id,
            json!({}),
        ))
        .await;
    }

    // ─── 文档操作 ────────────────────────────────────────────────────────

    pub async fn create_document(&self, props: DocumentProps) -> String {
        let id = format!("doc:{}", uuid_simple());
        let vertex = Vertex::new(&id, entity_types::DOCUMENT, json!(props));
        self.add_vertex(vertex).await;
        id
    }

    pub async fn link_document_to(&self, doc_id: &str, target_id: &str) {
        self.add_edge(Edge::new(
            edge_types::DESCRIBES,
            doc_id,
            target_id,
            json!({}),
        ))
        .await;
    }

    // ─── 依赖关系 ────────────────────────────────────────────────────────

    pub async fn add_dependency(&self, from_id: &str, to_id: &str) {
        // from 依赖 to（from 必须等 to 完成才能开始）
        self.add_edge(Edge::new(
            edge_types::DEPENDS_ON,
            from_id,
            to_id,
            json!({}),
        ))
        .await;
    }

    pub async fn add_blocker(&self, blocker_id: &str, blocked_id: &str) {
        // blocker 阻塞 blocked
        self.add_edge(Edge::new(
            edge_types::BLOCKS,
            blocker_id,
            blocked_id,
            json!({}),
        ))
        .await;
    }

    // ─── 进度计算 ────────────────────────────────────────────────────────

    /// 重新计算项目整体进度（基于需求和任务的加权平均）
    pub async fn recalc_project_progress(&self, project_id: &str) -> f32 {
        let reqs = self.list_requirements(project_id).await;
        let tasks = self.list_tasks_of_project(project_id).await;

        if reqs.is_empty() && tasks.is_empty() {
            return 0.0;
        }

        // 需求权重占 60%，任务权重占 40%
        let req_progress = if reqs.is_empty() {
            0.0
        } else {
            let total: f32 = reqs
                .iter()
                .map(|(_, p)| p.priority.weight() as f32)
                .sum();
            let weighted: f32 = reqs
                .iter()
                .map(|(_, p)| p.status.progress_weight() * p.priority.weight() as f32)
                .sum();
            if total > 0.0 { weighted / total } else { 0.0 }
        };

        let task_progress = if tasks.is_empty() {
            0.0
        } else {
            let total: f32 = tasks
                .iter()
                .map(|(_, p)| p.priority.weight() as f32)
                .sum();
            let weighted: f32 = tasks
                .iter()
                .map(|(_, p)| p.status.progress_weight() * p.priority.weight() as f32)
                .sum();
            if total > 0.0 { weighted / total } else { 0.0 }
        };

        let progress = if !reqs.is_empty() && !tasks.is_empty() {
            req_progress * 0.6 + task_progress * 0.4
        } else if !reqs.is_empty() {
            req_progress
        } else {
            task_progress
        };

        // 更新项目进度
        if let Some((_, mut props)) = self.get_project(project_id).await {
            props.progress = (progress * 100.0).round() / 100.0;
            self.update_project(project_id, props).await;
        }

        debug!("[project-graph] 项目 {} 进度: {:.1}%", project_id, progress * 100.0);
        progress
    }

    // ─── 影响分析 ────────────────────────────────────────────────────────

    /// 分析某个实体（需求/任务）变更会影响到哪些上游节点
    /// 返回所有受影响的实体 ID 列表
    pub async fn analyze_impact(&self, entity_id: &str) -> Vec<String> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(entity_id.to_string());

        while let Some(current) = queue.pop_front() {
            // 找所有"依赖"当前节点的实体（反向遍历 depends_on）
            // 即 incoming 的 depends_on 边的 source
            let in_edges = self.get_in_edges(&current, Some(edge_types::DEPENDS_ON)).await;
            for edge in in_edges {
                if affected.insert(edge.source.clone()) {
                    queue.push_back(edge.source);
                }
            }
            // 找所有被当前节点阻塞的
            let out_blocks = self.get_out_edges(&current, Some(edge_types::BLOCKS)).await;
            for edge in out_blocks {
                if affected.insert(edge.target.clone()) {
                    queue.push_back(edge.target);
                }
            }
            // 找所属的父需求 / 项目
            let belongs = self.get_out_edges(&current, Some(edge_types::BELONGS_TO)).await;
            for edge in belongs {
                affected.insert(edge.target.clone());
                // 不继续往上走项目，项目是汇总节点
            }
        }

        affected.into_iter().collect()
    }

    // ─── 人员负载分析 ────────────────────────────────────────────────────

    pub async fn person_workload(&self, person_id: &str) -> PersonWorkload {
        let all_tasks = self.list_person_tasks(person_id, None).await;

        let mut total_est = 0.0;
        let mut total_actual = 0.0;
        let mut in_progress = 0;
        let mut todo = 0;
        let mut completed = 0;
        let mut blocked = 0;
        let mut p0_count = 0;
        let mut p1_count = 0;

        for (_, props) in &all_tasks {
            if let Some(est) = props.estimate_hours {
                total_est += est;
            }
            if let Some(act) = props.actual_hours {
                total_actual += act;
            }
            match props.status {
                TaskStatus::Todo => todo += 1,
                TaskStatus::InProgress => in_progress += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Blocked => blocked += 1,
                TaskStatus::Cancelled => {}
            }
            match props.priority {
                Priority::P0 => p0_count += 1,
                Priority::P1 => p1_count += 1,
                _ => {}
            }
        }

        PersonWorkload {
            person_id: person_id.to_string(),
            total_tasks: all_tasks.len(),
            todo,
            in_progress,
            completed,
            blocked,
            p0_count,
            p1_count,
            total_estimate_hours: total_est,
            total_actual_hours: total_actual,
        }
    }

    // ─── 关键路径 ────────────────────────────────────────────────────────

    /// 找出项目的关键路径（最长依赖链）
    pub async fn critical_path(&self, project_id: &str) -> Vec<String> {
        let tasks = self.list_tasks_of_project(project_id).await;
        if tasks.is_empty() {
            return Vec::new();
        }

        // 拓扑排序 + 最长路径
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new(); // task -> 被它阻塞的 tasks
        let mut duration: HashMap<String, f32> = HashMap::new();

        for (v, props) in &tasks {
            let dur = props.estimate_hours.unwrap_or(1.0);
            duration.insert(v.id.clone(), dur);
            in_degree.entry(v.id.clone()).or_insert(0);
            adj.entry(v.id.clone()).or_default();
        }

        // 构建依赖图：A depends_on B => B 完成后 A 才能开始
        // 所以边是 B -> A（B 指向后续任务 A）
        for (v, _) in &tasks {
            let deps = self.get_out_edges(&v.id, Some(edge_types::DEPENDS_ON)).await;
            for dep_edge in deps {
                // v depends_on dep_edge.target
                // => dep_edge.target -> v
                adj.entry(dep_edge.target.clone())
                    .or_default()
                    .push(v.id.clone());
                *in_degree.entry(v.id.clone()).or_insert(0) += 1;
            }
        }

        // 拓扑排序
        let mut dist: HashMap<String, f32> = HashMap::new();
        let mut prev: HashMap<String, Option<String>> = HashMap::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        for (id, &deg) in &in_degree {
            if deg == 0 {
                dist.insert(id.clone(), *duration.get(id).unwrap_or(&0.0));
                prev.insert(id.clone(), None);
                queue.push_back(id.clone());
            }
        }

        let mut topo_order = Vec::new();
        while let Some(u) = queue.pop_front() {
            topo_order.push(u.clone());
            if let Some(neighbors) = adj.get(&u) {
                for v in neighbors {
                    let new_dist = dist.get(&u).unwrap_or(&0.0) + duration.get(v).unwrap_or(&0.0);
                    if new_dist > *dist.get(v).unwrap_or(&0.0) {
                        dist.insert(v.clone(), new_dist);
                        prev.insert(v.clone(), Some(u.clone()));
                    }
                    let deg = in_degree.get_mut(v).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(v.clone());
                    }
                }
            }
        }

        // 找最长路径终点
        let mut end_node = String::new();
        let mut max_dist = 0.0;
        for (id, &d) in &dist {
            if d > max_dist {
                max_dist = d;
                end_node = id.clone();
            }
        }

        // 回溯路径
        let mut path = Vec::new();
        let mut cur = Some(end_node);
        while let Some(node) = cur {
            path.push(node.clone());
            cur = prev.get(&node).cloned().flatten();
        }
        path.reverse();

        path
    }

    // ─── 图谱遍历（底层） ────────────────────────────────────────────────

    pub async fn traverse(
        &self,
        start_id: &str,
        direction: TraverseDirection,
        edge_types: Option<Vec<String>>,
        max_depth: usize,
    ) -> QueryResult {
        let g = self.inner.read().await;
        let mut result = QueryResult::success("traverse");

        let mut visited = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        if let Some(start_v) = g.vertices.get(start_id) {
            result.vertices.push(start_v.clone());
            visited.insert(start_id.to_string());
            queue.push_back((start_id.to_string(), 0));
        } else {
            return QueryResult::error("traverse", format!("起点不存在: {}", start_id));
        }

        while let Some((cur_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let edges_to_collect = match direction {
                TraverseDirection::Out => g.out_edges.get(&cur_id).cloned().unwrap_or_default(),
                TraverseDirection::In => g.in_edges.get(&cur_id).cloned().unwrap_or_default(),
                TraverseDirection::Both => {
                    let mut out = g.out_edges.get(&cur_id).cloned().unwrap_or_default();
                    let mut inn = g.in_edges.get(&cur_id).cloned().unwrap_or_default();
                    out.append(&mut inn);
                    out
                }
            };

            for edge in edges_to_collect {
                // 过滤边类型
                if let Some(ref ets) = edge_types {
                    if !ets.contains(&edge.edge_type) {
                        continue;
                    }
                }

                result.edges.push(edge.clone());

                let neighbor = if direction == TraverseDirection::In {
                    edge.source.clone()
                } else {
                    edge.target.clone()
                };

                if !visited.contains(&neighbor) {
                    if let Some(v) = g.vertices.get(&neighbor) {
                        result.vertices.push(v.clone());
                        visited.insert(neighbor.clone());
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        result.total = result.vertices.len();
        result
    }

    // ─── 统计信息 ────────────────────────────────────────────────────────

    pub async fn project_stats(&self, project_id: &str) -> ProjectStats {
        let reqs = self.list_requirements(project_id).await;
        let tasks = self.list_tasks_of_project(project_id).await;
        let progress = self.recalc_project_progress(project_id).await;

        // 按状态统计
        let mut req_by_status: HashMap<String, usize> = HashMap::new();
        for (_, r) in &reqs {
            *req_by_status
                .entry(format!("{:?}", r.status).to_lowercase())
                .or_insert(0) += 1;
        }

        let mut task_by_status: HashMap<String, usize> = HashMap::new();
        for (_, t) in &tasks {
            *task_by_status
                .entry(format!("{:?}", t.status).to_lowercase())
                .or_insert(0) += 1;
        }

        // 统计问题
        let issues_edges = self.get_in_edges(project_id, Some(edge_types::BELONGS_TO)).await;
        let issue_count = issues_edges
            .iter()
            .filter(|e| e.source.starts_with("issue:"))
            .count();

        // 参与人数
        let mut person_ids = HashSet::new();
        for (_, t) in &tasks {
            if let Some(ref aid) = t.assignee_id {
                person_ids.insert(aid.clone());
            }
        }

        ProjectStats {
            project_id: project_id.to_string(),
            requirement_count: reqs.len(),
            task_count: tasks.len(),
            issue_count,
            member_count: person_ids.len(),
            progress,
            requirements_by_status: req_by_status,
            tasks_by_status: task_by_status,
        }
    }

    // ─── 底层图操作 ──────────────────────────────────────────────────────

    async fn add_vertex(&self, vertex: Vertex) {
        let mut g = self.inner.write().await;
        g.vertices.insert(vertex.id.clone(), vertex);
    }

    async fn get_vertex(&self, id: &str) -> Option<Vertex> {
        let g = self.inner.read().await;
        g.vertices.get(id).cloned()
    }

    async fn update_vertex_properties(&self, id: &str, properties: serde_json::Value) -> bool {
        let mut g = self.inner.write().await;
        if let Some(v) = g.vertices.get_mut(id) {
            v.properties = properties;
            v.updated_at = chrono::Utc::now().to_rfc3339();
            true
        } else {
            false
        }
    }

    async fn add_edge(&self, edge: Edge) {
        let mut g = self.inner.write().await;
        g.out_edges
            .entry(edge.source.clone())
            .or_default()
            .push(edge.clone());
        g.in_edges
            .entry(edge.target.clone())
            .or_default()
            .push(edge);
    }

    async fn get_out_edges(&self, vertex_id: &str, edge_type: Option<&str>) -> Vec<Edge> {
        let g = self.inner.read().await;
        match g.out_edges.get(vertex_id) {
            Some(edges) => match edge_type {
                Some(et) => edges.iter().filter(|e| e.edge_type == et).cloned().collect(),
                None => edges.clone(),
            },
            None => Vec::new(),
        }
    }

    async fn get_in_edges(&self, vertex_id: &str, edge_type: Option<&str>) -> Vec<Edge> {
        let g = self.inner.read().await;
        match g.in_edges.get(vertex_id) {
            Some(edges) => match edge_type {
                Some(et) => edges.iter().filter(|e| e.edge_type == et).cloned().collect(),
                None => edges.clone(),
            },
            None => Vec::new(),
        }
    }

    /// 找出实体所属的项目
    pub async fn find_project_of_entity(&self, entity_id: &str) -> Option<String> {
        let edges = self.get_out_edges(entity_id, Some(edge_types::BELONGS_TO)).await;
        edges
            .into_iter()
            .find(|e| e.target.starts_with("project:"))
            .map(|e| e.target)
    }

    /// 获取出边（供服务层调用）
    pub async fn get_out_edges_for_svc(&self, vertex_id: &str, edge_type: Option<&str>) -> Vec<Edge> {
        self.get_out_edges(vertex_id, edge_type).await
    }

    /// 获取入边（供服务层调用）
    pub async fn get_in_edges_for_svc(&self, vertex_id: &str, edge_type: Option<&str>) -> Vec<Edge> {
        self.get_in_edges(vertex_id, edge_type).await
    }
}

impl Default for ProjectGraphEngine {
    fn default() -> Self { Self::new() }
}

// ─── 统计数据结构 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStats {
    pub project_id: String,
    pub requirement_count: usize,
    pub task_count: usize,
    pub issue_count: usize,
    pub member_count: usize,
    pub progress: f32,
    pub requirements_by_status: HashMap<String, usize>,
    pub tasks_by_status: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonWorkload {
    pub person_id: String,
    pub total_tasks: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub blocked: usize,
    pub p0_count: usize,
    pub p1_count: usize,
    pub total_estimate_hours: f32,
    pub total_actual_hours: f32,
}

// ─── 工具函数 ────────────────────────────────────────────────────────────────

fn uuid_simple() -> String {
    // 简单的 8 位随机 ID（P1 够用）
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // 用时间戳低 32 位 + 随机
    let rand = (nanos & 0xFFFFFFFF) as u32;
    format!("{:08x}", rand)
}

fn slugify(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase()
}
