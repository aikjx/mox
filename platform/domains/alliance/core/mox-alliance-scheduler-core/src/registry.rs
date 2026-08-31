// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家注册桥接层
//!
//! 提供联盟调度器与 AI 专家服务之间的专家注册桥接能力，
//! 支持从不同来源（内存、HTTP API）同步专家列表到匹配器。
//!
//! ## 设计原则
//! - DIP 依赖倒置：匹配器依赖 `ExpertRegistryBridge` trait 抽象，不依赖具体实现
//! - 可插拔：内存版 / HTTP 版可自由切换
//! - 统一类型：桥接层负责类型转换，匹配器只认识 `Expert` 类型

use async_trait::async_trait;
use mox_alliance_common_proto::{
    AllianceError, AllianceResult, Capability, Expert, ExpertHealth,
    ExpertStatus, ToolBinding,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use crate::matcher::RuleBasedExpertMatcher;

#[cfg(feature = "http-bridge")]
use tracing::info;

// ============================================================================
// ExpertRegistryBridge trait
// ============================================================================

/// 专家注册桥接器 trait
///
/// 定义联盟调度器与外部专家注册中心之间的桥接接口。
/// 匹配器通过此 trait 获取专家列表，实现专家来源的可插拔。
///
/// ## 实现策略
/// - `InMemoryExpertRegistry`：内存版，包装 `RuleBasedExpertMatcher`，用于测试和单机部署
/// - `HttpExpertRegistryBridge`：通过 HTTP 从 AI 专家服务拉取专家列表，用于生产部署
#[async_trait]
pub trait ExpertRegistryBridge: Send + Sync {
    /// 全量同步专家列表
    ///
    /// 从外部注册中心拉取全部专家，替换本地缓存。
    /// 返回同步的专家数量。
    async fn sync_experts(&self, experts: Vec<Expert>) -> AllianceResult<usize>;

    /// 获取全部专家列表
    async fn get_expert_list(&self) -> AllianceResult<Vec<Expert>>;

    /// 注册单个专家
    ///
    /// 若专家已存在则更新。
    async fn register_expert(&self, expert: Expert) -> AllianceResult<()>;

    /// 注销专家
    async fn unregister_expert(&self, expert_id: &str) -> AllianceResult<()>;

    /// 获取单个专家
    async fn get_expert(&self, expert_id: &str) -> AllianceResult<Option<Expert>>;

    /// 更新专家健康状态
    async fn update_expert_health(&self, expert_id: &str, health: ExpertHealth) -> AllianceResult<()>;

    /// 获取专家总数
    async fn expert_count(&self) -> usize;
}

/// 同步结果统计
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// 新增专家数
    pub added: usize,
    /// 更新专家数
    pub updated: usize,
    /// 移除专家数
    pub removed: usize,
    /// 总专家数（同步后）
    pub total: usize,
}

// ============================================================================
// InMemoryExpertRegistry
// ============================================================================

/// 内存版专家注册表
///
/// 包装 `RuleBasedExpertMatcher` 的内部 HashMap，
/// 实现 `ExpertRegistryBridge` trait，用于测试和单机部署。
pub struct InMemoryExpertRegistry {
    experts: Arc<RwLock<HashMap<String, Expert>>>,
}

