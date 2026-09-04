# 璇玑 RelGraph · 架构迁移映射表（旧15-crate → 新6层8域DDD矩阵）

> **文档身份**：旧架构（platform/domains/ 15 crate + gateway/runtime）到新架构（platform/domains/ 8域×core/svc/sdk + foundation/gateway/framework）的**唯一权威映射基准**。所有文档对齐、代码引用修正、路径更新必须以此表为准。
> **版本**：v1.0 ENT · 编制日期：2026-08-26
> **权威链**：L1 治理枢纽（28号mox 模块化系统架构分析报告）> 本迁移表（L2 执行基准）> 各下游文档
> **主责联盟**：开发联盟 R（架构·代码·迁移）

---

## 一、迁移总览

| 维度 | 旧架构 | 新架构 |
|------|--------|--------|
| 组织方式 | 扁平15 crate（platform/domains/） | 8域×5层矩阵（platform/domains/{域}/{层}/） |
| crate 数量 | 15（services）+ 1（gateway/runtime）+ 1（mox-common-meta）= 17 | 50+（domains内）+ 2（foundation）+ 1（gateway）+ 1（framework）= ~54 |
| 分层模型 | 无显式分层（crate混合领域逻辑与基础设施） | core（领域模型）/ svc（应用服务）/ sdk（对外类型）/ api（域间契约）/ svcapi（服务API） |
| 域划分 | 无（扁平） | 8域：ai / cloud / data / flow / kg / market / platform / voice |
| 横切层 | gateway/runtime（上帝crate，聚合16子服务） | foundation（2 crate）/ gateway（1 crate）/ framework（库） |
| 网关定位 | runtime = 聚合网关 + Cordis5插件内核 + RBAC + OpenAPI + 迁移引擎 + 治理 | mox-platform-gateway-svc = 纯路由+横切中间件（待瘦身） |
| 元数据 | mox-common-meta（独立crate，硬编码16行） | mox-platform-meta-core（platform域core层） |

---

## 二、旧→新 Crate 完整映射表（17→54）

### 2.1 核心业务 crate 映射

| # | 旧 crate（platform/domains/） | 旧 CRATE_ID | 新 crate（platform/domains/） | 新层级 | 迁移类型 | 说明 |
|---|-------------------------------|-------------|-------------------------------|--------|---------|------|
| 1 | operator-core | `acf14283-...` | flow/core/mox-flow-operator-core | core | ✅ 直接迁移 | 算子代数/守恒律/类型核心，从services移入flow域core层 |
| 2 | operator-wasm | `5a1df407-...` | flow/svc/mox-flow-operator-wasm-svc | svc | ✅ 直接迁移 | WASM算子沙箱执行/热加载插件 |
| 3 | graph-algorithms | `fbd31c6a-...` | kg/core/mox-kg-algo-core | core | ✅ 直接迁移 | 八大算法家族A1~A8（CNM/Brandes/Harmonic/PageRank/激活扩散/RRF/CEM/CPM） |
| 4 | kg-hub | `cb909f06-...` | kg/svc/mox-kg-hub-svc | svc | ✅ 直接迁移 | 知识图谱枢纽：混合索引+URN+摄入/推理/治理/影响/热点/闭环 |
| 5 | optimizer | `e56676c7-...` | flow/core/mox-flow-optimizer-core | core | ✅ 直接迁移 | CPM关键路径分析 + RCPSP资源约束调度 + CEM交叉熵优化 |
| 6 | flow-ai | `2fcd3eac-...` | ai/svc/mox-ai-flow-svc | svc | ✅ 直接迁移 | 流程AI：9模块（冒险/CPM/冲突/调度/拓扑/代码gen/流水线/原语/可视化） |
| 7 | ai-agent | `00374bdd-...` | ai/svc/mox-ai-agent-svc | svc | ✅ 直接迁移 | AI智能体：对话/浏览器自动化/BPMN/MultiAgent/ProviderRegistry + A7 CEM |
| 8 | mox-expert | `50bb6200-...` | ai/svc/mox-ai-expert-svc | svc | ✅ 直接迁移 | ⛨璇玑引擎：双璇玑十四维治理/归一化IR/裁决/验证/审计三汇/RBAC |
| 9 | mox-system | `b81eec75-...` | platform/core/mox-platform-system-core + platform/svc/mox-platform-enterprise-svc | core + svc | 🔄 拆分迁移 | 璇玑协作治理域：成员/任务/权限/通信/审计/RBAC/多后端。拆分为core（领域模型）+ svc（应用服务） |
| 10 | mox-common-meta | `34a20231-...` | platform/core/mox-platform-meta-core | core | ✅ 直接迁移 | 纯数据元crate：AisLayer枚举/CrateMeta结构体/all_crate_metas()。**需更新为新50+ crate列表** |
| 11 | primiflow-core | `8c8d2382-...` | flow/svc/mox-flow-primiflow-svc | svc | ✅ 直接迁移 | PrimiFlow解析/代码生成/8类骨架模板/执行/持久化 |
| 12 | primiflow-fusion | `75238345-...` | flow/svc/mox-flow-fusion-svc | svc | ✅ 直接迁移 | PrimiFlow六维融合/守恒闸门/Registry/平台编排/12Factor+可观测 |
| 13 | business-catalog | `62b2cca1-...` | data/svc/mox-data-catalog-svc | svc | ✅ 直接迁移 | 6预置FlowGraph + TopologyGraph（政务/财务/客服/ETL/MCP/螺旋） |
| 14 | template-market | `4d2e50c1-...` | market/svc/mox-market-template-svc | svc | ✅ 直接迁移 | 模板市场：发布/加载/评分/排序/Fork/2种子 |
| 15 | hermes-flow-bridge | `9bfaf43b-...` | flow/svc/mox-flow-bridge-svc | svc | ✅ 直接迁移 | Hermes Agent桥接：normalize/recorder/router/拦截注入 |

