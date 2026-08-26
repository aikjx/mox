'use strict';

/**
 * MOX Enterprise · 模块统一索引
 * =============================
 * 所有企业级模块的统一注册清单，将 A–O 已交付的 34 个文件
 * 纳入模块化系统的统一管理
 *
 * 使用方式：
 *   const { AppBootstrap } = require('./src/enterprise/module-system/app-bootstrap');
 *   const { registerAllModules } = require('./src/enterprise/module-system/module-index');
 *
 *   const app = new AppBootstrap({ appName: 'MOX Enterprise' });
 *   registerAllModules(app);
 *   await app.bootstrap();
 */

const path = require('path');

// ─── 模块分类 ───
const MODULE_CATEGORY = {
  CORE: 'core',
  STORAGE: 'storage',
  COMPUTE: 'compute',
  SECURITY: 'security',
  OBSERVABILITY: 'observability',
  MULTI_REGION: 'multi_region',
  DATA_LAKE: 'data_lake',
  FINOPS: 'finops',
  MULTI_TENANT: 'multi_tenant',
  DISASTER_RECOVERY: 'disaster_recovery',
  DEVOPS: 'devops',
  MODULE_SYSTEM: 'module_system',
};

// ─── 完整模块清单（A–O + 模块系统） ───
const MODULE_MANIFEST = [
  // ═══ 模块系统自身（P 轮新增） ═══
  {
    name: 'module-registry',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '模块注册中心：统一注册、发现、查询所有模块',
    path: './module-registry.js',
    capabilities: ['module-discovery', 'module-registration', 'capability-indexing'],
    tags: ['core', 'module-system'],
    dependencies: [],
  },
  {
    name: 'di-container',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '依赖注入容器：管理服务实例生命周期与依赖解析',
    path: './di-container.js',
    capabilities: ['dependency-injection', 'service-lifecycle', 'scope-management'],
    tags: ['core', 'module-system'],
    dependencies: [],
  },
  {
    name: 'enterprise-config',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '统一企业级配置中心：多环境、热更新、加密、Schema校验',
    path: './enterprise-config.js',
    capabilities: ['configuration-management', 'hot-reload', 'config-encryption', 'schema-validation'],
    tags: ['core', 'module-system', 'config'],
    dependencies: [],
  },
  {
    name: 'dependency-graph',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '依赖图与拓扑排序：DAG构建、循环检测、并行启动组',
    path: './dependency-graph.js',
    capabilities: ['topological-sort', 'cycle-detection', 'impact-analysis', 'graph-visualization'],
    tags: ['core', 'module-system'],
    dependencies: ['module-registry'],
  },
  {
    name: 'module-lifecycle',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '模块生命周期管理器：初始化、启动、健康检查、优雅关闭',
    path: './module-lifecycle.js',
    capabilities: ['lifecycle-management', 'health-check-scheduling', 'graceful-shutdown', 'degraded-mode'],
    tags: ['core', 'module-system'],
    dependencies: ['module-registry', 'di-container', 'enterprise-config', 'dependency-graph'],
  },
  {
    name: 'event-bus',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '模块间事件总线：发布订阅、通配符、死信队列、RPC',
    path: './event-bus.js',
    capabilities: ['pub-sub', 'event-sourcing', 'dead-letter-queue', 'rpc-over-events'],
    tags: ['core', 'module-system', 'messaging'],
    dependencies: [],
  },
  {
    name: 'health-aggregator',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '健康检查聚合器：liveness/readiness/startup/deep、K8s探针',
    path: './health-aggregator.js',
    capabilities: ['health-check', 'k8s-probes', 'availability-monitoring', 'health-history'],
    tags: ['core', 'module-system', 'observability'],
    dependencies: [],
  },
  {
    name: 'middleware-assembler',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '中间件装配器：统一注册、排序、组合、热更新Express中间件',
    path: './middleware-assembler.js',
    capabilities: ['middleware-management', 'priority-ordering', 'hot-reload', 'performance-monitoring'],
    tags: ['core', 'module-system', 'http'],
    dependencies: [],
  },
  {
    name: 'router-aggregator',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '路由聚合器：API版本控制、权限声明、OpenAPI文档自动生成',
    path: './router-aggregator.js',
    capabilities: ['route-management', 'api-versioning', 'openapi-generation', 'route-conflict-detection'],
    tags: ['core', 'module-system', 'http'],
    dependencies: ['middleware-assembler'],
  },
  {
    name: 'app-bootstrap',
    version: '1.0.0',
    category: MODULE_CATEGORY.MODULE_SYSTEM,
    description: '应用启动编排器：6阶段启动流程、优雅关闭、信号处理',
    path: './app-bootstrap.js',
    capabilities: ['application-lifecycle', 'bootstrap-orchestration', 'graceful-shutdown', 'signal-handling'],
    tags: ['core', 'module-system'],
    dependencies: ['module-registry', 'di-container', 'enterprise-config', 'module-lifecycle', 'event-bus', 'health-aggregator', 'middleware-assembler', 'router-aggregator'],
  },

  // ═══ A轮：架构与迁移 ═══
  {
    name: 'pg-shard-router',
    version: '1.0.0',
    category: MODULE_CATEGORY.STORAGE,
    description: 'PostgreSQL分库路由：一致性哈希1024 vnode、CRC32分片、连接池、双写过渡',
    path: '../pg-shard/pg-shard-router.js',
    capabilities: ['db-sharding', 'consistent-hashing', 'connection-pooling', 'dual-write'],
    tags: ['database', 'postgresql', 'sharding', 'migration'],
    dependencies: ['enterprise-config'],
  },
  {
    name: 's3-switch',
    version: '1.0.0',
    category: MODULE_CATEGORY.STORAGE,
    description: 'FS↔S3切换工具：迁移、校验、状态查看、配置模板生成',
    path: '../../../scripts/migration/s3-switch.js',
    capabilities: ['storage-migration', 'fs-to-s3', 'data-verification'],
    tags: ['storage', 's3', 'migration'],
    dependencies: ['enterprise-config'],
  },
  {
    name: 'sqlite-to-pg',
    version: '1.0.0',
    category: MODULE_CATEGORY.STORAGE,
    description: 'SQLite→PostgreSQL迁移工具：自动表结构映射、批量迁移、双重校验',
    path: '../../../scripts/migration/sqlite-to-pg.js',
    capabilities: ['db-migration', 'sqlite-to-pg', 'schema-mapping'],
    tags: ['database', 'migration', 'sqlite', 'postgresql'],
    dependencies: ['enterprise-config'],
  },

  // ═══ F轮：K8s与可观测 ═══
  {
    name: 'prometheus-rules',
    version: '1.0.0',
    category: MODULE_CATEGORY.OBSERVABILITY,
    description: 'Prometheus告警规则：20+条、5大类、P0-P2三级',
    path: '../../observability/prometheus-rules.yaml',
    capabilities: ['alerting-rules', 'prometheus'],
    tags: ['observability', 'prometheus', 'alerting'],
    dependencies: [],
  },
  {
    name: 'alertmanager-config',
    version: '1.0.0',
    category: MODULE_CATEGORY.OBSERVABILITY,
    description: 'AlertManager路由配置：P0电话+短信+钉钉、P1钉钉+邮件、告警抑制',
    path: '../../observability/alertmanager-config.yaml',
    capabilities: ['alert-routing', 'notification-management', 'alert-inhibition'],
    tags: ['observability', 'alerting', 'alertmanager'],
    dependencies: [],
  },

  // ═══ H轮：分布式计算 ═══
  {
    name: 'spark-gc-job',
    version: '1.0.0',
    category: MODULE_CATEGORY.COMPUTE,
    description: 'Spark分布式GC：256分片并行扫描、S3批量删除、GC报告',
    path: '../../compute/spark-jobs/spark-gc-job.py',
    capabilities: ['distributed-gc', 'spark', 'parallel-processing'],
    tags: ['compute', 'spark', 'garbage-collection'],
    dependencies: [],
  },
  {
    name: 'spark-verify-job',
    version: '1.0.0',
    category: MODULE_CATEGORY.COMPUTE,
    description: 'Spark分布式校验：内容SHA-256重算、存在性、EC校验、抽样',
    path: '../../compute/spark-jobs/spark-verify-job.py',
    capabilities: ['distributed-verification', 'data-integrity', 'ec-validation'],
    tags: ['compute', 'spark', 'verification'],
    dependencies: [],
  },

  // ═══ I轮：安全 ═══
  {
    name: 'rbac-middleware',
    version: '1.0.0',
    category: MODULE_CATEGORY.SECURITY,
    description: 'RBAC权限中间件：7种角色、权限通配符、租户隔离、API Key认证',
    path: '../../security/rbac-middleware.js',
    capabilities: ['rbac', 'authentication', 'authorization', 'tenant-isolation'],
    tags: ['security', 'rbac', 'auth'],
    dependencies: ['enterprise-config'],
  },
  {
    name: 'audit-logger',
    version: '1.0.0',
    category: MODULE_CATEGORY.SECURITY,
    description: '哈希链审计日志：SHA-256防篡改、异步批量、ClickHouse热存+S3冷存',
    path: '../../security/audit-logger.js',
    capabilities: ['audit-logging', 'tamper-proof', 'hash-chain'],
    tags: ['security', 'audit', 'compliance'],
    dependencies: ['enterprise-config', 'event-bus'],
  },
  {
    name: 'encryption-utils',
    version: '1.0.0',
    category: MODULE_CATEGORY.SECURITY,
    description: '全算法加密工具：AES-256-GCM、RSA-4096、ECDSA P-384、PBKDF2、密钥环轮换',
    path: '../../security/encryption-utils.js',
    capabilities: ['encryption', 'key-management', 'digital-signature', 'key-rotation'],
    tags: ['security', 'encryption', 'cryptography'],
    dependencies: ['enterprise-config'],
  },

  // ═══ K轮：多Region ═══
  {
    name: 'crr-sync-manager',
    version: '1.0.0',
    category: MODULE_CATEGORY.MULTI_REGION,
    description: '跨Region同步管理器：异步队列、断点续传、批量复制、指数退避、限流',
    path: '../multi-region/crr-sync-manager.js',
    capabilities: ['cross-region-replication', 'async-sync', 'resume-transfer', 'rate-limiting'],
    tags: ['multi-region', 'replication', 'dr'],
    dependencies: ['enterprise-config', 'event-bus'],
  },
  {
    name: 'conflict-resolver',
    version: '1.0.0',
    category: MODULE_CATEGORY.MULTI_REGION,
    description: '冲突解决器：LWW、向量时钟、源优先级、人工介入、类型兼容性矩阵',
    path: '../multi-region/conflict-resolver.js',
    capabilities: ['conflict-resolution', 'vector-clocks', 'last-write-wins'],
    tags: ['multi-region', 'consistency', 'conflict-resolution'],
    dependencies: [],
  },
  {
    name: 'read-repair',
    version: '1.0.0',
    category: MODULE_CATEGORY.MULTI_REGION,
    description: '读修复：ONE/QUORUM/ALL三级一致性、后台anti-entropy巡检、修复队列',
    path: '../multi-region/read-repair.js',
    capabilities: ['read-repair', 'quorum-consistency', 'anti-entropy'],
    tags: ['multi-region', 'consistency', 'repair'],
    dependencies: ['event-bus'],
  },

  // ═══ L轮：数据湖 ═══
  {
    name: 'iceberg-writer',
    version: '1.0.0',
    category: MODULE_CATEGORY.DATA_LAKE,
    description: 'Iceberg写入器：5张表Schema、Parquet+Zstd、分区、快照、ACID',
    path: '../../compute/data-lake/iceberg-writer.js',
    capabilities: ['iceberg-write', 'parquet', 'acid-transactions', 'schema-management'],
    tags: ['data-lake', 'iceberg', 'parquet'],
    dependencies: ['enterprise-config'],
  },
  {
    name: 'iceberg-query',
    version: '1.0.0',
    category: MODULE_CATEGORY.DATA_LAKE,
    description: 'Iceberg查询引擎：SQL解析、时间旅行、增量读取、谓词下推、多执行模式',
    path: '../../compute/data-lake/iceberg-query.js',
    capabilities: ['iceberg-query', 'time-travel', 'incremental-read', 'predicate-pushdown'],
    tags: ['data-lake', 'iceberg', 'query'],
    dependencies: ['iceberg-writer'],
  },
  {
    name: 'schema-evolution',
    version: '1.0.0',
    category: MODULE_CATEGORY.DATA_LAKE,
    description: 'Schema演进管理器：加/删/改列、改类型、分区变更、安全验证、回滚',
    path: '../../compute/data-lake/schema-evolution.js',
    capabilities: ['schema-evolution', 'backward-compatibility', 'schema-rollback'],
    tags: ['data-lake', 'iceberg', 'schema'],
    dependencies: ['iceberg-writer'],
  },

  // ═══ M轮：FinOps ═══
  {
    name: 'cost-collector',
    version: '1.0.0',
    category: MODULE_CATEGORY.FINOPS,
    description: 'FinOps成本采集器：多云采集、K8s成本估算、Iceberg写入、成本预测',
    path: '../finops/cost-collector.js',
    capabilities: ['cost-collection', 'multi-cloud', 'cost-forecasting', 'k8s-cost'],
    tags: ['finops', 'cost', 'cloud'],
    dependencies: ['enterprise-config', 'iceberg-writer'],
  },
  {
    name: 'budget-alerter',
    version: '1.0.0',
    category: MODULE_CATEGORY.FINOPS,
    description: '预算告警器：5级预算范围、4级告警阈值、超预算动作、通知渠道',
    path: '../finops/budget-alerter.js',
    capabilities: ['budget-management', 'cost-alerting', 'threshold-monitoring'],
    tags: ['finops', 'budget', 'alerting'],
    dependencies: ['cost-collector', 'event-bus'],
  },
  {
    name: 'optimization-recommender',
    version: '1.0.0',
    category: MODULE_CATEGORY.FINOPS,
    description: '成本优化建议器：6维度12条建议、优先级排序、投资回收期、实施跟踪',
    path: '../finops/optimization-recommender.js',
    capabilities: ['cost-optimization', 'recommendation-engine', 'savings-tracking'],
    tags: ['finops', 'optimization', 'cost'],
    dependencies: ['cost-collector'],
  },

  // ═══ N轮：多租户 ═══
  {
    name: 'quota-manager',
    version: '1.0.0',
    category: MODULE_CATEGORY.MULTI_TENANT,
    description: '多租户配额管理器：6维度配额、4套餐、QPS滑动窗口限流、Express中间件',
    path: '../multi-tenant/quota-manager.js',
    capabilities: ['quota-management', 'rate-limiting', 'tenant-isolation', 'plan-management'],
    tags: ['multi-tenant', 'quota', 'rate-limiting'],
    dependencies: ['enterprise-config', 'event-bus'],
  },
  {
    name: 'usage-metering',
    version: '1.0.0',
    category: MODULE_CATEGORY.MULTI_TENANT,
    description: '用量采集器：8用量类型、4聚合粒度、Redis分布式计数、Iceberg持久化',
    path: '../multi-tenant/usage-metering.js',
    capabilities: ['usage-metering', 'real-time-counting', 'aggregation', 'trend-analysis'],
    tags: ['multi-tenant', 'metering', 'billing'],
    dependencies: ['enterprise-config', 'iceberg-writer'],
  },
  {
    name: 'billing-engine',
    version: '1.0.0',
    category: MODULE_CATEGORY.MULTI_TENANT,
    description: '计费引擎：5种计费模式、阶梯定价、订阅+按量混合、预付费余额、支付网关',
    path: '../multi-tenant/billing-engine.js',
    capabilities: ['billing', 'invoicing', 'tiered-pricing', 'payment-processing'],
    tags: ['multi-tenant', 'billing', 'commerce'],
    dependencies: ['usage-metering', 'quota-manager', 'enterprise-config'],
  },

  // ═══ O轮：备份灾备 ═══
  {
    name: 'backup-manager',
    version: '1.0.0',
    category: MODULE_CATEGORY.DISASTER_RECOVERY,
    description: '备份管理器：全量+增量WAL+快照、跨Region复制、自动清理、恢复验证',
    path: '../disaster-recovery/backup-manager.js',
    capabilities: ['backup-management', 'incremental-backup', 'snapshot', 'cross-region-backup', 'restore'],
    tags: ['backup', 'dr', 'recovery'],
    dependencies: ['enterprise-config', 'event-bus'],
  },
  {
    name: 'dr-drill',
    version: '1.0.0',
    category: MODULE_CATEGORY.DISASTER_RECOVERY,
    description: 'DR灾难恢复演练脚本：5场景（节点/AZ/Region/数据损坏/全量恢复）、RTO/RPO测量',
    path: '../../../scripts/disaster-recovery/dr-drill.sh',
    capabilities: ['dr-drill', 'chaos-engineering', 'rto-rpo-measurement'],
    tags: ['dr', 'chaos', '演练'],
    dependencies: ['backup-manager'],
  },
];