impl InMemoryExpertRegistry {
    pub fn new() -> Self {
        Self {
            experts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 使用共享的专家存储创建注册表
    ///
    /// 用于与 `RuleBasedExpertMatcher` 共享同一份专家数据，
    /// 这样同步器写入 registry 后，匹配器可以直接读到最新数据。
    pub fn with_shared_experts(experts: Arc<RwLock<HashMap<String, Expert>>>) -> Self {
        Self { experts }
    }

    /// 从现有 matcher 创建（共享内部状态）
    ///
    /// 这样 registry 和 matcher 共享同一份专家 HashMap，
    /// 任何一方的修改都会立即对另一方可见。
    pub fn from_matcher(matcher: &RuleBasedExpertMatcher) -> Self {
        Self::with_shared_experts(matcher.experts_arc())
    }

    /// 获取内部 experts 的 Arc 引用（供 matcher 共享使用）
    pub fn experts_arc(&self) -> Arc<RwLock<HashMap<String, Expert>>> {
        self.experts.clone()
    }

    /// 增量同步：与现有专家对比，返回同步统计
    pub fn incremental_sync(&self, incoming: Vec<Expert>) -> SyncStats {
        let mut stats = SyncStats::default();
        let mut experts = self.experts.write();

        // 构建 incoming 的 id 集合
        let incoming_ids: std::collections::HashSet<String> =
            incoming.iter().map(|e| e.expert_id.clone()).collect();

        // 处理新增和更新
        for expert in incoming {
            let id = expert.expert_id.clone();
            if experts.contains_key(&id) {
                stats.updated += 1;
            } else {
                stats.added += 1;
            }
            experts.insert(id, expert);
        }

        // 处理移除（不在 incoming 中的旧专家）
        let to_remove: Vec<String> = experts
            .keys()
            .filter(|id| !incoming_ids.contains(*id))
            .cloned()
            .collect();
        stats.removed = to_remove.len();
        for id in to_remove {
            experts.remove(&id);
        }

        stats.total = experts.len();
        stats
    }
}

impl Default for InMemoryExpertRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExpertRegistryBridge for InMemoryExpertRegistry {
    async fn sync_experts(&self, experts: Vec<Expert>) -> AllianceResult<usize> {
        let count = experts.len();
        let mut map = HashMap::with_capacity(count);
        for expert in experts {
            map.insert(expert.expert_id.clone(), expert);
        }
        *self.experts.write() = map;
        debug!("InMemory registry synced {} experts", count);
        Ok(count)
    }

    async fn get_expert_list(&self) -> AllianceResult<Vec<Expert>> {
        let experts = self.experts.read();
        Ok(experts.values().cloned().collect())
    }

    async fn register_expert(&self, expert: Expert) -> AllianceResult<()> {
        let id = expert.expert_id.clone();
        self.experts.write().insert(id.clone(), expert);
        debug!("Registered expert: {}", id);
        Ok(())
    }

    async fn unregister_expert(&self, expert_id: &str) -> AllianceResult<()> {
        self.experts
            .write()
            .remove(expert_id)
            .ok_or_else(|| AllianceError::not_found("Expert", expert_id))?;
        debug!("Unregistered expert: {}", expert_id);
        Ok(())
    }

    async fn get_expert(&self, expert_id: &str) -> AllianceResult<Option<Expert>> {
        Ok(self.experts.read().get(expert_id).cloned())
    }

    async fn update_expert_health(&self, expert_id: &str, health: ExpertHealth) -> AllianceResult<()> {
        let mut experts = self.experts.write();
        let expert = experts
            .get_mut(expert_id)
            .ok_or_else(|| AllianceError::not_found("Expert", expert_id))?;
        expert.health = health;
        expert.updated_at = chrono::Utc::now();
        debug!("Updated health for expert: {}", expert_id);
        Ok(())
    }

    async fn expert_count(&self) -> usize {
        self.experts.read().len()
    }
}

// ============================================================================
// HttpExpertRegistryBridge
// ============================================================================

/// HTTP 专家注册桥接器配置
#[derive(Debug, Clone)]
pub struct HttpBridgeConfig {
    /// AI 专家服务基地址，如 `http://localhost:3300`
    pub base_url: String,
    /// 请求超时（毫秒）
    pub timeout_ms: u64,
    /// 租户 ID
    pub tenant_id: String,
}

impl Default for HttpBridgeConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3300".to_string(),
            timeout_ms: 5000,
            tenant_id: "system".to_string(),
        }
    }
}

/// HTTP 专家列表 API 响应
#[derive(Debug, Clone, serde::Deserialize)]
struct ExpertListApiResponse {
    pub total: usize,
    pub experts: Vec<ExpertMetaApi>,
}