### 2.2 网关/横切 crate 映射

| # | 旧 crate | 新 crate | 迁移类型 | 说明 |
|---|---------|---------|---------|------|
| 16 | gateway/runtime（runtime） | gateway/mox-platform-gateway-svc | 🔄 重写+瘦身 | 旧runtime是上帝crate（聚合16子服务+Cordis5+RBAC+OpenAPI+迁移+治理）。新网关应仅做路由+横切中间件，业务聚合下沉到各域svc层 |
| 17 | （无对应） | foundation/mox-platform-foundation | 🆕 新增 | 平台基础库：通用类型、错误处理、配置、工具函数 |
| 18 | （无对应） | foundation/mox-cloud-foundation | 🆕 新增 | 云基础设施基础库：云存储抽象、卷管理、S3适配 |
| 19 | （无对应） | framework/（mox-framework） | 🆕 新增 | 框架层：可能是插件框架/扩展点定义 |

### 2.3 新架构中新增的 crate（旧架构无对应）

| 域 | 层 | 新 crate | 职责 |
|----|----|---------|------|
| ai | core | mox-ai-intent-core | AI意图识别领域模型（A5激活扩散意图路由核心） |
| ai | core | mox-ai-core | AI领域核心（统一AI内核） |
| cloud | svc | mox-cloud-master-svc | 云主节点服务 |
| cloud | svc | mox-cloud-volume-svc | 云卷管理服务 |
| cloud | svc | mox-cloud-s3-svc | S3兼容存储服务 |
| cloud | svc | mox-cloud-filer-svc | 文件器服务 |
| cloud | sdk | mox-cloud-sdk | 云服务SDK |
| data | core | mox-data-formula-core | 公式引擎核心（高精度计算） |
| data | core | mox-data-norm-core | 数据归一化核心 |
| data | core | mox-data-standards-core | 数据标准核心 |
| data | svc | mox-data-plane-svc | 数据平面服务 |
| data | svc | mox-data-etl-svc | ETL服务 |
| data | svc | mox-data-compliance-svc | 数据合规服务 |
| data | sdk | mox-data-formula-native | 公式引擎原生绑定（FFI） |
| data | sdk | mox-data-norm-intent-native | 归一化意图原生绑定（FFI） |
| kg | core | mox-kg-meta-core | 图谱元数据核心 |
| kg | svc | mox-kg-storage-svc | 图谱存储服务 |
| kg | svc | mox-kg-service-svc | 图谱服务 |
| kg | svc | mox-kg-streams-svc | 图谱流处理服务 |
| kg | svc | mox-kg-spark-svc | 图谱Spark集成服务 |
| kg | svc | mox-kg-fusion-svc | 图谱融合服务 |
| kg | sdk | mox-kg-sdk | 图谱SDK |
| platform | core | mox-platform-iam-core | IAM身份与访问管理核心 |
| platform | core | mox-platform-datastore-core | 数据存储核心（多后端抽象） |
| platform | core | mox-platform-orchestrator-core | 编排器核心 |
| platform | svc | mox-platform-orchestrator-svc | 编排器服务 |
| platform | sdk | mox-platform-test-harness | 测试框架SDK |
| voice | core | mox-voice-dsp-core | 语音DSP核心 |
| voice | svc | mox-voice-core-svc | 语音核心服务 |
| voice | svc | mox-voice-asr-svc | 语音ASR服务 |
| voice | svc | mox-voice-intent-svc | 语音意图服务 |
| voice | svc | mox-voice-operator-svc | 语音算子服务 |
| voice | svc | mox-voice-desktop-app | 语音桌面应用（**独立产品形态**） |
| voice | sdk | mox-voice-dsp-py | 语音DSP Python绑定（PyO3） |

