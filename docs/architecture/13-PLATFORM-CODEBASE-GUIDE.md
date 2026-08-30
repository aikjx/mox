# MOX 平台代码库结构指南

> 版本：v1.0 | 日期：2026-08-29 | 状态：生效
>
> 本文档是 MOX 平台代码库的**权威结构说明**，所有开发者必须遵守。

## 1. 顶层目录结构

```
infotopograph/
├── platform/              # 核心后端（Rust 模块化架构）
│   ├── foundation/        # 基础层：通用工具、错误、路径、可观测性
│   ├── domains/           # 领域层：各业务域的 api/core/svc/sdk
│   ├── gateway/           # 网关层：API 网关服务
│   ├── framework/         # 框架层：Web 框架抽象
│   ├── legacy/            # 历史遗留代码（只读，不再维护）
│   └── arch-test/         # 架构测试工具
├── frontend-ui/           # 前端工程（Vue + Vite）
├── docs/                  # 统一文档中心
│   ├── architecture/      # 架构设计文档
│   ├── enterprise/        # 企业级需求与交付文档
│   ├── expert-alliance/   # 专家联盟相关文档
│   ├── specifications/    # 规格与标准（含任务规格）
│   ├── standards/         # 架构规范与标准
│   ├── working-reports/   # 过程分析报告
│   └── _archive/          # 历史归档
├── deploy/                # 部署配置
│   ├── docker/            # Docker 相关
│   ├── helm/              # Kubernetes Helm Charts
│   ├── config/            # 应用配置文件
│   ├── docs/              # 部署运维文档
│   └── sql/               # SQL 脚本
├── prototypes/            # HTML 原型/演示项目（非生产代码）
├── .github/               # CI/CD 工作流
├── ais/                   # 第三方 AI 工具参考仓库（Git 子模块/本地克隆）
├── tests/                 # 端到端测试与夹具
├── tools/                 # 运维与开发工具脚本
├── data/                  # 运行时数据（不入库）
├── log/                   # 运行时日志（不入库）
├── Cargo.toml             # Rust 工作空间根配置
├── docker-compose.yml     # 一体化部署编排
├── .gitignore             # Git 忽略规则
└── README.md              # 项目入口说明
```

## 2. 核心源码架构（platform/）

### 2.1 分层原则

MOX 后端采用 **六层架构**，从上到下依赖方向严格单向：

```
┌─────────────────────────────────┐
│          Application            │  编排层：orchestrator-svc
├─────────────────────────────────┤
│             Gateway             │  网关层：gateway-svc
├─────────────────────────────────┤
│           API Layer             │  API 层：各域 api crate
├─────────────────────────────────┤
│          Service Layer          │  服务层：各域 svc crate
├─────────────────────────────────┤
│           Core Layer            │  核心层：各域 core crate
├─────────────────────────────────┤
│         Foundation Layer        │  基础层：foundation crate
└─────────────────────────────────┘
```

### 2.2 业务域划分

| 域 | 路径前缀 | 职责 |
|----|----------|------|
| AI | `platform/domains/ai/` | AI 引擎、专家联盟、Agent、意图识别 |
| Data | `platform/domains/data/` | 数据平面、ETL、合规、目录 |
| KG | `platform/domains/kg/` | 知识图谱存储、算法、服务、融合 |
| Cloud | `platform/domains/cloud/` | 云盘、对象存储、卷管理 |
| Voice | `platform/domains/voice/` | 语音 DSP、ASR、意图、操作 |
| Flow | `platform/domains/flow/` | 流程引擎、算子、融合 |
| Market | `platform/domains/market/` | 应用市场、模板 |
| Platform | `platform/domains/platform/` | 系统、IAM、编排、企业、插件 |
| Project | `platform/domains/project/` | 项目图谱 |

### 2.3 每层 Crate 命名规范

每个域遵循统一的 crate 命名和目录结构：

```
domains/<domain>/
├── api/                     # API 层：<domain>-api
├── core/                    # 核心层
│   ├── mox-<domain>-xxx-core/
│   └── ...
├── svc/                     # 服务层
│   ├── mox-<domain>-xxx-svc/
│   └── ...
└── sdk/                     # SDK 层（可选）
    ├── mox-<domain>-xxx-sdk/
    └── ...
```

## 3. 文档体系

### 3.1 文档分类

| 目录 | 用途 | 维护者 |
|------|------|--------|
| `docs/architecture/` | 架构设计、ADR、技术规范 | 架构组 |
| `docs/enterprise/` | 企业级需求、交付、验收报告 | 产品+架构 |
| `docs/expert-alliance/` | 专家联盟设计与实现 | AI 专家联盟组 |
| `docs/specifications/` | 接口规格、数据规范、任务规格 | 各域负责人 |
| `docs/standards/` | 编码标准、流程标准 | 架构组 |
| `docs/working-reports/` | 过程分析报告、技术调研 | 全体开发者 |
| `deploy/docs/` | 部署运维手册 | 运维组 |

### 3.2 文档准入规则

1. **过程文档**（`.trae/` 中产生的）需在任务完成后 **3 天内**：
   - 有长期价值的 → 归档到 `docs/working-reports/`
   - 临时性质的 → 删除
2. **正式文档**必须放入正确的分类目录
3. **新增文档**必须在对应目录的 `README.md` 中登记
4. 禁止在根目录或非文档目录放置 Markdown 文档

## 4. 新增目录审批流程

任何新增的**顶层目录**必须经过以下流程：

```
提出申请 → 架构组评审 → 批准/驳回 → 执行 → 更新本文档
```

申请需包含：
- 目录名称与用途
- 预期生命周期（临时/永久）
- 与现有目录的关系
- Git 跟踪需求（入库/忽略）

## 5. 命名规范

### 5.1 目录命名

- 使用 **小写 + 连字符**（kebab-case）：`frontend-ui`, `mox-server`
- 缩写词保持小写：`ais/`, `kg/`
- 隐藏目录以 `.` 开头：`.github/`, `.trae/`（仅限工具配置）

### 5.2 文件命名

- Rust 源文件：`snake_case`
- Markdown 文档：`kebab-case`，中文文档可用中文标题
- 配置文件：`kebab-case` 或按工具约定

## 6. 清理与维护

### 6.1 月度清理日

每月第一个周五为"代码库清理日"，各负责人检查：

- [ ] 运行时文件是否意外入库
- [ ] 临时文件是否清理
- [ ] 过程文档是否已归档
- [ ] 废弃代码是否标记 DEPRECATED
- [ ] 目录结构指南是否更新

### 6.2 废弃代码处理

1. 确认代码不再使用 → 移入 `platform/legacy/`
2. 添加 `DEPRECATED.md` 说明废弃原因和替代方案
3. 标记保留期限（通常 3 个月）
4. 到期后彻底删除

## 7. 相关文档

- [架构规范](../standards/ai-native-architecture-standard.md)
- [归一化架构](architecture/NORMALIZED_ARCHITECTURE.md)
- [企业级文档索引](../enterprise/00-INDEX.md)