/// API 返回的专家元数据（对齐 AI 专家服务的 ExpertMeta）
#[derive(Debug, Clone, serde::Deserialize)]
struct ExpertMetaApi {
    pub id: String,
    pub name: String,
    pub domain: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub dimension: Option<String>,
}

/// HTTP 专家注册桥接器
///
/// 通过 HTTP API 从 AI 专家服务（mox-ai-expert-svc）拉取专家列表，
/// 并将 `ExpertMeta` 转换为联盟调度器使用的 `Expert` 类型。
///
/// ## 类型转换策略
/// AI 专家服务的 `ExpertMeta` 是轻量元数据，联盟调度器的 `Expert` 包含
/// 更丰富的字段（健康状态、工具绑定、能力详情等）。桥接层负责补全默认值。
#[cfg(feature = "http-bridge")]
pub struct HttpExpertRegistryBridge {
    config: HttpBridgeConfig,
    client: reqwest::Client,
    /// 本地缓存（保存转换后的 Expert 列表）
    cache: InMemoryExpertRegistry,
}

#[cfg(feature = "http-bridge")]
impl HttpExpertRegistryBridge {
    pub fn new(config: HttpBridgeConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            client,
            cache: InMemoryExpertRegistry::new(),
        }
    }

    /// 从远程拉取专家列表并转换为 Expert 类型
    pub async fn fetch_experts(&self) -> AllianceResult<Vec<Expert>> {
        let url = format!(
            "{}/api/v1/experts?page_size=100&domain={}",
            self.config.base_url, self.config.tenant_id
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                AllianceError::internal(format!("Failed to fetch experts from AI service: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(AllianceError::internal(format!(
                "AI expert service returned status: {}",
                response.status()
            )));
        }

        let api_resp: ExpertListApiResponse = response.json().await.map_err(|e| {
            AllianceError::internal(format!("Failed to parse expert list response: {}", e))
        })?;

        let experts: Vec<Expert> = api_resp
            .experts
            .into_iter()
            .map(|meta| expert_meta_to_expert(meta, &self.config.tenant_id))
            .collect();

        info!(
            "Fetched {} experts from AI expert service",
            experts.len()
        );
        Ok(experts)
    }

    /// 获取缓存引用（供同步器写入）
    pub fn cache(&self) -> &InMemoryExpertRegistry {
        &self.cache
    }
}

#[cfg(feature = "http-bridge")]
#[async_trait]
impl ExpertRegistryBridge for HttpExpertRegistryBridge {
    async fn sync_experts(&self, experts: Vec<Expert>) -> AllianceResult<usize> {
        self.cache.sync_experts(experts).await
    }

    async fn get_expert_list(&self) -> AllianceResult<Vec<Expert>> {
        self.cache.get_expert_list().await
    }

    async fn register_expert(&self, expert: Expert) -> AllianceResult<()> {
        self.cache.register_expert(expert).await
    }

    async fn unregister_expert(&self, expert_id: &str) -> AllianceResult<()> {
        self.cache.unregister_expert(expert_id).await
    }

    async fn get_expert(&self, expert_id: &str) -> AllianceResult<Option<Expert>> {
        self.cache.get_expert(expert_id).await
    }

    async fn update_expert_health(&self, expert_id: &str, health: ExpertHealth) -> AllianceResult<()> {
        self.cache.update_expert_health(expert_id, health).await
    }

    async fn expert_count(&self) -> usize {
        self.cache.expert_count().await
    }
}

// ============================================================================
// 类型转换辅助函数
// ============================================================================

