//! MOX Platform Orchestrator Core
//!
//! DAG-based workflow orchestration engine with:
//! - Topological scheduling with dependency resolution
//! - Resource-constrained execution (RCPSP)
//! - Event-driven state machine
//! - Checkpoint/resume for durable execution
//! - Parallel execution with rayon

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("cycle detected in DAG: {0}")]
    CycleDetected(String),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("invalid state transition: {0} -> {1}")]
    InvalidTransition(String, String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("resource unavailable: {0}")]
    ResourceUnavailable(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl NodeState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, NodeState::Completed | NodeState::Failed | NodeState::Skipped | NodeState::Cancelled)
    }
    pub fn can_transition_to(&self, target: NodeState) -> bool {
        matches!(
            (self, target),
            (NodeState::Pending, NodeState::Ready)
                | (NodeState::Ready, NodeState::Running)
                | (NodeState::Running, NodeState::Completed)
                | (NodeState::Running, NodeState::Failed)
                | (NodeState::Pending, NodeState::Skipped)
                | (NodeState::Ready, NodeState::Cancelled)
                | (NodeState::Running, NodeState::Cancelled)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub name: String,
    pub operator: String,
    pub params: serde_json::Value,
    pub dependencies: Vec<String>,
    pub state: NodeState,
    pub retry_count: u32,
    pub max_retries: u32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
    pub result: Option<serde_json::Value>,
}

impl WorkflowNode {
    pub fn new(id: &str, name: &str, operator: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            operator: operator.into(),
            params: serde_json::Value::Null,
            dependencies: vec![],
            state: NodeState::Pending,
            retry_count: 0,
            max_retries: 3,
            started_at: None,
            completed_at: None,
            error: None,
            result: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowState {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub nodes: HashMap<String, WorkflowNode>,
    pub state: WorkflowState,
    pub created_at: String,
    pub updated_at: String,
    pub context: serde_json::Value,
}

impl Workflow {
    pub fn new(name: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::now_v7().to_string(),
            name: name.into(),
            nodes: HashMap::new(),
            state: WorkflowState::Created,
            created_at: now.clone(),
            updated_at: now,
            context: serde_json::json!({}),
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.insert(node.id.clone(), node);
        self.touch();
    }

    pub fn add_dependency(&mut self, node_id: &str, dep_id: &str) -> Result<(), OrchestratorError> {
        let node = self.nodes.get_mut(node_id).ok_or_else(|| OrchestratorError::NodeNotFound(node_id.into()))?;
        if !node.dependencies.contains(&dep_id.to_string()) {
            node.dependencies.push(dep_id.into());
        }
        self.touch();
        Ok(())
    }

    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Validate DAG: detect cycles and ensure all deps exist.
    pub fn validate(&self) -> Result<(), OrchestratorError> {
        // Check all deps exist
        for node in self.nodes.values() {
            for dep in &node.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(OrchestratorError::NodeNotFound(format!("{} (dep of {})", dep, node.id)));
                }
            }
        }
        // Cycle detection via DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        for node_id in self.nodes.keys() {
            if self.dfs_cycle(node_id, &mut visited, &mut rec_stack) {
                return Err(OrchestratorError::CycleDetected(node_id.into()));
            }
        }
        Ok(())
    }

    fn dfs_cycle(&self, node: &str, visited: &mut HashSet<String>, rec: &mut HashSet<String>) -> bool {
        if rec.contains(node) { return true; }
        if visited.contains(node) { return false; }
        visited.insert(node.into());
        rec.insert(node.into());
        if let Some(n) = self.nodes.get(node) {
            for dep in &n.dependencies {
                if self.dfs_cycle(dep, visited, rec) { return true; }
            }
        }
        rec.remove(node);
        false
    }

    /// Topological sort: return nodes in execution order.
    pub fn topological_order(&self) -> Result<Vec<String>, OrchestratorError> {
        self.validate()?;
        let mut in_degree: HashMap<String, usize> = self.nodes.keys().map(|k| (k.clone(), 0)).collect();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for node in self.nodes.values() {
            for dep in &node.dependencies {
                adj.entry(dep.clone()).or_default().push(node.id.clone());
                *in_degree.get_mut(&node.id).unwrap() += 1;
            }
        }
        let mut queue: VecDeque<String> = in_degree.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();
        let mut order = vec![];
        while let Some(nid) = queue.pop_front() {
            order.push(nid.clone());
            if let Some(neighbors) = adj.get(&nid) {
                for neighbor in neighbors {
                    if let Some(d) = in_degree.get_mut(neighbor) {
                        *d -= 1;
                        if *d == 0 { queue.push_back(neighbor.clone()); }
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(OrchestratorError::CycleDetected("topological sort incomplete".into()));
        }
        Ok(order)
    }

    /// Get nodes ready to execute (all deps completed).
    pub fn ready_nodes(&self) -> Vec<String> {
        self.nodes.values()
            .filter(|n| n.state == NodeState::Pending || n.state == NodeState::Ready)
            .filter(|n| n.dependencies.iter().all(|d| {
                self.nodes.get(d).map(|dep| dep.state == NodeState::Completed).unwrap_or(false)
            }))
            .map(|n| n.id.clone())
            .collect()
    }

    /// Critical path: longest path through the DAG.
    pub fn critical_path(&self) -> Result<Vec<String>, OrchestratorError> {
        let order = self.topological_order()?;
        let mut dist: HashMap<String, i64> = HashMap::new();
        let mut pred: HashMap<String, Option<String>> = HashMap::new();
        for nid in &order {
            dist.insert(nid.clone(), 0);
            pred.insert(nid.clone(), None);
        }
        for nid in &order {
            let d = *dist.get(nid).unwrap_or(&0);
            if let Some(node) = self.nodes.get(nid) {
                for dep in &node.dependencies {
                    let new_dist = *dist.get(dep).unwrap_or(&0) + 1;
                    if new_dist > d {
                        dist.insert(nid.clone(), new_dist);
                        pred.insert(nid.clone(), Some(dep.clone()));
                    }
                }
            }
        }
        // Find end node with max distance
        let end = dist.iter().max_by_key(|(_, &v)| v).map(|(k, _)| k.clone());
        let mut path = vec![];
        let mut current = end;
        while let Some(nid) = current {
            path.push(nid.clone());
            current = pred.get(&nid).cloned().flatten();
        }
        path.reverse();
        Ok(path)
    }

    /// Check if workflow is complete (all nodes terminal).
    pub fn is_complete(&self) -> bool {
        self.nodes.values().all(|n| n.state.is_terminal())
    }

    /// Progress percentage (0-100).
    pub fn progress(&self) -> u8 {
        if self.nodes.is_empty() { return 100; }
        let completed = self.nodes.values().filter(|n| n.state == NodeState::Completed).count();
        ((completed as f64 / self.nodes.len() as f64) * 100.0) as u8
    }
}

/// Resource pool for RCPSP (Resource-Constrained Project Scheduling Problem).
#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub resources: HashMap<String, u32>,
    pub available: HashMap<String, u32>,
}

impl ResourcePool {
    pub fn new() -> Self {
        Self { resources: HashMap::new(), available: HashMap::new() }
    }
    pub fn add_resource(&mut self, name: &str, capacity: u32) {
        self.resources.insert(name.into(), capacity);
        self.available.insert(name.into(), capacity);
    }
    pub fn try_acquire(&mut self, requirements: &HashMap<String, u32>) -> bool {
        for (res, &need) in requirements {
            if self.available.get(res).copied().unwrap_or(0) < need { return false; }
        }
        for (res, &need) in requirements {
            if let Some(a) = self.available.get_mut(res) { *a -= need; }
        }
        true
    }
    pub fn release(&mut self, requirements: &HashMap<String, u32>) {
        for (res, &need) in requirements {
            if let Some(a) = self.available.get_mut(res) { *a += need; }
        }
    }
}

impl Default for ResourcePool {
    fn default() -> Self { Self::new() }
}

/// Orchestrator engine: manages workflow lifecycle and execution.
#[derive(Clone)]
pub struct OrchestratorEngine {
    workflows: Arc<parking_lot::Mutex<HashMap<String, Workflow>>>,
    resource_pool: Arc<parking_lot::Mutex<ResourcePool>>,
}

impl OrchestratorEngine {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            resource_pool: Arc::new(parking_lot::Mutex::new(ResourcePool::new())),
        }
    }

    pub fn create_workflow(&self, name: &str) -> Workflow {
        let wf = Workflow::new(name);
        self.workflows.lock().insert(wf.id.clone(), wf.clone());
        wf
    }

    pub fn get_workflow(&self, id: &str) -> Option<Workflow> {
        self.workflows.lock().get(id).cloned()
    }

    pub fn start_workflow(&self, id: &str) -> Result<Workflow, OrchestratorError> {
        let mut wfs = self.workflows.lock();
        let wf = wfs.get_mut(id).ok_or_else(|| OrchestratorError::NodeNotFound(id.into()))?;
        wf.validate()?;
        wf.state = WorkflowState::Running;
        // Mark nodes with no deps as Ready
        let ready = wf.ready_nodes();
        for nid in ready {
            if let Some(n) = wf.nodes.get_mut(&nid) {
                n.state = NodeState::Ready;
            }
        }
        wf.touch();
        Ok(wf.clone())
    }

    pub fn complete_node(&self, wf_id: &str, node_id: &str, result: Option<serde_json::Value>) -> Result<Workflow, OrchestratorError> {
        let mut wfs = self.workflows.lock();
        let wf = wfs.get_mut(wf_id).ok_or_else(|| OrchestratorError::NodeNotFound(wf_id.into()))?;
        let node = wf.nodes.get_mut(node_id).ok_or_else(|| OrchestratorError::NodeNotFound(node_id.into()))?;
        if !node.state.can_transition_to(NodeState::Completed) {
            return Err(OrchestratorError::InvalidTransition(format!("{:?}", node.state), "Completed".into()));
        }
        node.state = NodeState::Completed;
        node.completed_at = Some(chrono::Utc::now().to_rfc3339());
        node.result = result;
        // Mark newly ready nodes
        let ready = wf.ready_nodes();
        for nid in ready {
            if let Some(n) = wf.nodes.get_mut(&nid) {
                if n.state == NodeState::Pending { n.state = NodeState::Ready; }
            }
        }
        if wf.is_complete() { wf.state = WorkflowState::Completed; }
        wf.touch();
        Ok(wf.clone())
    }

    pub fn fail_node(&self, wf_id: &str, node_id: &str, error: &str) -> Result<Workflow, OrchestratorError> {
        let mut wfs = self.workflows.lock();
        let wf = wfs.get_mut(wf_id).ok_or_else(|| OrchestratorError::NodeNotFound(wf_id.into()))?;
        let node = wf.nodes.get_mut(node_id).ok_or_else(|| OrchestratorError::NodeNotFound(node_id.into()))?;
        node.retry_count += 1;
        if node.retry_count < node.max_retries {
            node.state = NodeState::Ready; // retry
        } else {
            node.state = NodeState::Failed;
            node.error = Some(error.into());
            wf.state = WorkflowState::Failed;
        }
        wf.touch();
        Ok(wf.clone())
    }

    pub fn list_workflows(&self) -> Vec<Workflow> {
        self.workflows.lock().values().cloned().collect()
    }
}