/**
 * 注册所有模块到应用
 * @param {object} app  AppBootstrap 实例
 */
function registerAllModules(app) {
  const registered = [];
  const failed = [];

  for (const descriptor of MODULE_MANIFEST) {
    try {
      // 构建完整的模块描述符
      const moduleDescriptor = {
        name: descriptor.name,
        version: descriptor.version,
        description: descriptor.description,
        category: descriptor.category,
        dependencies: descriptor.dependencies,
        capabilities: descriptor.capabilities,
        tags: descriptor.tags,
        config: {},
        // 延迟加载：init 时才 require 模块文件
        init: async (context) => {
          try {
            const modulePath = path.resolve(__dirname, descriptor.path);
            const moduleExports = require(modulePath);
            return moduleExports;
          } catch (err) {
            context.eventBus?.publish('module.load_error', {
              module: descriptor.name,
              error: err.message,
            }, { priority: 1 });
            // 非核心模块加载失败不阻断启动
            if (descriptor.category !== MODULE_CATEGORY.MODULE_SYSTEM) {
              return { _loadError: err.message, _degraded: true };
            }
            throw err;
          }
        },
        start: async (instance) => {
          // 如果模块有 start 方法则调用
          if (instance && typeof instance.start === 'function') {
            await instance.start();
          }
        },
        stop: async (instance) => {
          if (instance && typeof instance.stop === 'function') {
            await instance.stop();
          }
        },
        healthCheck: async (instance) => {
          if (instance && typeof instance.healthCheck === 'function') {
            return instance.healthCheck();
          }
          if (instance && instance._degraded) {
            return { status: 'degraded', details: { loadError: instance._loadError } };
          }
          return { status: 'healthy' };
        },
      };

      app.registerModule(moduleDescriptor);
      registered.push(descriptor.name);
    } catch (err) {
      failed.push({ name: descriptor.name, error: err.message });
    }
  }

  return {
    total: MODULE_MANIFEST.length,
    registered: registered.length,
    failed: failed.length,
    failedModules: failed,
    categories: Object.values(MODULE_CATEGORY),
  };
}