---

## 三、路径替换速查表（文档修正用）

> 用法：在任何文档中搜索旧路径，替换为新路径。**全局替换前必须确认上下文，避免误替换。**

| 旧路径字符串 | 新路径字符串 | 出现频率 |
|-------------|-------------|---------|
| `platform/domains/operator-core` | `platform/domains/flow/core/mox-flow-operator-core` | 高 |
| `platform/domains/operator-wasm` | `platform/domains/flow/svc/mox-flow-operator-wasm-svc` | 中 |
| `platform/domains/graph-algorithms` | `platform/domains/kg/core/mox-kg-algo-core` | 高 |
| `platform/domains/kg-hub` | `platform/domains/kg/svc/mox-kg-hub-svc` | 高 |
| `platform/domains/optimizer` | `platform/domains/flow/core/mox-flow-optimizer-core` | 中 |
| `platform/domains/flow-ai` | `platform/domains/ai/svc/mox-ai-flow-svc` | 中 |
| `platform/domains/ai-agent` | `platform/domains/ai/svc/mox-ai-agent-svc` | 高 |
| `platform/domains/mox-expert` | `platform/domains/ai/svc/mox-ai-expert-svc` | 高 |
| `platform/domains/mox-system` | `platform/domains/platform/core/mox-platform-system-core`（核心）/ `platform/domains/platform/svc/mox-platform-enterprise-svc`（服务） | 高 |
| `platform/domains/mox-common-meta` | `platform/domains/platform/core/mox-platform-meta-core` | 中 |
| `platform/domains/primiflow-core` | `platform/domains/flow/svc/mox-flow-primiflow-svc` | 中 |
| `platform/domains/primiflow-fusion` | `platform/domains/flow/svc/mox-flow-fusion-svc` | 中 |
| `platform/domains/business-catalog` | `platform/domains/data/svc/mox-data-catalog-svc` | 低 |
| `platform/domains/template-market` | `platform/domains/market/svc/mox-market-template-svc` | 低 |
| `platform/domains/hermes-flow-bridge` | `platform/domains/flow/svc/mox-flow-bridge-svc` | 低 |
| `platform/gateway/runtime` | `platform/gateway/mox-platform-gateway-svc` | 高 |
| `crates/`（旧别名） | `platform/domains/`（新规范） | 中 |

---

## 四、crate 名称替换速查表

| 旧 package.name | 新 package.name | 说明 |
|-----------------|-----------------|------|
| `operator-core` | `mox-flow-operator-core` | 算子内核 |
| `operator-wasm` | `mox-flow-operator-wasm-svc` | WASM沙箱 |
| `graph-algorithms` | `mox-kg-algo-core` | 图算法 |
| `kg-hub` | `mox-kg-hub-svc` | 图谱枢纽 |
| `optimizer` | `mox-flow-optimizer-core` | 优化器 |
| `flow-ai` | `mox-ai-flow-svc` | 流程AI |
| `ai-agent` | `mox-ai-agent-svc` | AI智能体 |
| `mox-expert` | `mox-ai-expert-svc` | 璇玑专家 |
| `mox-system` | `mox-platform-system-core` + `mox-platform-enterprise-svc` | 璇玑系统（拆分） |
| `mox-common-meta` | `mox-platform-meta-core` | 元数据 |
| `primiflow-core` | `mox-flow-primiflow-svc` | PrimiFlow核心 |
| `primiflow-fusion` | `mox-flow-fusion-svc` | PrimiFlow融合 |
| `business-catalog` | `mox-data-catalog-svc` | 业务目录 |
| `template-market` | `mox-market-template-svc` | 模板市场 |
| `hermes-flow-bridge` | `mox-flow-bridge-svc` | Hermes桥接 |
| `runtime` | `mox-platform-gateway-svc` | 网关 |