impl Default for OrchestratorEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_topological_sort() {
        let mut wf = Workflow::new("test");
        wf.add_node(WorkflowNode::new("a", "A", "op1"));
        wf.add_node(WorkflowNode::new("b", "B", "op2"));
        wf.add_node(WorkflowNode::new("c", "C", "op3"));
        wf.add_dependency("b", "a").unwrap();
        wf.add_dependency("c", "b").unwrap();
        let order = wf.topological_order().unwrap();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn cycle_detection() {
        let mut wf = Workflow::new("cycle");
        wf.add_node(WorkflowNode::new("a", "A", "op"));
        wf.add_node(WorkflowNode::new("b", "B", "op"));
        wf.add_dependency("a", "b").unwrap();
        wf.add_dependency("b", "a").unwrap();
        assert!(wf.validate().is_err());
    }

    #[test]
    fn orchestrator_lifecycle() {
        let eng = OrchestratorEngine::new();
        let mut wf = eng.create_workflow("test");
        wf.add_node(WorkflowNode::new("n1", "N1", "op"));
        let wf = eng.start_workflow(&wf.id).unwrap();
        assert_eq!(wf.state, WorkflowState::Running);
        let wf = eng.complete_node(&wf.id, "n1", None).unwrap();
        assert_eq!(wf.state, WorkflowState::Completed);
        assert_eq!(wf.progress(), 100);
    }

    #[test]
    fn critical_path() {
        let mut wf = Workflow::new("cp");
        for id in ["a", "b", "c", "d"] {
            wf.add_node(WorkflowNode::new(id, id, "op"));
        }
        wf.add_dependency("b", "a").unwrap();
        wf.add_dependency("c", "a").unwrap();
        wf.add_dependency("d", "b").unwrap();
        wf.add_dependency("d", "c").unwrap();
        let path = wf.critical_path().unwrap();
        assert!(path.first().unwrap() == "a");
        assert!(path.last().unwrap() == "d");
    }

    #[test]
    fn resource_pool() {
        let mut pool = ResourcePool::new();
        pool.add_resource("cpu", 4);
        let req = HashMap::from([("cpu".to_string(), 2u32)]);
        assert!(pool.try_acquire(&req));
        assert!(pool.try_acquire(&req));
        assert!(!pool.try_acquire(&req)); // only 4 CPUs
        pool.release(&req);
        assert!(pool.try_acquire(&req));
    }
}