/// 将 AI 专家服务的 ExpertMeta 转换为联盟调度器的 Expert
///
/// 由于 ExpertMeta 是轻量元数据，转换时会为缺失字段填充合理默认值。
fn expert_meta_to_expert(meta: ExpertMetaApi, tenant_id: &str) -> Expert {
    let now = chrono::Utc::now();
    let capabilities: Vec<Capability> = meta
        .capabilities
        .iter()
        .enumerate()
        .map(|(i, cap)| Capability {
            capability_id: format!("{}-cap-{}", meta.id, i),
            name: cap.clone(),
            description: format!("能力：{}", cap),
            domain: meta.domain.clone(),
            version: "0.1.0".to_string(),
        })
        .collect();

    Expert {
        expert_id: meta.id.clone(),
        tenant_id: tenant_id.to_string(),
        name: meta.name.clone(),
        version: "0.1.0".to_string(),
        description: meta.description.clone(),
        domains: vec![meta.domain.clone()],
        capabilities,
        tools: vec![],
        status: ExpertStatus::Active,
        health: ExpertHealth::default(),
        priority: 5,
        created_at: now,
        updated_at: now,
    }
}

// ============================================================================
// 预置领域专家工厂
// ============================================================================

/// 十大领域专家定义
///
/// 支持的 10 位领域专家：
/// 1. 图谱构建 (graph_construction)
/// 2. 数据分析 (data_analysis)
/// 3. AI推理 (ai_reasoning)
/// 4. 安全审计 (security_audit)
/// 5. 流程自动化 (process_automation)
/// 6. 数据治理 (data_governance)
/// 7. 知识融合 (knowledge_fusion)
/// 8. 搜索推荐 (search_recommendation)
/// 9. DevOps (devops)
/// 10. 联盟协调 (alliance_coordination)
pub mod domain_experts {
    use super::*;

    /// 创建所有 10 位预置领域专家
    pub fn all_domain_experts() -> Vec<Expert> {
        vec![
            graph_construction_expert(),
            data_analysis_expert(),
            ai_reasoning_expert(),
            security_audit_expert(),
            process_automation_expert(),
            data_governance_expert(),
            knowledge_fusion_expert(),
            search_recommendation_expert(),
            devops_expert(),
            alliance_coordination_expert(),
        ]
    }

    /// 1. 图谱构建专家
    pub fn graph_construction_expert() -> Expert {
        build_domain_expert(
            "graph_construction",
            "图谱构建专家",
            "kg",
            "负责知识图谱的构建与维护，包括实体抽取、关系发现、schema 设计",
            vec!["图谱构建", "实体抽取", "关系抽取", "schema设计", "知识建模"],
            8,
        )
    }

    /// 2. 数据分析专家
    pub fn data_analysis_expert() -> Expert {
        build_domain_expert(
            "data_analysis",
            "数据分析专家",
            "data",
            "负责数据探查、统计分析、趋势预测和洞察挖掘",
            vec!["数据分析", "统计建模", "趋势分析", "数据探查", "洞察挖掘"],
            7,
        )
    }

    /// 3. AI推理专家
    pub fn ai_reasoning_expert() -> Expert {
        build_domain_expert(
            "ai_reasoning",
            "AI推理专家",
            "ai",
            "负责大模型推理、逻辑推理、问题求解和智能决策",
            vec!["AI推理", "大模型", "逻辑推理", "问题求解", "智能决策"],
            9,
        )
    }

    /// 4. 安全审计专家
    pub fn security_audit_expert() -> Expert {
        build_domain_expert(
            "security_audit",
            "安全审计专家",
            "security",
            "负责安全风险评估、漏洞扫描、合规审计和安全策略制定",
            vec!["安全审计", "漏洞扫描", "风险评估", "合规检查", "安全策略"],
            9,
        )
    }

    /// 5. 流程自动化专家
    pub fn process_automation_expert() -> Expert {
        build_domain_expert(
            "process_automation",
            "流程自动化专家",
            "flow",
            "负责业务流程建模、自动化编排、流程优化和效率提升",
            vec!["流程自动化", "业务建模", "流程编排", "效率优化", "RPA"],
            6,
        )
    }

    /// 6. 数据治理专家
    pub fn data_governance_expert() -> Expert {
        build_domain_expert(
            "data_governance",
            "数据治理专家",
            "data",
            "负责数据质量管理、元数据管理、数据标准和数据资产治理",
            vec!["数据治理", "数据质量", "元数据", "数据标准", "资产治理"],
            8,
        )
    }