---

## 五、ENGINE_NAME 映射（代码内常量）

| 旧 ENGINE_NAME | 新 ENGINE_NAME（推断） | 所在 crate |
|---------------|----------------------|-----------|
| `mox::operator_core` | `mox::flow::operator_core` | mox-flow-operator-core |
| `mox::graph_algorithms` | `mox::kg::algo_core` | mox-kg-algo-core |
| `mox::kg_hub` | `mox::kg::hub_svc` | mox-kg-hub-svc |
| `mox::optimizer` | `mox::flow::optimizer_core` | mox-flow-optimizer-core |
| `mox::flow_ai` | `mox::ai::flow_svc` | mox-ai-flow-svc |
| `mox::ai_agent` | `mox::ai::agent_svc` | mox-ai-agent-svc |
| `mox::mox_expert` | `mox::ai::expert_svc` | mox-ai-expert-svc |
| `mox::mox_system` | `mox::platform::system_core` | mox-platform-system-core |
| `mox::runtime` | `mox::platform::gateway_svc` | mox-platform-gateway-svc |
| `mox::mox_common_meta` | `mox::platform::meta_core` | mox-platform-meta-core |

> ⚠️ **注意**：ENGINE_NAME 是各 crate `src/lib.rs` 中 `pub const ENGINE_NAME` 的实际值。上表为基于命名规范的推断，**必须以各 crate 实际 lib.rs 中的常量为准**。迁移后需运行 `mox-platform-meta-core::all_crate_metas()` 验证一致性。

---

## 六、AIS Layer 映射

| 旧 AIS Layer | 新 AIS Layer（推断） | 说明 |
|--------------|---------------------|------|
| L6Kernel（operator-core） | L2Core（mox-flow-operator-core） | core层统一为L2Core |
| L4Services（14个svc crate） | L3Svc（各域svc层crate） | svc层统一为L3Svc |
| L3Orchestration（runtime） | L1Gateway（mox-platform-gateway-svc） | 网关层为L1Gateway |
| L5Domain（mox-common-meta） | L2Core（mox-platform-meta-core） | 元数据归入core层 |
| L7Infrastructure（mox-system） | L2Core + L3Svc（拆分后） | 系统核心为core，企业服务为svc |
| （无） | L4Sdk（各域sdk层） | 新增sdk层级 |
| （无） | L5Api（各域api层，待填充） | 新增api层级（域间契约） |
| （无） | L0Foundation（foundation层） | 新增基础层 |

---

## 七、迁移验证检查清单

- [ ] 所有 `platform/domains/` 路径引用已替换为 `platform/domains/` 对应路径
- [ ] 所有旧 package.name（operator-core, graph-algorithms, etc.）已替换
- [ ] `mox-platform-meta-core::all_crate_metas()` 已更新为新50+ crate列表
- [ ] 各 crate `src/lib.rs` 中 `CRATE_ID` / `ENGINE_NAME` 常量已验证
- [ ] `Cargo.toml` workspace members 与实际目录一致（已确认73 member）
- [ ] `platform/domains/` 目录已确认不存在（0子目录，已验证）
- [ ] 文档中"15 Crate" / "16 Crate" 表述已更新为"50+ Crate / 8域矩阵"
- [ ] 旧→新映射表（本文档）已被02架构文档、README、CLAUDE、22号总控卡引用
- [ ] `tools/info-graph dedup` 判重工具中的路径配置已更新
- [ ] CI/CD 脚本中的路径引用已更新

---

## 八、变更记录

| 版本 | 日期 | 变更内容 | 签署 |
|------|------|---------|------|
| v1.0 | 2026-08-26 | 首发：17旧crate→54新crate完整映射 + 路径替换速查表 + crate名称映射 + ENGINE_NAME映射 + AIS Layer映射 + 迁移验证检查清单（10项） | 开发联盟R____ |