/**
 * 获取模块清单（用于文档/诊断）
 */
function getManifest() {
  return {
    generatedAt: new Date().toISOString(),
    totalModules: MODULE_MANIFEST.length,
    categories: MODULE_CATEGORY,
    byCategory: MODULE_MANIFEST.reduce((acc, m) => {
      acc[m.category] = (acc[m.category] || 0) + 1;
      return acc;
    }, {}),
    modules: MODULE_MANIFEST.map(m => ({
      name: m.name,
      version: m.version,
      category: m.category,
      description: m.description,
      dependencies: m.dependencies,
      capabilities: m.capabilities,
      tags: m.tags,
    })),
  };
}

/**
 * 按分类获取模块
 */
function getModulesByCategory(category) {
  return MODULE_MANIFEST.filter(m => m.category === category);
}

/**
 * 获取模块依赖图（用于可视化）
 */
function getDependencyGraphData() {
  const nodes = MODULE_MANIFEST.map(m => ({
    id: m.name,
    label: m.name,
    category: m.category,
    version: m.version,
  }));

  const edges = [];
  for (const m of MODULE_MANIFEST) {
    for (const dep of m.dependencies) {
      edges.push({ source: m.name, target: dep, type: 'required' });
    }
  }

  return { nodes, edges };
}

module.exports = {
  MODULE_MANIFEST,
  MODULE_CATEGORY,
  registerAllModules,
  getManifest,
  getModulesByCategory,
  getDependencyGraphData,
};