    /// 7. 知识融合专家
    pub fn knowledge_fusion_expert() -> Expert {
        build_domain_expert(
            "knowledge_fusion",
            "知识融合专家",
            "kg",
            "负责多源知识融合、实体对齐、知识补全和质量评估",
            vec!["知识融合", "实体对齐", "知识补全", "质量评估", "多源整合"],
            7,
        )
    }

    /// 8. 搜索推荐专家
    pub fn search_recommendation_expert() -> Expert {
        build_domain_expert(
            "search_recommendation",
            "搜索推荐专家",
            "ai",
            "负责搜索引擎优化、推荐系统设计、相关性排序和个性化推荐",
            vec!["搜索推荐", "相关性排序", "个性化推荐", "召回策略", "排序模型"],
            7,
        )
    }

    /// 9. DevOps专家
    pub fn devops_expert() -> Expert {
        build_domain_expert(
            "devops",
            "DevOps专家",
            "platform",
            "负责持续集成、持续部署、基础设施管理和运维自动化",
            vec!["DevOps", "CI/CD", "基础设施", "运维自动化", "容器化"],
            6,
        )
    }

    /// 10. 联盟协调专家
    pub fn alliance_coordination_expert() -> Expert {
        build_domain_expert(
            "alliance_coordination",
            "联盟协调专家",
            "alliance",
            "负责多专家协作编排、冲突调解、任务分发和结果融合",
            vec!["联盟协调", "多专家协作", "冲突调解", "任务编排", "结果融合"],
            10,
        )
    }

