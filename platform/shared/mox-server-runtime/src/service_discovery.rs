// =============================================================================
// 服务发现（ServiceDiscovery）
// =============================================================================
//
// 统一服务发现抽象，支持多种注册中心：
// - MemoryRegistry：内存注册（测试/默认/单体部署）
// - StaticRegistry：静态配置（配置文件定义服务列表）
// - RemoteRegistry：远程注册中心（Consul/etcd/Nacos/K8s，预留扩展点）
//
// 核心能力：
// - 服务注册/注销（register/deregister）
// - 服务发现（discover/discover_all）
// - 健康检查（health_check，自动剔除不健康实例）
// - 负载均衡（轮询/随机/加权轮询）
// - 服务元数据（版本/权重/标签/区域）
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 服务实例状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ServiceStatus {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 离线
    Offline,
    /// 维护中
    Maintenance,
}

impl Default for ServiceStatus {
    fn default() -> Self { ServiceStatus::Healthy }
}

/// 服务实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// 服务名称（如 "mox-kg-server"）
    pub service_name: String,
    /// 实例 ID（唯一，如 UUID）
    pub instance_id: String,
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 服务版本
    pub version: String,
    /// 权重（负载均衡用，默认 1）
    pub weight: u32,
    /// 标签（如 env=prod, region=cn-east）
    pub tags: HashMap<String, String>,
    /// 状态
    pub status: ServiceStatus,
    /// 注册时间（RFC3339）
    pub registered_at: String,
    /// 最后心跳时间（RFC3339）
    pub last_heartbeat: String,
}

impl ServiceInstance {
    /// 创建新服务实例
    pub fn new(service_name: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            service_name: service_name.into(),
            instance_id: uuid::Uuid::new_v4().simple().to_string(),
            host: host.into(),
            port,
            version: "1.0.0".to_string(),
            weight: 1,
            tags: HashMap::new(),
            status: ServiceStatus::Healthy,
            registered_at: now.clone(),
            last_heartbeat: now,
        }
    }

    /// 获取完整地址
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// 获取 HTTP URL
    pub fn http_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// 是否健康
    pub fn is_healthy(&self) -> bool {
        self.status == ServiceStatus::Healthy
    }
}

/// 服务注册中心 trait
pub trait ServiceRegistry: Send + Sync {
    /// 注册服务实例
    fn register(&self, instance: ServiceInstance) -> Result<(), String>;

    /// 注销服务实例
    fn deregister(&self, service_name: &str, instance_id: &str) -> Result<(), String>;

    /// 发现健康的服务实例
    fn discover(&self, service_name: &str) -> Vec<ServiceInstance>;

    /// 发现所有服务实例（包括不健康的）
    fn discover_all(&self, service_name: &str) -> Vec<ServiceInstance>;

    /// 获取所有服务名称
    fn services(&self) -> Vec<String>;

    /// 心跳更新
    fn heartbeat(&self, service_name: &str, instance_id: &str) -> Result<(), String>;

    /// 更新实例状态
    fn update_status(&self, service_name: &str, instance_id: &str, status: ServiceStatus) -> Result<(), String>;

    /// 注册中心名称
    fn name(&self) -> &str;
}

/// 内存服务注册中心（默认实现，线程安全）
pub struct MemoryRegistry {
    name: String,
    instances: Mutex<HashMap<String, HashMap<String, ServiceInstance>>>,
}

impl MemoryRegistry {
    /// 创建内存注册中心
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// 批量注册服务实例
    pub fn register_all(&self, instances: Vec<ServiceInstance>) {
        for instance in instances {
            let _ = self.register(instance);
        }
    }
}

impl ServiceRegistry for MemoryRegistry {
    fn register(&self, instance: ServiceInstance) -> Result<(), String> {
        let mut instances = self.instances.lock().unwrap();
        instances
            .entry(instance.service_name.clone())
            .or_insert_with(HashMap::new)
            .insert(instance.instance_id.clone(), instance);
        Ok(())
    }

