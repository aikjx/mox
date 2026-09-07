# 全仓代码模块目录

由 `python tools/module_catalog.py` 从 Cargo 元数据与目录生成；请勿手改。

本目录记录代码归属和入口，不代表功能验收、生产可用性或部署就绪。运行时依赖包含可选依赖；开发和构建依赖分别列出。

返回 [模块导航](README.md) · [API 注册表](../API-REGISTRY.md) · [端口注册表](../api/PORT-REGISTRY.md)

## 后端模块

当前 workspace：**143** 个 crate。

### ai（12）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-ai-agent-svc](<../../platform/domains/ai/svc/mox-ai-agent-svc/Cargo.toml>) | svc | — | dev: mox-ai-flow-svc, mox-market-template-svc<br>runtime: mox-kg-algo-core, mox-kg-sdk, mox-platform-foundation, mox-platform-operator-core, mox-platform-system-core |
| [mox-ai-alliance-engine](<../../platform/domains/ai/core/mox-ai-alliance-engine/Cargo.toml>) | core | — | runtime: mox-ai-expert-core, mox-ai-expert-proto, mox-audit, mox-pipeline-framework, mox-unified-contract |
| [mox-ai-api](<../../platform/domains/ai/api/Cargo.toml>) | api | — | — |
| [mox-ai-core](<../../platform/domains/ai/core/mox-ai-core/Cargo.toml>) | core | — | runtime: mox-platform-model-core |
| [mox-ai-expert-core](<../../platform/domains/ai/core/mox-ai-expert-core/Cargo.toml>) | core | — | runtime: mox-ai-expert-proto, mox-ai-flow-core, mox-audit, mox-error, mox-platform-foundation |
| [mox-ai-expert-proto](<../../platform/domains/ai/proto/mox-ai-expert-proto/Cargo.toml>) | proto | — | runtime: mox-error, mox-platform-foundation |
| [mox-ai-expert-svc](<../../platform/domains/ai/svc/mox-ai-expert-svc/Cargo.toml>) | svc | mox | runtime: mox-ai-expert-proto, mox-ai-flow-svc, mox-kg-sdk, mox-platform-foundation |
| [mox-ai-flow-core](<../../platform/domains/ai/core/mox-ai-flow-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |
| [mox-ai-flow-sdk](<../../platform/domains/ai/sdk/mox-ai-flow-sdk/Cargo.toml>) | sdk | — | runtime: mox-ai-flow-core |
| [mox-ai-flow-svc](<../../platform/domains/ai/svc/mox-ai-flow-svc/Cargo.toml>) | svc | flowopt | runtime: mox-ai-flow-core, mox-platform-foundation |
| [mox-ai-intent-core](<../../platform/domains/ai/core/mox-ai-intent-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |
| [mox-ai-intent-svc](<../../platform/domains/ai/svc/mox-ai-intent-svc/Cargo.toml>) | svc | mox-ai-intent-svc | runtime: mox-ai-api, mox-ai-intent-core, mox-framework, mox-platform-foundation |

### alliance（13）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-alliance-api](<../../platform/domains/alliance/api/Cargo.toml>) | api | — | runtime: mox-alliance-common-proto, mox-alliance-executor-proto, mox-alliance-scheduler-proto, mox-error, mox-platform-foundation |
| [mox-alliance-boot-config](<../../platform/domains/alliance/core/mox-alliance-boot-config/Cargo.toml>) | core | — | runtime: mox-alliance-common-proto |
| [mox-alliance-common-proto](<../../platform/domains/alliance/proto/mox-alliance-common-proto/Cargo.toml>) | proto | — | runtime: mox-error, mox-platform-foundation |
| [mox-alliance-config-core](<../../platform/domains/alliance/core/mox-alliance-config-core/Cargo.toml>) | core | — | runtime: mox-alliance-common-proto, mox-error, mox-platform-foundation |
| [mox-alliance-core](<../../platform/domains/alliance/core/mox-alliance-core/Cargo.toml>) | core | — | runtime: mox-alliance-common-proto, mox-error, mox-platform-foundation |
| [mox-alliance-executor-core](<../../platform/domains/alliance/core/mox-alliance-executor-core/Cargo.toml>) | core | — | dev: mox-alliance-scheduler-core, mox-alliance-scheduler-proto<br>runtime: mox-ai-expert-proto, mox-alliance-common-proto, mox-alliance-core, mox-alliance-executor-proto, mox-error, mox-platform-foundation |
| [mox-alliance-executor-proto](<../../platform/domains/alliance/proto/mox-alliance-executor-proto/Cargo.toml>) | proto | — | runtime: mox-alliance-common-proto, mox-error, mox-platform-foundation |
| [mox-alliance-executor-svc](<../../platform/domains/alliance/svc/mox-alliance-executor-svc/Cargo.toml>) | svc | mox-alliance-executor | runtime: mox-ai-expert-proto, mox-ai-expert-svc, mox-alliance-api, mox-alliance-boot-config, mox-alliance-common-proto, mox-alliance-executor-core, mox-alliance-executor-proto |
| [mox-alliance-http-sdk](<../../platform/domains/alliance/sdk/mox-alliance-http-sdk/Cargo.toml>) | sdk | — | runtime: mox-alliance-api, mox-alliance-common-proto, mox-alliance-scheduler-core, mox-alliance-scheduler-proto, mox-api-protocol |
| [mox-alliance-scheduler-core](<../../platform/domains/alliance/core/mox-alliance-scheduler-core/Cargo.toml>) | core | — | runtime: mox-alliance-common-proto, mox-alliance-config-core, mox-alliance-core, mox-alliance-executor-proto, mox-alliance-scheduler-proto, mox-error, mox-platform-foundation |
| [mox-alliance-scheduler-proto](<../../platform/domains/alliance/proto/mox-alliance-scheduler-proto/Cargo.toml>) | proto | — | runtime: mox-alliance-common-proto, mox-error, mox-platform-foundation |
| [mox-alliance-scheduler-svc](<../../platform/domains/alliance/svc/mox-alliance-scheduler-svc/Cargo.toml>) | svc | mox-alliance-scheduler | runtime: mox-alliance-api, mox-alliance-boot-config, mox-alliance-common-proto, mox-alliance-config-core, mox-alliance-executor-proto, mox-alliance-scheduler-core, mox-alliance-scheduler-proto |
| [mox-alliance-sdk](<../../platform/domains/alliance/sdk/mox-alliance-sdk/Cargo.toml>) | sdk | — | runtime: mox-alliance-api, mox-alliance-common-proto, mox-alliance-executor-proto, mox-alliance-scheduler-proto, mox-error, mox-platform-foundation |

### base（7）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-base-graph-core](<../../platform/domains/base/mox-base-graph-core/Cargo.toml>) | mox-base-graph-core | — | runtime: mox-base-model-core |
| [mox-base-index-core](<../../platform/domains/base/mox-base-index-core/Cargo.toml>) | mox-base-index-core | — | — |
| [mox-base-lifecycle-core](<../../platform/domains/base/mox-base-lifecycle-core/Cargo.toml>) | mox-base-lifecycle-core | — | runtime: mox-base-store-core |
| [mox-base-model-core](<../../platform/domains/base/mox-base-model-core/Cargo.toml>) | mox-base-model-core | — | — |
| [mox-base-perm-core](<../../platform/domains/base/mox-base-perm-core/Cargo.toml>) | mox-base-perm-core | — | runtime: mox-base-model-core, mox-rbac-engine |
| [mox-base-query-core](<../../platform/domains/base/mox-base-query-core/Cargo.toml>) | mox-base-query-core | — | runtime: mox-base-graph-core, mox-base-index-core, mox-base-model-core |
| [mox-base-store-core](<../../platform/domains/base/mox-base-store-core/Cargo.toml>) | mox-base-store-core | — | — |

### cloud（13）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-cloud-admin-sdk](<../../platform/domains/cloud/sdk/mox-cloud-admin-sdk/Cargo.toml>) | sdk | — | runtime: mox-base-store-core, mox-cloud-api, mox-cloud-store-core |
| [mox-cloud-api](<../../platform/domains/cloud/api/Cargo.toml>) | api | — | — |
| [mox-cloud-domain-traits](<../../platform/domains/cloud/core/mox-cloud-domain-traits/Cargo.toml>) | core | — | — |
| [mox-cloud-filer-svc](<../../platform/domains/cloud/svc/mox-cloud-filer-svc/Cargo.toml>) | svc | — | runtime: mox-base-store-core, mox-cloud-domain-traits, mox-cloud-kernel, mox-cloud-store-core |
| [mox-cloud-kb-core](<../../platform/domains/cloud/core/mox-cloud-kb-core/Cargo.toml>) | core | — | dev: mox-ai-alliance-engine, mox-config-core, mox-unified-contract<br>runtime: mox-unified-contract |
| [mox-cloud-kernel](<../../platform/domains/cloud/core/mox-cloud-kernel/Cargo.toml>) | core | — | — |
| [mox-cloud-master-svc](<../../platform/domains/cloud/svc/mox-cloud-master-svc/Cargo.toml>) | svc | — | dev: mox-cloud-volume-svc<br>runtime: mox-cloud-foundation |
| [mox-cloud-rebalance-svc](<../../platform/domains/cloud/svc/mox-cloud-rebalance-svc/Cargo.toml>) | svc | — | runtime: mox-cloud-foundation, mox-cloud-master-svc, mox-cloud-volume-svc |
| [mox-cloud-s3-svc](<../../platform/domains/cloud/svc/mox-cloud-s3-svc/Cargo.toml>) | svc | — | runtime: mox-cloud-domain-traits, mox-cloud-foundation, mox-cloud-kernel, mox-cloud-master-svc, mox-cloud-store-core, mox-data-standards-core |
| [mox-cloud-sdk](<../../platform/domains/cloud/sdk/mox-cloud-sdk/Cargo.toml>) | sdk | — | — |
| [mox-cloud-server](<../../platform/domains/cloud/svc/mox-cloud-server/Cargo.toml>) | svc | mox-cloud-server | runtime: mox-cache-core, mox-cloud-kernel, mox-cloud-master-svc, mox-cloud-store-core, mox-server-runtime |
| [mox-cloud-store-core](<../../platform/domains/cloud/core/mox-cloud-store-core/Cargo.toml>) | core | — | runtime: mox-base-store-core, mox-cloud-foundation, mox-cloud-kernel |
| [mox-cloud-volume-svc](<../../platform/domains/cloud/svc/mox-cloud-volume-svc/Cargo.toml>) | svc | — | runtime: mox-cloud-domain-traits, mox-cloud-foundation, mox-cloud-kernel |

### data（10）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-data-api](<../../platform/domains/data/api/Cargo.toml>) | api | — | — |
| [mox-data-catalog-svc](<../../platform/domains/data/svc/mox-data-catalog-svc/Cargo.toml>) | svc | catalog | runtime: mox-ai-expert-svc, mox-ai-flow-sdk, mox-platform-foundation |
| [mox-data-compliance-svc](<../../platform/domains/data/svc/mox-data-compliance-svc/Cargo.toml>) | svc | — | — |
| [mox-data-etl-svc](<../../platform/domains/data/svc/mox-data-etl-svc/Cargo.toml>) | svc | — | — |
| [mox-data-formula-core](<../../platform/domains/data/core/mox-data-formula-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |
| [mox-data-formula-native](<../../platform/domains/data/sdk/mox-data-formula-native/Cargo.toml>) | sdk | — | runtime: mox-data-formula-core |
| [mox-data-norm-core](<../../platform/domains/data/core/mox-data-norm-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |
| [mox-data-norm-intent-native](<../../platform/domains/data/sdk/mox-data-norm-intent-native/Cargo.toml>) | sdk | — | runtime: mox-ai-intent-core, mox-data-formula-core, mox-data-norm-core |
| [mox-data-plane-svc](<../../platform/domains/data/svc/mox-data-plane-svc/Cargo.toml>) | svc | — | — |
| [mox-data-standards-core](<../../platform/domains/data/core/mox-data-standards-core/Cargo.toml>) | core | — | runtime: mox-cloud-foundation |

### flow（18）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-flow-ai-assistant-core](<../../platform/domains/flow/core/mox-flow-ai-assistant-core/Cargo.toml>) | core | — | — |
| [mox-flow-algo-alliance-core](<../../platform/domains/flow/core/mox-flow-algo-alliance-core/Cargo.toml>) | core | — | runtime: mox-flow-operator-core, mox-platform-foundation |
| [mox-flow-api](<../../platform/domains/flow/api/Cargo.toml>) | api | — | — |
| [mox-flow-bridge-svc](<../../platform/domains/flow/svc/mox-flow-bridge-svc/Cargo.toml>) | svc | bridge_demo | runtime: mox-ai-expert-svc, mox-ai-flow-sdk, mox-platform-foundation |
| [mox-flow-ea-workspace-svc](<../../platform/domains/flow/svc/mox-flow-ea-workspace-svc/Cargo.toml>) | svc | — | runtime: mox-flow-unified-arch-core, mox-platform-foundation, mox-unified-algo-core |
| [mox-flow-fusion-svc](<../../platform/domains/flow/svc/mox-flow-fusion-svc/Cargo.toml>) | svc | mox-flow-fusion-svc | runtime: mox-flow-primiflow-svc, mox-platform-foundation, mox-platform-graph-core |
| [mox-flow-lowcode-core](<../../platform/domains/flow/core/mox-flow-lowcode-core/Cargo.toml>) | core | — | — |
| [mox-flow-operator-core](<../../platform/domains/flow/core/mox-flow-operator-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation, mox-platform-operator-core |
| [mox-flow-operator-wasm-svc](<../../platform/domains/flow/svc/mox-flow-operator-wasm-svc/Cargo.toml>) | svc | — | runtime: mox-flow-operator-core, mox-platform-foundation |
| [mox-flow-optimizer-core](<../../platform/domains/flow/core/mox-flow-optimizer-core/Cargo.toml>) | core | — | runtime: mox-flow-operator-core, mox-platform-foundation |
| [mox-flow-primiflow-svc](<../../platform/domains/flow/svc/mox-flow-primiflow-svc/Cargo.toml>) | svc | — | runtime: mox-ai-flow-sdk, mox-platform-foundation, mox-platform-system-core |
| [mox-flow-unified-arch-core](<../../platform/domains/flow/core/mox-flow-unified-arch-core/Cargo.toml>) | core | — | runtime: mox-flow-algo-alliance-core, mox-flow-unified-meta-core, mox-flow-unified-storage-core, mox-platform-foundation |
| [mox-flow-unified-frontend-core](<../../platform/domains/flow/core/mox-flow-unified-frontend-core/Cargo.toml>) | core | — | — |
| [mox-flow-unified-meta-core](<../../platform/domains/flow/core/mox-flow-unified-meta-core/Cargo.toml>) | core | — | runtime: mox-flow-unified-storage-core, mox-platform-foundation |
| [mox-flow-unified-perm-core](<../../platform/domains/flow/core/mox-flow-unified-perm-core/Cargo.toml>) | core | — | — |
| [mox-flow-unified-platform](<../../platform/domains/flow/core/mox-flow-unified-platform/Cargo.toml>) | core | — | runtime: mox-flow-ai-assistant-core, mox-flow-algo-alliance-core, mox-flow-lowcode-core, mox-flow-unified-arch-core, mox-flow-unified-frontend-core, mox-flow-unified-meta-core, mox-flow-unified-perm-core, mox-flow-unified-process-core, mox-flow-unified-storage-core |
| [mox-flow-unified-process-core](<../../platform/domains/flow/core/mox-flow-unified-process-core/Cargo.toml>) | core | — | — |
| [mox-flow-unified-storage-core](<../../platform/domains/flow/core/mox-flow-unified-storage-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |

### foundation（2）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-pipeline-framework](<../../platform/domains/foundation/mox-pipeline-framework/Cargo.toml>) | mox-pipeline-framework | — | runtime: mox-audit, mox-error |
| [mox-rbac-engine](<../../platform/domains/foundation/mox-rbac-engine/Cargo.toml>) | mox-rbac-engine | — | runtime: mox-audit, mox-error |

### kb（2）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-kb-core](<../../platform/domains/kb/core/mox-kb-core/Cargo.toml>) | core | — | runtime: mox-base-model-core, mox-base-query-core, mox-base-store-core, mox-cache-core |
| [mox-kb-server](<../../platform/domains/kb/svc/mox-kb-server/Cargo.toml>) | svc | mox-kb-server | runtime: mox-cache-core, mox-kb-core, mox-server-runtime |

### kg（12）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-kb-svc](<../../platform/domains/kg/svc/mox-kb-svc/Cargo.toml>) | svc | — | runtime: mox-ai-expert-proto, mox-ai-expert-svc, mox-api-protocol, mox-base-store-core, mox-cloud-store-core, mox-kg-storage-svc |
| [mox-kg-algo-core](<../../platform/domains/kg/core/mox-kg-algo-core/Cargo.toml>) | core | compare_with_node, export_formula | runtime: mox-platform-foundation |
| [mox-kg-api](<../../platform/domains/kg/api/Cargo.toml>) | api | — | runtime: mox-error |
| [mox-kg-fusion-svc](<../../platform/domains/kg/svc/mox-kg-fusion-svc/Cargo.toml>) | svc | — | runtime: mox-kg-service-svc |
| [mox-kg-hub-svc](<../../platform/domains/kg/svc/mox-kg-hub-svc/Cargo.toml>) | svc | — | runtime: mox-kg-algo-core, mox-kg-sdk, mox-platform-foundation, mox-platform-graph-core |
| [mox-kg-meta-core](<../../platform/domains/kg/core/mox-kg-meta-core/Cargo.toml>) | core | — | runtime: mox-cloud-foundation |
| [mox-kg-sdk](<../../platform/domains/kg/sdk/mox-kg-sdk/Cargo.toml>) | sdk | — | — |
| [mox-kg-server](<../../platform/domains/kg/svc/mox-kg-server/Cargo.toml>) | svc | mox-kg-server | runtime: mox-cache-core, mox-kg-algo-core, mox-kg-core, mox-kg-meta-core, mox-kg-service-svc, mox-server-runtime |
| [mox-kg-service-svc](<../../platform/domains/kg/svc/mox-kg-service-svc/Cargo.toml>) | svc | — | runtime: mox-api-protocol, mox-cloud-foundation, mox-framework, mox-kg-algo-core, mox-kg-meta-core, mox-kg-storage-svc |
| [mox-kg-spark-svc](<../../platform/domains/kg/svc/mox-kg-spark-svc/Cargo.toml>) | svc | — | — |
| [mox-kg-storage-svc](<../../platform/domains/kg/svc/mox-kg-storage-svc/Cargo.toml>) | svc | — | runtime: mox-cloud-foundation, mox-kg-algo-core, mox-kg-meta-core |
| [mox-kg-streams-svc](<../../platform/domains/kg/svc/mox-kg-streams-svc/Cargo.toml>) | svc | — | runtime: mox-cloud-foundation, mox-kg-storage-svc |

### market（2）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-market-api](<../../platform/domains/market/api/Cargo.toml>) | api | — | — |
| [mox-market-template-svc](<../../platform/domains/market/svc/mox-market-template-svc/Cargo.toml>) | svc | — | runtime: mox-platform-foundation |

### platform（42）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-api-protocol](<../../platform/foundation/mox-api-protocol/Cargo.toml>) | foundation | — | runtime: mox-error |
| [mox-arch-test](<../../platform/arch-test/Cargo.toml>) | arch-test | — | — |
| [mox-audit](<../../platform/foundation/mox-audit/Cargo.toml>) | foundation | — | runtime: mox-error, mox-platform-foundation |
| [mox-auth-core](<../../platform/shared/mox-auth-core/Cargo.toml>) | shared | — | runtime: mox-unified-contract |
| [mox-cache-core](<../../platform/shared/mox-cache-core/Cargo.toml>) | shared | — | runtime: mox-observability-core |
| [mox-cloud-foundation](<../../platform/foundation/mox-cloud-foundation/Cargo.toml>) | foundation | — | — |
| [mox-config-core](<../../platform/shared/mox-config-core/Cargo.toml>) | shared | — | runtime: mox-unified-contract |
| [mox-connector-core](<../../platform/domains/platform/core/mox-connector-core/Cargo.toml>) | core | — | — |
| [mox-content-publisher](<../../platform/domains/platform/svc/mox-content-publisher/Cargo.toml>) | svc | — | runtime: mox-connector-core, mox-platform-foundation, mox-platform-integration-core |
| [mox-dsql-core](<../../platform/domains/platform/core/mox-dsql-core/Cargo.toml>) | core | — | runtime: mox-cache-core |
| [mox-enterprise-core](<../../platform/domains/platform/core/mox-enterprise-core/Cargo.toml>) | core | — | runtime: mox-framework |
| [mox-error](<../../platform/foundation/mox-error/Cargo.toml>) | foundation | — | — |
| [mox-event-core](<../../platform/shared/mox-event-core/Cargo.toml>) | shared | — | — |
| [mox-framework](<../../platform/foundation/mox-framework/Cargo.toml>) | foundation | — | — |
| [mox-iam-server](<../../platform/domains/platform/svc/mox-iam-server/Cargo.toml>) | svc | mox-iam-server | runtime: mox-auth-core, mox-cache-core, mox-platform-iam-core, mox-server-runtime |
| [mox-kg-core](<../../platform/domains/platform/core/mox-kg-core/Cargo.toml>) | core | — | — |
| [mox-lock-core](<../../platform/shared/mox-lock-core/Cargo.toml>) | shared | — | — |
| [mox-observability-core](<../../platform/shared/mox-observability-core/Cargo.toml>) | shared | — | runtime: mox-unified-contract |
| [mox-platform-api](<../../platform/domains/platform/api/Cargo.toml>) | api | — | — |
| [mox-platform-datastore-core](<../../platform/domains/platform/core/mox-platform-datastore-core/Cargo.toml>) | core | — | — |
| [mox-platform-enterprise-svc](<../../platform/domains/platform/svc/mox-platform-enterprise-svc/Cargo.toml>) | svc | enterprise-svc | runtime: mox-platform-datastore-core, mox-platform-iam-core, mox-platform-meta-core, mox-platform-orchestrator-core |
| [mox-platform-foundation](<../../platform/foundation/mox-platform-foundation/Cargo.toml>) | foundation | — | — |
| [mox-platform-gateway-svc](<../../platform/gateway/mox-platform-gateway-svc/Cargo.toml>) | gateway | mox-server | runtime: mox-ai-expert-svc, mox-ai-flow-svc, mox-alliance-common-proto, mox-alliance-http-sdk, mox-api-protocol, mox-audit, mox-kb-svc, mox-kg-service-svc, mox-platform-api, mox-platform-iam-core |
| [mox-platform-graph-core](<../../platform/domains/platform/core/mox-platform-graph-core/Cargo.toml>) | core | — | — |
| [mox-platform-iam-core](<../../platform/domains/platform/core/mox-platform-iam-core/Cargo.toml>) | core | — | — |
| [mox-platform-integration-core](<../../platform/domains/platform/core/mox-platform-integration-core/Cargo.toml>) | core | — | runtime: mox-connector-core, mox-enterprise-core, mox-framework, mox-platform-model-core, mox-plugin-core |
| [mox-platform-meta-core](<../../platform/domains/platform/core/mox-platform-meta-core/Cargo.toml>) | core | — | — |
| [mox-platform-model-core](<../../platform/domains/platform/core/mox-platform-model-core/Cargo.toml>) | core | — | — |
| [mox-platform-module-core](<../../platform/domains/platform/core/mox-platform-module-core/Cargo.toml>) | core | — | — |
| [mox-platform-observability](<../../platform/foundation/mox-platform-observability/Cargo.toml>) | foundation | — | — |
| [mox-platform-operator-core](<../../platform/domains/platform/core/mox-platform-operator-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |
| [mox-platform-orchestrator-core](<../../platform/domains/platform/core/mox-platform-orchestrator-core/Cargo.toml>) | core | — | runtime: mox-platform-datastore-core, mox-platform-iam-core, mox-platform-meta-core |
| [mox-platform-orchestrator-svc](<../../platform/domains/platform/svc/mox-platform-orchestrator-svc/Cargo.toml>) | svc | operator-server | runtime: mox-ai-agent-svc, mox-ai-expert-svc, mox-ai-flow-sdk, mox-api-protocol, mox-data-catalog-svc, mox-flow-fusion-svc, mox-flow-operator-core, mox-flow-operator-wasm-svc, mox-flow-optimizer-core, mox-flow-primiflow-svc, mox-kg-algo-core, mox-platform-foundation, mox-platform-meta-core, mox-platform-module-core, mox-platform-system-core |
| [mox-platform-paths](<../../platform/foundation/mox-platform-paths/Cargo.toml>) | foundation | — | — |
| [mox-platform-system-core](<../../platform/domains/platform/core/mox-platform-system-core/Cargo.toml>) | core | mox-platform-system-core | runtime: mox-platform-foundation |
| [mox-platform-test-harness](<../../platform/domains/platform/sdk/mox-platform-test-harness/Cargo.toml>) | sdk | — | runtime: mox-cloud-volume-svc, mox-data-compliance-svc, mox-data-etl-svc, mox-data-plane-svc, mox-kg-fusion-svc, mox-platform-gateway-svc |
| [mox-plugin-core](<../../platform/domains/platform/core/mox-plugin-core/Cargo.toml>) | core | — | — |
| [mox-plugin-sdk](<../../platform/domains/platform/sdk/mox-plugin-sdk/Cargo.toml>) | sdk | — | — |
| [mox-resilience-core](<../../platform/shared/mox-resilience-core/Cargo.toml>) | shared | — | — |
| [mox-server-runtime](<../../platform/shared/mox-server-runtime/Cargo.toml>) | shared | — | runtime: mox-auth-core, mox-cache-core, mox-config-core, mox-error, mox-observability-core, mox-resilience-core |
| [mox-unified-algo-core](<../../platform/shared/mox-unified-algo-core/Cargo.toml>) | shared | — | runtime: mox-platform-foundation |
| [mox-unified-contract](<../../platform/shared/mox-unified-contract/Cargo.toml>) | shared | — | — |

### project（2）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-project-graph-core](<../../platform/domains/project/core/mox-project-graph-core/Cargo.toml>) | core | — | runtime: mox-kg-core |
| [mox-project-graph-svc](<../../platform/domains/project/svc/mox-project-graph-svc/Cargo.toml>) | svc | mox-project-graph-svc | runtime: mox-kg-core, mox-project-graph-core |

### voice（8）

| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |
|---|---|---|---|
| [mox-voice-api](<../../platform/domains/voice/api/Cargo.toml>) | api | — | — |
| [mox-voice-asr-svc](<../../platform/domains/voice/svc/mox-voice-asr-svc/Cargo.toml>) | svc | — | runtime: mox-platform-foundation, mox-voice-core-svc |
| [mox-voice-core-svc](<../../platform/domains/voice/svc/mox-voice-core-svc/Cargo.toml>) | svc | — | runtime: mox-platform-foundation, mox-platform-system-core |
| [mox-voice-desktop-app](<../../platform/domains/voice/svc/mox-voice-desktop-app/Cargo.toml>) | svc | mox-voice-desktop-app | runtime: mox-platform-foundation, mox-voice-asr-svc, mox-voice-core-svc, mox-voice-intent-svc, mox-voice-operator-svc |
| [mox-voice-dsp-core](<../../platform/domains/voice/core/mox-voice-dsp-core/Cargo.toml>) | core | — | runtime: mox-platform-foundation |
| [mox-voice-dsp-py](<../../platform/domains/voice/sdk/mox-voice-dsp-py/Cargo.toml>) | sdk | — | runtime: mox-voice-dsp-core |
| [mox-voice-intent-svc](<../../platform/domains/voice/svc/mox-voice-intent-svc/Cargo.toml>) | svc | — | runtime: mox-platform-foundation, mox-voice-core-svc |
| [mox-voice-operator-svc](<../../platform/domains/voice/svc/mox-voice-operator-svc/Cargo.toml>) | svc | — | runtime: mox-platform-foundation, mox-voice-asr-svc, mox-voice-core-svc, mox-voice-intent-svc |

## 前端入口与功能文件

页面文件不等于已注册路由；实际挂载关系以路由源码为准。

### router（11）

- [frontend-ui/src/router/index.js](<../../frontend-ui/src/router/index.js>)
- [frontend-ui/src/router/modules/ai.js](<../../frontend-ui/src/router/modules/ai.js>)
- [frontend-ui/src/router/modules/alliance.js](<../../frontend-ui/src/router/modules/alliance.js>)
- [frontend-ui/src/router/modules/fallback.js](<../../frontend-ui/src/router/modules/fallback.js>)
- [frontend-ui/src/router/modules/graph.js](<../../frontend-ui/src/router/modules/graph.js>)
- [frontend-ui/src/router/modules/market.js](<../../frontend-ui/src/router/modules/market.js>)
- [frontend-ui/src/router/modules/operators.js](<../../frontend-ui/src/router/modules/operators.js>)
- [frontend-ui/src/router/modules/project.js](<../../frontend-ui/src/router/modules/project.js>)
- [frontend-ui/src/router/modules/public.js](<../../frontend-ui/src/router/modules/public.js>)
- [frontend-ui/src/router/modules/system.js](<../../frontend-ui/src/router/modules/system.js>)
- [frontend-ui/src/router/modules/workflow.js](<../../frontend-ui/src/router/modules/workflow.js>)

### views（71）

- [frontend-ui/src/views/admin/AdminView.vue](<../../frontend-ui/src/views/admin/AdminView.vue>)
- [frontend-ui/src/views/admin/panels/AdminAccess.vue](<../../frontend-ui/src/views/admin/panels/AdminAccess.vue>)
- [frontend-ui/src/views/admin/panels/AdminApi.vue](<../../frontend-ui/src/views/admin/panels/AdminApi.vue>)
- [frontend-ui/src/views/admin/panels/AdminAudit.vue](<../../frontend-ui/src/views/admin/panels/AdminAudit.vue>)
- [frontend-ui/src/views/admin/panels/AdminConfig.vue](<../../frontend-ui/src/views/admin/panels/AdminConfig.vue>)
- [frontend-ui/src/views/admin/panels/AdminDepartment.vue](<../../frontend-ui/src/views/admin/panels/AdminDepartment.vue>)
- [frontend-ui/src/views/admin/panels/AdminDict.vue](<../../frontend-ui/src/views/admin/panels/AdminDict.vue>)
- [frontend-ui/src/views/admin/panels/AdminDocs.vue](<../../frontend-ui/src/views/admin/panels/AdminDocs.vue>)
- [frontend-ui/src/views/admin/panels/AdminHitl.vue](<../../frontend-ui/src/views/admin/panels/AdminHitl.vue>)
- [frontend-ui/src/views/admin/panels/AdminLlm.vue](<../../frontend-ui/src/views/admin/panels/AdminLlm.vue>)
- [frontend-ui/src/views/admin/panels/AdminLogs.vue](<../../frontend-ui/src/views/admin/panels/AdminLogs.vue>)
- [frontend-ui/src/views/admin/panels/AdminMenu.vue](<../../frontend-ui/src/views/admin/panels/AdminMenu.vue>)
- [frontend-ui/src/views/admin/panels/AdminMonitor.vue](<../../frontend-ui/src/views/admin/panels/AdminMonitor.vue>)
- [frontend-ui/src/views/admin/panels/AdminOverview.vue](<../../frontend-ui/src/views/admin/panels/AdminOverview.vue>)
- [frontend-ui/src/views/admin/panels/AdminRole.vue](<../../frontend-ui/src/views/admin/panels/AdminRole.vue>)
- [frontend-ui/src/views/admin/panels/AdminStorage.vue](<../../frontend-ui/src/views/admin/panels/AdminStorage.vue>)
- [frontend-ui/src/views/admin/panels/AdminUser.vue](<../../frontend-ui/src/views/admin/panels/AdminUser.vue>)
- [frontend-ui/src/views/ai/AlgoLabView.vue](<../../frontend-ui/src/views/ai/AlgoLabView.vue>)
- [frontend-ui/src/views/ai/BotCenterView.vue](<../../frontend-ui/src/views/ai/BotCenterView.vue>)
- [frontend-ui/src/views/ai/CaomeiView.vue](<../../frontend-ui/src/views/ai/CaomeiView.vue>)
- [frontend-ui/src/views/ai/ChatView.vue](<../../frontend-ui/src/views/ai/ChatView.vue>)
- [frontend-ui/src/views/ai/InfiniteOptimizerView.vue](<../../frontend-ui/src/views/ai/InfiniteOptimizerView.vue>)
- [frontend-ui/src/views/ai/Melody2ScoreView.vue](<../../frontend-ui/src/views/ai/Melody2ScoreView.vue>)
- [frontend-ui/src/views/auth/ForgotPassword.vue](<../../frontend-ui/src/views/auth/ForgotPassword.vue>)
- [frontend-ui/src/views/auth/Login.vue](<../../frontend-ui/src/views/auth/Login.vue>)
- [frontend-ui/src/views/auth/Register.vue](<../../frontend-ui/src/views/auth/Register.vue>)
- [frontend-ui/src/views/expert/AllianceTaskView.vue](<../../frontend-ui/src/views/expert/AllianceTaskView.vue>)
- [frontend-ui/src/views/expert/ExpertCenterView.vue](<../../frontend-ui/src/views/expert/ExpertCenterView.vue>)
- [frontend-ui/src/views/expert/ExpertConfigView.vue](<../../frontend-ui/src/views/expert/ExpertConfigView.vue>)
- [frontend-ui/src/views/expert/ExpertPlazaView.vue](<../../frontend-ui/src/views/expert/ExpertPlazaView.vue>)
- [frontend-ui/src/views/expert/panels/ExpertEnterprisePanel.vue](<../../frontend-ui/src/views/expert/panels/ExpertEnterprisePanel.vue>)
- [frontend-ui/src/views/expert/panels/ExpertOrchestratorPanel.vue](<../../frontend-ui/src/views/expert/panels/ExpertOrchestratorPanel.vue>)
- [frontend-ui/src/views/expert/panels/ExpertOverviewPanel.vue](<../../frontend-ui/src/views/expert/panels/ExpertOverviewPanel.vue>)
- [frontend-ui/src/views/graph/FlowGraph.vue](<../../frontend-ui/src/views/graph/FlowGraph.vue>)
- [frontend-ui/src/views/graph/GraphView.vue](<../../frontend-ui/src/views/graph/GraphView.vue>)
- [frontend-ui/src/views/graph/MoxFusionView.vue](<../../frontend-ui/src/views/graph/MoxFusionView.vue>)
- [frontend-ui/src/views/market/MarketDetailView.vue](<../../frontend-ui/src/views/market/MarketDetailView.vue>)
- [frontend-ui/src/views/market/MarketView.vue](<../../frontend-ui/src/views/market/MarketView.vue>)
- [frontend-ui/src/views/misc/BusinessHall.vue](<../../frontend-ui/src/views/misc/BusinessHall.vue>)
- [frontend-ui/src/views/misc/Forbidden.vue](<../../frontend-ui/src/views/misc/Forbidden.vue>)
- [frontend-ui/src/views/misc/Login.vue](<../../frontend-ui/src/views/misc/Login.vue>)
- [frontend-ui/src/views/misc/PortalHome.vue](<../../frontend-ui/src/views/misc/PortalHome.vue>)
- [frontend-ui/src/views/operators/OperatorsView.vue](<../../frontend-ui/src/views/operators/OperatorsView.vue>)
- [frontend-ui/src/views/project/Dashboard.vue](<../../frontend-ui/src/views/project/Dashboard.vue>)
- [frontend-ui/src/views/project/panels/KnowledgeBasePanel.vue](<../../frontend-ui/src/views/project/panels/KnowledgeBasePanel.vue>)
- [frontend-ui/src/views/project/panels/ResourcesOverviewPanel.vue](<../../frontend-ui/src/views/project/panels/ResourcesOverviewPanel.vue>)
- [frontend-ui/src/views/project/ProjectsView.vue](<../../frontend-ui/src/views/project/ProjectsView.vue>)
- [frontend-ui/src/views/project/ResourcesView.vue](<../../frontend-ui/src/views/project/ResourcesView.vue>)
- [frontend-ui/src/views/project/TaskView.vue](<../../frontend-ui/src/views/project/TaskView.vue>)
- [frontend-ui/src/views/project/Workbench.vue](<../../frontend-ui/src/views/project/Workbench.vue>)
- [frontend-ui/src/views/workflow/BrowserView.vue](<../../frontend-ui/src/views/workflow/BrowserView.vue>)
- [frontend-ui/src/views/workflow/panels/AutomationPanel.vue](<../../frontend-ui/src/views/workflow/panels/AutomationPanel.vue>)
- [frontend-ui/src/views/workflow/panels/McpPanel.vue](<../../frontend-ui/src/views/workflow/panels/McpPanel.vue>)
- [frontend-ui/src/views/workflow/panels/PluginsPanel.vue](<../../frontend-ui/src/views/workflow/panels/PluginsPanel.vue>)
- [frontend-ui/src/views/workflow/panels/WorkflowFlowsPanel.vue](<../../frontend-ui/src/views/workflow/panels/WorkflowFlowsPanel.vue>)
- [frontend-ui/src/views/workflow/WorkflowView.vue](<../../frontend-ui/src/views/workflow/WorkflowView.vue>)
- [frontend-ui/src/views/workspace/ExpertWorkspaceView.vue](<../../frontend-ui/src/views/workspace/ExpertWorkspaceView.vue>)
- [frontend-ui/src/views/workspace/panels/AIAssistantPanel.vue](<../../frontend-ui/src/views/workspace/panels/AIAssistantPanel.vue>)
- [frontend-ui/src/views/workspace/panels/CollaborationPanel.vue](<../../frontend-ui/src/views/workspace/panels/CollaborationPanel.vue>)
- [frontend-ui/src/views/workspace/panels/DebateDialog.vue](<../../frontend-ui/src/views/workspace/panels/DebateDialog.vue>)
- [frontend-ui/src/views/workspace/panels/ExpertPanel.vue](<../../frontend-ui/src/views/workspace/panels/ExpertPanel.vue>)
- [frontend-ui/src/views/workspace/panels/FilePanel.vue](<../../frontend-ui/src/views/workspace/panels/FilePanel.vue>)
- [frontend-ui/src/views/workspace/panels/GraphCanvasPanel.vue](<../../frontend-ui/src/views/workspace/panels/GraphCanvasPanel.vue>)
- [frontend-ui/src/views/workspace/panels/HistoryPanel.vue](<../../frontend-ui/src/views/workspace/panels/HistoryPanel.vue>)
- [frontend-ui/src/views/workspace/panels/KnowledgeBasePanel.vue](<../../frontend-ui/src/views/workspace/panels/KnowledgeBasePanel.vue>)
- [frontend-ui/src/views/workspace/panels/KpiPanel.vue](<../../frontend-ui/src/views/workspace/panels/KpiPanel.vue>)
- [frontend-ui/src/views/workspace/panels/MultiConsultDialog.vue](<../../frontend-ui/src/views/workspace/panels/MultiConsultDialog.vue>)
- [frontend-ui/src/views/workspace/panels/SmartRouteDialog.vue](<../../frontend-ui/src/views/workspace/panels/SmartRouteDialog.vue>)
- [frontend-ui/src/views/workspace/panels/TaskOrchestrationPanel.vue](<../../frontend-ui/src/views/workspace/panels/TaskOrchestrationPanel.vue>)
- [frontend-ui/src/views/workspace/panels/WhiteboardPanel.vue](<../../frontend-ui/src/views/workspace/panels/WhiteboardPanel.vue>)
- [frontend-ui/src/views/workspace/panels/WorkspaceHeader.vue](<../../frontend-ui/src/views/workspace/panels/WorkspaceHeader.vue>)

### api（24）

- [frontend-ui/src/api/actuator.api.js](<../../frontend-ui/src/api/actuator.api.js>)
- [frontend-ui/src/api/ai.api.js](<../../frontend-ui/src/api/ai.api.js>)
- [frontend-ui/src/api/alliance.js](<../../frontend-ui/src/api/alliance.js>)
- [frontend-ui/src/api/allianceTaskModel.js](<../../frontend-ui/src/api/allianceTaskModel.js>)
- [frontend-ui/src/api/allianceTasks.test.js](<../../frontend-ui/src/api/allianceTasks.test.js>)
- [frontend-ui/src/api/auth.js](<../../frontend-ui/src/api/auth.js>)
- [frontend-ui/src/api/caomei.api.js](<../../frontend-ui/src/api/caomei.api.js>)
- [frontend-ui/src/api/experts.api.js](<../../frontend-ui/src/api/experts.api.js>)
- [frontend-ui/src/api/graph.api.js](<../../frontend-ui/src/api/graph.api.js>)
- [frontend-ui/src/api/http.js](<../../frontend-ui/src/api/http.js>)
- [frontend-ui/src/api/http.test.js](<../../frontend-ui/src/api/http.test.js>)
- [frontend-ui/src/api/index.js](<../../frontend-ui/src/api/index.js>)
- [frontend-ui/src/api/kb.api.js](<../../frontend-ui/src/api/kb.api.js>)
- [frontend-ui/src/api/llm.api.js](<../../frontend-ui/src/api/llm.api.js>)
- [frontend-ui/src/api/market.api.js](<../../frontend-ui/src/api/market.api.js>)
- [frontend-ui/src/api/melody.api.js](<../../frontend-ui/src/api/melody.api.js>)
- [frontend-ui/src/api/monitor.api.js](<../../frontend-ui/src/api/monitor.api.js>)
- [frontend-ui/src/api/mox.api.js](<../../frontend-ui/src/api/mox.api.js>)
- [frontend-ui/src/api/notification.api.js](<../../frontend-ui/src/api/notification.api.js>)
- [frontend-ui/src/api/operators.api.js](<../../frontend-ui/src/api/operators.api.js>)
- [frontend-ui/src/api/projects.api.js](<../../frontend-ui/src/api/projects.api.js>)
- [frontend-ui/src/api/system.api.js](<../../frontend-ui/src/api/system.api.js>)
- [frontend-ui/src/api/workflow.api.js](<../../frontend-ui/src/api/workflow.api.js>)
- [frontend-ui/src/api/workspace.api.js](<../../frontend-ui/src/api/workspace.api.js>)

### stores（10）

- [frontend-ui/src/stores/ai.store.js](<../../frontend-ui/src/stores/ai.store.js>)
- [frontend-ui/src/stores/alliance.store.js](<../../frontend-ui/src/stores/alliance.store.js>)
- [frontend-ui/src/stores/app.store.js](<../../frontend-ui/src/stores/app.store.js>)
- [frontend-ui/src/stores/auth.store.js](<../../frontend-ui/src/stores/auth.store.js>)
- [frontend-ui/src/stores/auth.store.test.js](<../../frontend-ui/src/stores/auth.store.test.js>)
- [frontend-ui/src/stores/index.js](<../../frontend-ui/src/stores/index.js>)
- [frontend-ui/src/stores/permission.store.js](<../../frontend-ui/src/stores/permission.store.js>)
- [frontend-ui/src/stores/project.store.js](<../../frontend-ui/src/stores/project.store.js>)
- [frontend-ui/src/stores/ui.store.js](<../../frontend-ui/src/stores/ui.store.js>)
- [frontend-ui/src/stores/user.store.js](<../../frontend-ui/src/stores/user.store.js>)

### composables（13）

- [frontend-ui/src/composables/projectContext.js](<../../frontend-ui/src/composables/projectContext.js>)
- [frontend-ui/src/composables/useAllianceTasks.js](<../../frontend-ui/src/composables/useAllianceTasks.js>)
- [frontend-ui/src/composables/useAllianceTasks.test.js](<../../frontend-ui/src/composables/useAllianceTasks.test.js>)
- [frontend-ui/src/composables/useKnowledgeBase.js](<../../frontend-ui/src/composables/useKnowledgeBase.js>)
- [frontend-ui/src/composables/useMessageActions.js](<../../frontend-ui/src/composables/useMessageActions.js>)
- [frontend-ui/src/composables/useSSE.js](<../../frontend-ui/src/composables/useSSE.js>)
- [frontend-ui/src/composables/useTheme.js](<../../frontend-ui/src/composables/useTheme.js>)
- [frontend-ui/src/composables/useTheme.test.js](<../../frontend-ui/src/composables/useTheme.test.js>)
- [frontend-ui/src/composables/workspace/useAlliance.js](<../../frontend-ui/src/composables/workspace/useAlliance.js>)
- [frontend-ui/src/composables/workspace/useGraphCanvas.js](<../../frontend-ui/src/composables/workspace/useGraphCanvas.js>)
- [frontend-ui/src/composables/workspace/useTaskOrchestration.js](<../../frontend-ui/src/composables/workspace/useTaskOrchestration.js>)
- [frontend-ui/src/composables/workspace/useWhiteboard.js](<../../frontend-ui/src/composables/workspace/useWhiteboard.js>)
- [frontend-ui/src/composables/workspace/useWorkspaceData.js](<../../frontend-ui/src/composables/workspace/useWorkspaceData.js>)

## projects 子目录

包含产品、示例及验收产物；目录存在不表示它是独立服务。

| 目录 | 顶层说明/构建入口 |
|---|---|
| [market-games](<../../projects/market-games>) | 无顶层入口标记，需人工确认归属 |
| [melody2score](<../../projects/melody2score>) | [README.md](<../../projects/melody2score/README.md>) |
| [mox-dualrpc](<../../projects/mox-dualrpc>) | [Cargo.toml](<../../projects/mox-dualrpc/Cargo.toml>) · [README.md](<../../projects/mox-dualrpc/README.md>) |
| [mox-official-site](<../../projects/mox-official-site>) | 无顶层入口标记，需人工确认归属 |
| [primiflow](<../../projects/primiflow>) | [README.md](<../../projects/primiflow/README.md>) |
| [t10-cloud-artifacts](<../../projects/t10-cloud-artifacts>) | [README.md](<../../projects/t10-cloud-artifacts/README.md>) |
| [t11-graph-artifacts](<../../projects/t11-graph-artifacts>) | [README.md](<../../projects/t11-graph-artifacts/README.md>) |
| [t17-ef-runs](<../../projects/t17-ef-runs>) | 无顶层入口标记，需人工确认归属 |
| [t17-sdk-examples](<../../projects/t17-sdk-examples>) | [README.md](<../../projects/t17-sdk-examples/README.md>) |
| [t19-regression](<../../projects/t19-regression>) | 无顶层入口标记，需人工确认归属 |
| [t19-regression-report](<../../projects/t19-regression-report>) | [README.md](<../../projects/t19-regression-report/README.md>) |
| [t20-canary-metrics](<../../projects/t20-canary-metrics>) | [README.md](<../../projects/t20-canary-metrics/README.md>) |
| [t22-simd-artifacts](<../../projects/t22-simd-artifacts>) | 无顶层入口标记，需人工确认归属 |
| [t24-gm-artifacts](<../../projects/t24-gm-artifacts>) | 无顶层入口标记，需人工确认归属 |
| [t25-glacier-artifacts](<../../projects/t25-glacier-artifacts>) | 无顶层入口标记，需人工确认归属 |
| [vendor-eval](<../../projects/vendor-eval>) | 无顶层入口标记，需人工确认归属 |
| [xiaobai_voice](<../../projects/xiaobai_voice>) | [pyproject.toml](<../../projects/xiaobai_voice/pyproject.toml>) · [README.md](<../../projects/xiaobai_voice/README.md>) |