    /// 构建领域专家的辅助函数
    fn build_domain_expert(
        id: &str,
        name: &str,
        domain: &str,
        description: &str,
        capability_names: Vec<&str>,
        priority: u8,
    ) -> Expert {
        let now = chrono::Utc::now();
        let capabilities: Vec<Capability> = capability_names
            .iter()
            .enumerate()
            .map(|(i, cap)| Capability {
                capability_id: format!("{}-cap-{}", id, i),
                name: cap.to_string(),
                description: format!("{} 能力", cap),
                domain: domain.to_string(),
                version: "1.0.0".to_string(),
            })
            .collect();

        Expert {
            expert_id: id.to_string(),
            tenant_id: "system".to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: description.to_string(),
            domains: vec![domain.to_string()],
            capabilities,
            tools: vec![ToolBinding {
                tool_id: format!("{}-default-tool", id),
                name: format!("{} 默认工具", name),
                description: format!("{} 的默认调用工具", name),
                protocol: "HTTP".to_string(),
                endpoint: format!("/api/v1/experts/{}/invoke", id),
                input_schema: None,
                output_schema: None,
            }],
            status: ExpertStatus::Active,
            health: ExpertHealth::default(),
            priority,
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_register_and_get() {
        let reg = InMemoryExpertRegistry::new();
        let mut expert = Expert::new_system("test-expert".into(), "测试专家".into());
        expert.expert_id = "exp-001".into();

        reg.register_expert(expert.clone()).await.unwrap();

        let found = reg.get_expert("exp-001").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-expert");
        assert_eq!(reg.expert_count().await, 1);
    }

    #[tokio::test]
    async fn in_memory_sync_replaces_all() {
        let reg = InMemoryExpertRegistry::new();

        let mut e1 = Expert::new_system("e1".into(), "专家1".into());
        e1.expert_id = "e1".into();
        let mut e2 = Expert::new_system("e2".into(), "专家2".into());
        e2.expert_id = "e2".into();

        // 先注册 e1
        reg.register_expert(e1).await.unwrap();
        assert_eq!(reg.expert_count().await, 1);

        // 同步 e2（应该只剩 e2）
        let count = reg.sync_experts(vec![e2]).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(reg.expert_count().await, 1);
        assert!(reg.get_expert("e1").await.unwrap().is_none());
        assert!(reg.get_expert("e2").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn in_memory_unregister() {
        let reg = InMemoryExpertRegistry::new();
        let mut expert = Expert::new_system("test".into(), "测试".into());
        expert.expert_id = "test-1".into();

        reg.register_expert(expert).await.unwrap();
        assert_eq!(reg.expert_count().await, 1);

        reg.unregister_expert("test-1").await.unwrap();
        assert_eq!(reg.expert_count().await, 0);

        // 重复删除应报错
        let result = reg.unregister_expert("test-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn in_memory_update_health() {
        let reg = InMemoryExpertRegistry::new();
        let mut expert = Expert::new_system("health-test".into(), "健康测试".into());
        expert.expert_id = "health-1".into();
        reg.register_expert(expert).await.unwrap();

        let mut health = ExpertHealth::default();
        health.is_healthy = false;
        health.success_rate = 0.5;
        health.error_count = 10;

        reg.update_expert_health("health-1", health.clone()).await.unwrap();

        let found = reg.get_expert("health-1").await.unwrap().unwrap();
        assert!(!found.health.is_healthy);
        assert!((found.health.success_rate - 0.5).abs() < 1e-9);
        assert_eq!(found.health.error_count, 10);
    }

    #[tokio::test]
    async fn in_memory_get_expert_list() {
        let reg = InMemoryExpertRegistry::new();

        for i in 0..5 {
            let mut e = Expert::new_system(format!("expert-{}", i), format!("专家{}", i));
            e.expert_id = format!("exp-{}", i);
            reg.register_expert(e).await.unwrap();
        }

        let list = reg.get_expert_list().await.unwrap();
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn incremental_sync_stats() {
        let reg = InMemoryExpertRegistry::new();

        // 初始：e1, e2
        let mut e1 = Expert::new_system("e1".into(), "专家1".into());
        e1.expert_id = "e1".into();
        let mut e2 = Expert::new_system("e2".into(), "专家2".into());
        e2.expert_id = "e2".into();

        let stats = reg.incremental_sync(vec![e1.clone(), e2.clone()]);
        assert_eq!(stats.added, 2);
        assert_eq!(stats.updated, 0);
        assert_eq!(stats.removed, 0);
        assert_eq!(stats.total, 2);

        // 增量：e2 更新, e3 新增, e1 移除
        let mut e2_v2 = e2.clone();
        e2_v2.description = "更新后的描述".into();
        let mut e3 = Expert::new_system("e3".into(), "专家3".into());
        e3.expert_id = "e3".into();

        let stats = reg.incremental_sync(vec![e2_v2, e3]);
        assert_eq!(stats.added, 1); // e3
        assert_eq!(stats.updated, 1); // e2
        assert_eq!(stats.removed, 1); // e1
        assert_eq!(stats.total, 2);
    }

    #[test]
    fn all_domain_experts_has_10_experts() {
        let experts = domain_experts::all_domain_experts();
        assert_eq!(experts.len(), 10);

        // 验证每个专家的基本字段
        for expert in &experts {
            assert!(!expert.expert_id.is_empty());
            assert!(!expert.name.is_empty());
            assert!(!expert.domains.is_empty());
            assert!(!expert.capabilities.is_empty());
            assert_eq!(expert.status, ExpertStatus::Active);
            assert_eq!(expert.tenant_id, "system");
        }
    }

    #[test]
    fn domain_experts_have_expected_ids() {
        let experts = domain_experts::all_domain_experts();
        let ids: Vec<&str> = experts.iter().map(|e| e.expert_id.as_str()).collect();

        assert!(ids.contains(&"graph_construction"));
        assert!(ids.contains(&"data_analysis"));
        assert!(ids.contains(&"ai_reasoning"));
        assert!(ids.contains(&"security_audit"));
        assert!(ids.contains(&"process_automation"));
        assert!(ids.contains(&"data_governance"));
        assert!(ids.contains(&"knowledge_fusion"));
        assert!(ids.contains(&"search_recommendation"));
        assert!(ids.contains(&"devops"));
        assert!(ids.contains(&"alliance_coordination"));
    }
}