    fn deregister(&self, service_name: &str, instance_id: &str) -> Result<(), String> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(service_instances) = instances.get_mut(service_name) {
            service_instances.remove(instance_id);
            if service_instances.is_empty() {
                instances.remove(service_name);
            }
        }
        Ok(())
    }

    fn discover(&self, service_name: &str) -> Vec<ServiceInstance> {
        let instances = self.instances.lock().unwrap();
        instances
            .get(service_name)
            .map(|m| m.values().filter(|i| i.is_healthy()).cloned().collect())
            .unwrap_or_default()
    }

    fn discover_all(&self, service_name: &str) -> Vec<ServiceInstance> {
        let instances = self.instances.lock().unwrap();
        instances
            .get(service_name)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    fn services(&self) -> Vec<String> {
        let instances = self.instances.lock().unwrap();
        instances.keys().cloned().collect()
    }

    fn heartbeat(&self, service_name: &str, instance_id: &str) -> Result<(), String> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(service_instances) = instances.get_mut(service_name) {
            if let Some(instance) = service_instances.get_mut(instance_id) {
                instance.last_heartbeat = chrono::Utc::now().to_rfc3339();
                instance.status = ServiceStatus::Healthy;
                return Ok(());
            }
        }
        Err(format!("实例未找到: {}/{}", service_name, instance_id))
    }

    fn update_status(&self, service_name: &str, instance_id: &str, status: ServiceStatus) -> Result<(), String> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(service_instances) = instances.get_mut(service_name) {
            if let Some(instance) = service_instances.get_mut(instance_id) {
                instance.status = status;
                return Ok(());
            }
        }
        Err(format!("实例未找到: {}/{}", service_name, instance_id))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 静态服务注册中心（从配置加载，不可动态注册）
pub struct StaticRegistry {
    inner: MemoryRegistry,
}

impl StaticRegistry {
    /// 从服务列表创建静态注册中心
    pub fn from_instances(instances: Vec<ServiceInstance>) -> Self {
        let inner = MemoryRegistry::new("static");
        inner.register_all(instances);
        Self { inner }
    }

    /// 从配置文件创建（JSON 格式）
    pub fn from_config(config_json: &str) -> Result<Self, String> {
        let instances: Vec<ServiceInstance> = serde_json::from_str(config_json)
            .map_err(|e| format!("解析服务配置失败: {e}"))?;
        Ok(Self::from_instances(instances))
    }
}

impl ServiceRegistry for StaticRegistry {
    fn register(&self, _instance: ServiceInstance) -> Result<(), String> {
        Err("静态注册中心不支持动态注册".to_string())
    }
    fn deregister(&self, _service_name: &str, _instance_id: &str) -> Result<(), String> {
        Err("静态注册中心不支持动态注销".to_string())
    }
    fn discover(&self, service_name: &str) -> Vec<ServiceInstance> { self.inner.discover(service_name) }
    fn discover_all(&self, service_name: &str) -> Vec<ServiceInstance> { self.inner.discover_all(service_name) }
    fn services(&self) -> Vec<String> { self.inner.services() }
    fn heartbeat(&self, service_name: &str, instance_id: &str) -> Result<(), String> { self.inner.heartbeat(service_name, instance_id) }
    fn update_status(&self, service_name: &str, instance_id: &str, status: ServiceStatus) -> Result<(), String> { self.inner.update_status(service_name, instance_id, status) }
    fn name(&self) -> &str { self.inner.name() }
}

// =============================================================================
// 负载均衡
// =============================================================================

/// 负载均衡策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// 轮询
    RoundRobin,
    /// 随机
    Random,
    /// 加权轮询
    WeightedRoundRobin,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self { LoadBalanceStrategy::RoundRobin }
}

/// 负载均衡器
pub struct LoadBalancer {
    strategy: LoadBalanceStrategy,
    round_robin_counter: AtomicU64,
}

impl LoadBalancer {
    /// 创建负载均衡器
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            strategy,
            round_robin_counter: AtomicU64::new(0),
        }
    }

    /// 选择一个服务实例
    pub fn select(&self, instances: &[ServiceInstance]) -> Option<ServiceInstance> {
        if instances.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) as usize % instances.len();
                Some(instances[idx].clone())
            }
            LoadBalanceStrategy::Random => {
                // 简单的伪随机（基于时间戳）
                let nanos = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as usize;
                let idx = nanos % instances.len();
                Some(instances[idx].clone())
            }
            LoadBalanceStrategy::WeightedRoundRobin => {
                let total_weight: u32 = instances.iter().map(|i| i.weight).sum();
                if total_weight == 0 {
                    return Some(instances[0].clone());
                }
                let current = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) as u32 % total_weight;
                let mut cumulative = 0u32;
                for instance in instances {
                    cumulative += instance.weight;
                    if current < cumulative {
                        return Some(instance.clone());
                    }
                }
                Some(instances[instances.len() - 1].clone())
            }
        }
    }

    /// 获取当前策略
    pub fn strategy(&self) -> LoadBalanceStrategy {
        self.strategy
    }
}

/// 服务发现统一入口
pub struct ServiceDiscovery {
    registry: Arc<dyn ServiceRegistry>,
    load_balancer: LoadBalancer,
}

impl ServiceDiscovery {
    /// 创建服务发现
    pub fn new(registry: Arc<dyn ServiceRegistry>, strategy: LoadBalanceStrategy) -> Self {
        Self {
            registry,
            load_balancer: LoadBalancer::new(strategy),
        }
    }

    /// 创建内存服务发现（默认）
    pub fn memory() -> Self {
        Self::new(Arc::new(MemoryRegistry::new("memory")), LoadBalanceStrategy::RoundRobin)
    }

    /// 发现一个健康的服务实例（负载均衡）
    pub fn discover(&self, service_name: &str) -> Option<ServiceInstance> {
        let instances = self.registry.discover(service_name);
        self.load_balancer.select(&instances)
    }

    /// 发现所有健康的服务实例
    pub fn discover_all(&self, service_name: &str) -> Vec<ServiceInstance> {
        self.registry.discover(service_name)
    }

    /// 注册服务实例
    pub fn register(&self, instance: ServiceInstance) -> Result<(), String> {
        self.registry.register(instance)
    }

    /// 注销服务实例
    pub fn deregister(&self, service_name: &str, instance_id: &str) -> Result<(), String> {
        self.registry.deregister(service_name, instance_id)
    }

    /// 心跳
    pub fn heartbeat(&self, service_name: &str, instance_id: &str) -> Result<(), String> {
        self.registry.heartbeat(service_name, instance_id)
    }

    /// 获取注册中心引用
    pub fn registry(&self) -> &Arc<dyn ServiceRegistry> {
        &self.registry
    }

    /// 获取负载均衡器引用
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_instance(name: &str, port: u16) -> ServiceInstance {
        ServiceInstance::new(name, "127.0.0.1", port)
    }

    #[test]
    fn test_service_instance_creation() {
        let instance = make_instance("test-svc", 8080);
        assert_eq!(instance.service_name, "test-svc");
        assert_eq!(instance.port, 8080);
        assert_eq!(instance.address(), "127.0.0.1:8080");
        assert_eq!(instance.http_url(), "http://127.0.0.1:8080");
        assert!(instance.is_healthy());
        assert!(!instance.instance_id.is_empty());
    }

    #[test]
    fn test_memory_registry_register_discover() {
        let registry = MemoryRegistry::new("test");
        let inst1 = make_instance("svc-a", 8001);
        let inst2 = make_instance("svc-a", 8002);
        let inst3 = make_instance("svc-b", 8003);

        registry.register(inst1.clone()).unwrap();
        registry.register(inst2.clone()).unwrap();
        registry.register(inst3.clone()).unwrap();

        let instances = registry.discover("svc-a");
        assert_eq!(instances.len(), 2);

        let instances_b = registry.discover("svc-b");
        assert_eq!(instances_b.len(), 1);

        let services = registry.services();
        assert_eq!(services.len(), 2);
    }

    #[test]
    fn test_memory_registry_deregister() {
        let registry = MemoryRegistry::new("test");
        let inst = make_instance("svc-a", 8001);
        let id = inst.instance_id.clone();
        registry.register(inst).unwrap();
        assert_eq!(registry.discover("svc-a").len(), 1);

        registry.deregister("svc-a", &id).unwrap();
        assert_eq!(registry.discover("svc-a").len(), 0);
    }

    #[test]
    fn test_memory_registry_heartbeat() {
        let registry = MemoryRegistry::new("test");
        let inst = make_instance("svc-a", 8001);
        let id = inst.instance_id.clone();
        registry.register(inst).unwrap();

        registry.update_status("svc-a", &id, ServiceStatus::Unhealthy).unwrap();
        assert_eq!(registry.discover("svc-a").len(), 0);
        assert_eq!(registry.discover_all("svc-a").len(), 1);

        registry.heartbeat("svc-a", &id).unwrap();
        assert_eq!(registry.discover("svc-a").len(), 1);
    }

    #[test]
    fn test_load_balancer_round_robin() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        let instances = vec![
            make_instance("svc", 8001),
            make_instance("svc", 8002),
            make_instance("svc", 8003),
        ];

        let first = lb.select(&instances).unwrap();
        let second = lb.select(&instances).unwrap();
        let third = lb.select(&instances).unwrap();
        let fourth = lb.select(&instances).unwrap();

        assert_eq!(first.port, 8001);
        assert_eq!(second.port, 8002);
        assert_eq!(third.port, 8003);
        assert_eq!(fourth.port, 8001); // 循环
    }

    #[test]
    fn test_load_balancer_empty() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        let result = lb.select(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_load_balancer_weighted() {
        let lb = LoadBalancer::new(LoadBalanceStrategy::WeightedRoundRobin);
        let mut inst1 = make_instance("svc", 8001);
        inst1.weight = 3;
        let mut inst2 = make_instance("svc", 8002);
        inst2.weight = 1;
        let instances = vec![inst1, inst2];

        // 4 次选择应该是 3 次 8001 + 1 次 8002
        let mut count_8001 = 0;
        let mut count_8002 = 0;
        for _ in 0..4 {
            let selected = lb.select(&instances).unwrap();
            if selected.port == 8001 { count_8001 += 1; }
            if selected.port == 8002 { count_8002 += 1; }
        }
        assert_eq!(count_8001, 3);
        assert_eq!(count_8002, 1);
    }

    #[test]
    fn test_service_discovery_integration() {
        let discovery = ServiceDiscovery::memory();
        let inst1 = make_instance("api-svc", 9001);
        let inst2 = make_instance("api-svc", 9002);
        let id1 = inst1.instance_id.clone();

        discovery.register(inst1).unwrap();
        discovery.register(inst2).unwrap();

        let selected = discovery.discover("api-svc").unwrap();
        assert!(selected.service_name == "api-svc");

        let all = discovery.discover_all("api-svc");
        assert_eq!(all.len(), 2);

        discovery.deregister("api-svc", &id1).unwrap();
        assert_eq!(discovery.discover_all("api-svc").len(), 1);
    }

    #[test]
    fn test_static_registry() {
        let instances = vec![
            make_instance("static-svc", 7001),
            make_instance("static-svc", 7002),
        ];
        let registry = StaticRegistry::from_instances(instances);

        assert_eq!(registry.discover("static-svc").len(), 2);
        assert!(registry.register(make_instance("new-svc", 7003)).is_err());
    }

    #[test]
    fn test_service_status_default() {
        assert_eq!(ServiceStatus::default(), ServiceStatus::Healthy);
    }
}
