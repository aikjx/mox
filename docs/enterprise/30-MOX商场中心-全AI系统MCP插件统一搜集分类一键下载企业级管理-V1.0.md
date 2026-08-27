# MOX 商场中心：全 AI 系统 / MCP / 插件统一搜集·分类·一键下载·企业级管理

> **版本**: v1.0  
> **日期**: 2026-08-27  
> **状态**: 架构设计  
> **归属**: 开发专家联盟 · 全维归一化  
> **关联**: [29-MOX总任务中心架构](29-MOX总任务中心-全AI工具全软件系统统一控制架构-V1.0.md) · [架构-数据分离规范](../standards/architecture-data-separation.md)

---

## 一、问题定义：5万+ MCP 服务器如何管理？

### 1.1 MCP 生态现状（2026年8月）

| 注册中心 | 规模 | 特点 | 来源 |
|---|---|---|---|
| **Official MCP Registry** | Canonical | 命名空间验证，权威元数据，API 可编程发现 | ["https://registry.modelcontextprotocol.io/"] |
| **Smithery.ai** | ~7,000+ | "Docker Hub of MCP"，CLI 一键安装，92% 成功率 | ["https://explainx.ai/blog/top-10-mcp-server-directories-2026","https://codex.danielvaughan.com/2026/05/27/codex-cli-mcp-server-discovery-registries-smithery-glama-official-registry/"] |
| **Glama** | ~20,000-50,000+ | 安全扫描，元注册表（聚合 Anthropic/GitHub/PulseMCP/微软） | ["https://openhelm.ai/blog/mcp-registry-directory-guide","https://www.truefoundry.com/blog/best-mcp-registries"] |
| **PulseMCP** | ~11,000-20,000+ | 人工审核，质量过滤 | ["https://www.digitalapi.ai/blogs/mcp-registry"] |
| **mcp.so** | ~18,998 | 社区驱动，长尾搜索 | ["https://www.awesomeagents.ai/leaderboards/mcp-server-ecosystem-leaderboard/"] |
| **LobeHub MCP** | 56,000+ | 一键部署，社区市场 | ["https://explainx.ai/blog/top-10-mcp-server-directories-2026"] |
| **Awesome MCP Servers** (GitHub) | 分类索引 | AI集成1632/浏览器281/云373/代码118... | ["https://github.com/TensorBlock/awesome-mcp-servers"] |

**核心痛点**：
1. **碎片化**：10+ 注册中心，格式不一，质量参差不齐
2. **安全风险**：2026年6月 JetBrains 市场 15 个第三方插件窃取 AI API Key 事件 ["https://mingooland.com/2026/06/jetbrains-marketplace-ecosystem-security-update-addressing-malicious-third-party-ai-plugins/"]
3. **管理混乱**：安装/更新/卸载/版本回滚无统一工具
4. **分类缺失**：5万+ 服务器无统一分类体系

### 1.2 MOX 商场中心的定位

**MOX Marketplace Center (MMC)** 是 MOX 总任务中心的**资源供给层**，负责：
- **聚合**：从 10+ 注册中心统一搜集 MCP 服务器、AI 系统、插件
- **分类**：官方/第三方/开源三级分类 + 功能域分类
- **下载**：一键下载到本地（`plugins/` 目录，架构-数据分离）
- **管理**：版本/更新/卸载/安全/权限全生命周期管理

---

## 二、三级分类体系：官方 / 第三方 / 开源

### 2.1 分类定义与处理模式

| 级别 | 定义 | 信任等级 | 处理模式 | 安全要求 | 示例 |
|---|---|---|---|---|---|
| **🟢 官方 (Official)** | MOX 团队开发维护，或经过 MOX 严格审核认证 | 最高 | 自动安装，自动更新，全功能可用 | 签名验证 + 沙箱 | MOX 官方 MCP Server（WPS/PS/飞书适配） |
| **🟡 第三方认证 (Certified Third-Party)** | 第三方开发者提交，经过 MOX 安全审核 + 功能测试 | 中高 | 一键安装，手动确认更新，受限功能 | 签名验证 + 沙箱 + 权限审核 | Smithery 精选 / Glama 高分服务器 |
| **🔴 社区开源 (Community Open-Source)** | 来自公开注册中心，未经 MOX 审核 | 最低 | 需手动确认安装，沙箱强制隔离，功能受限 | 强制沙箱 + 网络隔离 + 行为审计 | GitHub Awesome MCP / mcp.so 长尾 |

### 2.2 不同处理模式详解

#### 🟢 官方资源处理模式

```
安装流程：搜索 → 一键安装 → 自动签名验证 → 自动配置 → 立即可用
更新策略：自动更新（可配置为手动）
权限范围：全功能（文件系统/网络/进程等按需申请）
沙箱级别：标准沙箱（资源限制，非强制隔离）
回滚能力：支持版本回滚（保留最近3个版本）
审计要求：标准操作日志
```

#### 🟡 第三方认证资源处理模式

```
安装流程：搜索 → 查看安全报告 → 一键安装 → 签名验证 → 权限确认 → 可用
更新策略：通知更新，手动确认（diff 预览）
权限范围：受限功能（需逐项申请，默认最小权限）
沙箱级别：增强沙箱（强制隔离 + 系统调用过滤）
回滚能力：支持版本回滚
审计要求：详细操作日志 + 异常行为告警
```

#### 🔴 社区开源资源处理模式

```
安装流程：搜索 → 查看源码/评分 → 手动确认安装（二次确认）→ 强制沙箱 → 可用
更新策略：仅通知，不自动更新（需用户主动操作）
权限范围：最小权限（默认仅允许 stdout，其他需显式授权）
沙箱级别：最高安全沙箱（网络隔离 + 文件系统只读 + 进程隔离 + 资源硬限制）
回滚能力：支持卸载，不支持版本回滚
审计要求：全链路行为审计 + 异常自动隔离
```

### 2.3 安全升级机制

```
社区开源(🔴) ──(提交审核 + 安全扫描 + 功能测试)──> 第三方认证(🟡)
第三方认证(🟡) ──(长期稳定 + 高评分 + MOX团队接管)──> 官方(🟢)
官方(🟢) ──(发现严重漏洞/恶意行为)──> 立即下架 + 黑名单
```

---

## 三、功能域分类体系（12 大类）

在三级分类基础上，按功能域进行二级分类：

| 功能域 | 说明 | 典型 MCP 服务器 |
|---|---|---|
| **🤖 AI & LLM 集成** | AI 模型调用、推理、微调 | OpenAI/Claude/Gemini API、本地 Ollama |
| **🌐 浏览器自动化** | 网页操作、爬虫、截图 | Playwright/Puppeteer、浏览器控制 |
| **💻 开发工具** | 代码分析、Git、CI/CD | GitHub/GitLab、代码审查、Docker |
| **☁️ 云平台** | 云服务管理、部署 | AWS/Azure/GCP、K8s、Serverless |
| **📊 数据与数据库** | 数据库操作、ETL、分析 | PostgreSQL/MySQL、SQLite、数据管道 |
| **📝 办公套件** | 文档/表格/演示操作 | WPS/Office、Notion、Confluence |
| **🎨 图像设计** | 图像处理、设计工具 | Photoshop、Figma、图像生成 |
| **📁 文件系统** | 文件操作、搜索、同步 | 文件管理、云盘、FTP/SFTP |
| **📧 通信协作** | 邮件、即时消息、日历 | 飞书/钉钉/企微、邮件、日历 |
| **🔒 安全运维** | 安全扫描、监控、日志 | 漏洞扫描、Prometheus、ELK |
| **🎯 业务系统** | CRM/ERP/财务等 | Salesforce、SAP、财务系统 |
| **🧩 其他** | 物联网、硬件、实验性 | 智能家居、机器人、科研工具 |

---

## 四、MOX 商场中心架构设计

### 4.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                        用户界面层 (UI)                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │ 商场浏览页    │  │ 已安装管理页  │  │ 安全中心(审计/权限/沙箱) │  │
│  └──────┬───────┘  └──────┬───────┘  └────────────┬─────────────┘  │
└─────────┼───────────────────┼────────────────────────┼────────────────┘
          │                   │                        │
┌─────────▼───────────────────▼────────────────────────▼────────────────┐
│                     MOX 商场中心 API 层 (L4 Svc)                       │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  mox-market-center-svc                                             │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────────┐ │  │
│  │  │ 搜索聚合  │ │ 安装管理  │ │ 更新管理  │ │ 安全与权限治理     │ │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└─────────┬───────────────────┬────────────────────────┬────────────────┘
          │                   │                        │
┌─────────▼─────────┐ ┌───────▼────────┐ ┌───────────▼──────────────┐
│  聚合索引层 (L3)   │ │  本地仓库层     │ │  安全沙箱层               │
│  mox-market-core  │ │  plugins/       │ │  mox-flow-operator-wasm  │
│                   │ │  ├── wasm/      │ │  (WASM 沙箱执行)          │
│  • 10+ 注册中心聚合│ │  ├── scripts/   │ │                           │
│  • 统一元数据模型  │ │  └── extensions/│ │  • 资源限制(CPU/内存/磁盘)│
│  • 评分/安全扫描   │ │  └── index.json │ │  • 系统调用过滤            │
│  • 分类索引        │ │                 │ │  • 网络隔离                │
│  • 增量同步        │ │  本地元数据DB    │ │  • 文件系统隔离            │
└────────────────────┘ └────────────────┘ └───────────────────────────┘
          │
┌─────────▼─────────────────────────────────────────────────────────────┐
│                     外部注册中心聚合层                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │ Official │ │ Smithery │ │  Glama   │ │ PulseMCP │ │  mcp.so  │  │
│  │ Registry │ │   .ai    │ │          │ │          │ │          │  │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │ LobeHub  │ │ Awesome  │ │  GitHub  │ │  npm/PyPI│                │
│  │   MCP    │ │  MCP     │ │  Topics  │ │  MCP包   │                │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘                │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.2 核心模块设计

#### 模块 1：聚合索引引擎（`mox-market-core`，L3）

```rust
// mox-market-api/src/market.rs (L2 API 契约)

/// 统一资源元数据模型（聚合 10+ 注册中心）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResource {
    pub resource_id: Uuid,           // MOX 内部唯一 ID
    pub name: String,                 // 资源名称
    pub description: String,          // 功能描述
    pub category: ResourceCategory,   // 功能域分类（12大类）
    pub trust_level: TrustLevel,      // 信任等级（Official/Certified/Community）
    pub source: RegistrySource,       // 来源注册中心
    pub source_id: String,            // 来源注册中心的原始 ID
    pub version: String,              // 当前版本（SemVer）
    pub versions: Vec<VersionInfo>,   // 历史版本列表
    pub author: String,               // 作者/组织
    pub license: String,              // 开源协议
    pub tools: Vec<ToolDescriptor>,   // 暴露的 MCP 工具列表
    pub permissions: Vec<Permission>, // 需要的权限
    pub install_count: u64,           // 安装量（聚合统计）
    pub rating: f64,                  // 评分（0-5，聚合多源）
    pub security_score: f64,          // 安全评分（0-100，扫描结果）
    pub last_updated: DateTime<Utc>,  // 最后更新时间
    pub homepage: Option<String>,      // 主页链接
    pub repository: Option<String>,    // 源码仓库
    pub signatures: Vec<Signature>,    // 签名信息
}

/// 信任等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    Official,       // 🟢 官方
    Certified,      // 🟡 第三方认证
    Community,      // 🔴 社区开源
    Blacklisted,    // ⚫ 黑名单（恶意/漏洞）
}

/// 注册中心来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegistrySource {
    OfficialMCP,
    Smithery,
    Glama,
    PulseMCP,
    McpSo,
    LobeHub,
    AwesomeMCP,
    GitHub,
    Npm,
    PyPI,
    MoxOfficial,    // MOX 官方市场
}

/// 市场聚合服务 trait
#[async_trait]
pub trait MarketAggregator: Send + Sync {
    /// 从所有注册中心增量同步资源元数据
    async fn sync_all(&self) -> Result<SyncReport>;

    /// 从指定注册中心同步
    async fn sync_from(&self, source: RegistrySource) -> Result<SyncReport>;

    /// 搜索资源（支持关键词/分类/信任等级/评分过滤）
    async fn search(&self, query: SearchQuery) -> Result<Vec<MarketResource>>;

    /// 获取资源详情
    async fn get_resource(&self, resource_id: Uuid) -> Result<MarketResource>;

    /// 获取资源的安全报告
    async fn get_security_report(&self, resource_id: Uuid) -> Result<SecurityReport>;
}
```

#### 模块 2：安装管理器（`mox-market-center-svc`，L4）

```rust
/// 安装管理器 trait
#[async_trait]
pub trait InstallManager: Send + Sync {
    /// 一键安装资源
    async fn install(&self, resource_id: Uuid, options: InstallOptions) -> Result<InstalledResource>;

    /// 卸载资源
    async fn uninstall(&self, install_id: Uuid) -> Result<()>;

    /// 更新资源到最新版本
    async fn update(&self, install_id: Uuid) -> Result<InstalledResource>;

    /// 回滚到指定版本
    async fn rollback(&self, install_id: Uuid, version: &str) -> Result<InstalledResource>;

    /// 列出已安装资源
    async fn list_installed(&self, filter: InstallFilter) -> Result<Vec<InstalledResource>>;

    /// 启用/禁用资源
    async fn set_enabled(&self, install_id: Uuid, enabled: bool) -> Result<()>;

    /// 检查更新
    async fn check_updates(&self) -> Result<Vec<UpdateInfo>>;
}

/// 安装选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    pub version: Option<String>,           // 指定版本（默认最新）
    pub auto_update: bool,                 // 自动更新（官方默认 true）
    pub permissions: Vec<Permission>,      // 授权的权限列表
    pub sandbox_level: SandboxLevel,       // 沙箱级别
    pub config: serde_json::Value,         // 资源配置（API Key 等）
    pub install_path: Option<PathBuf>,     // 自定义安装路径（默认 plugins/）
}

/// 已安装资源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledResource {
    pub install_id: Uuid,
    pub resource_id: Uuid,
    pub name: String,
    pub version: String,
    pub trust_level: TrustLevel,
    pub install_path: PathBuf,             // 本地安装路径（plugins/wasm/xxx 或 plugins/scripts/xxx）
    pub config_path: PathBuf,              // 配置文件路径（data/ 目录）
    pub enabled: bool,
    pub auto_update: bool,
    pub installed_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
    pub permissions: Vec<Permission>,
    pub sandbox_level: SandboxLevel,
    pub status: InstallStatus,
}
```

#### 模块 3：安全与权限治理

```rust
/// 安全扫描器 trait
#[async_trait]
pub trait SecurityScanner: Send + Sync {
    /// 扫描资源包（静态分析）
    async fn scan_package(&self, package_path: &Path) -> Result<SecurityReport>;

    /// 运行时行为监控（动态分析）
    async fn monitor_runtime(&self, install_id: Uuid) -> Result<RuntimeBehavior>;

    /// 验证数字签名
    async fn verify_signature(&self, resource: &MarketResource) -> Result<bool>;
}

/// 安全报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub resource_id: Uuid,
    pub overall_score: f64,           // 总体安全评分（0-100）
    pub malware_detected: bool,       // 是否检测到恶意代码
    pub secrets_detected: Vec<String>,// 检测到的硬编码密钥
    pub network_access: Vec<String>,  // 网络访问目标
    pub file_access: Vec<String>,     // 文件访问路径
    pub process_spawn: Vec<String>,   // 进程创建
    pub vulnerabilities: Vec<VulnInfo>,// 已知漏洞
    pub signature_valid: bool,        // 签名是否有效
    pub recommendation: SecurityRecommendation,
}

/// 安全建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityRecommendation {
    Safe,           // 安全，可安装
    Caution,        // 谨慎，需用户确认
    Restricted,     // 受限，仅沙箱内运行
    Dangerous,      // 危险，不建议安装
    Blocked,        // 阻止，已加入黑名单
}
```

---

## 五、一键下载到本地：安装流程详解

### 5.1 安装流水线

```
用户点击"一键安装"
      │
      ▼
┌─────────────────────────────────────────────────────────┐
│ 1. 资源解析                                                │
│    • 根据 resource_id 获取 MarketResource 元数据           │
│    • 确定下载来源（注册中心 API / GitHub Release / npm）   │
│    • 确定包格式（MCPB bundle / WASM / Python 脚本 / Node）│
└──────────────────────────┬──────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 2. 安全预检（安装前）                                      │
│    • 验证数字签名（官方/认证资源必须）                      │
│    • 静态安全扫描（恶意代码/硬编码密钥/漏洞）               │
│    • 权限分析（需要哪些权限，是否超出最小权限原则）         │
│    • 根据信任等级决定：                                      │
│      🟢 官方 → 自动通过                                     │
│      🟡 认证 → 展示安全报告，用户确认                       │
│      🔴 社区 → 展示安全报告 + 二次确认 + 强制最高沙箱      │
└──────────────────────────┬──────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 3. 下载与缓存                                              │
│    • 下载资源包到 data/cache/market/                       │
│    • 校验 checksum（SHA-256）                              │
│    • 解压到临时目录                                         │
└──────────────────────────┬──────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 4. 安装到本地（架构-数据分离）                              │
│    • WASM 插件 → plugins/wasm/{name}/                     │
│    • 脚本插件 → plugins/scripts/{name}/                    │
│    • 扩展包   → plugins/extensions/{name}/                 │
│    • 配置文件 → data/storage/market/{name}/config.json    │
│    • 元数据   → data/storage/market/index.json             │
│    • 注意：所有运行时数据在 data/，所有代码在 plugins/，    │
│           均不在 platform/ 架构目录内                       │
└──────────────────────────┬──────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 5. 沙箱配置与注册                                          │
│    • 根据信任等级配置沙箱级别：                              │
│      🟢 官方 → 标准沙箱（资源限制）                         │
│      🟡 认证 → 增强沙箱（系统调用过滤）                     │
│      🔴 社区 → 最高沙箱（网络隔离+文件只读+进程隔离）       │
│    • 注册到 MCP 工具注册表（mox-market-api PluginRegistry）│
│    • 写入安装记录（install_id, 版本, 权限, 时间戳）        │
└──────────────────────────┬──────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 6. 安装完成通知                                            │
│    • 返回 InstalledResource 信息                           │
│    • 前端展示"安装成功" + 工具列表 + 使用入口              │
│    • 写入审计日志（谁/何时/安装了什么/信任等级/权限）      │
└─────────────────────────────────────────────────────────┘
```

### 5.2 一键安装命令

```bash
# CLI 一键安装（类似 brew install / npm install）
mox market install <resource-name>           # 安装最新版
mox market install <resource-name>@1.2.3     # 指定版本
mox market install <resource-name> --sandbox  # 强制沙箱
mox market install <resource-name> --no-auto-update  # 禁用自动更新

# 管理命令
mox market list                                # 列出已安装
mox market update <name>                       # 更新
mox market uninstall <name>                    # 卸载
mox market rollback <name>@1.2.0              # 版本回滚
mox market search <keyword>                    # 搜索
mox market info <name>                         # 查看详情+安全报告
```

---

## 六、全生命周期管理

### 6.1 版本管理

```
版本策略：SemVer (MAJOR.MINOR.PATCH)
├── MAJOR 版本变更 → 不兼容 API 变更，需用户手动确认升级
├── MINOR 版本变更 → 向下兼容的功能新增，可自动更新
└── PATCH 版本变更 → Bug 修复/安全补丁，强制自动更新

版本保留：
├── 官方资源 → 保留最近 3 个版本（支持回滚）
├── 认证资源 → 保留最近 2 个版本
└── 社区资源 → 仅保留当前版本（卸载即删除）
```

### 6.2 更新管理

| 信任等级 | 更新策略 | 通知方式 | 确认要求 |
|---|---|---|---|
| 🟢 官方 | 自动更新（PATCH）+ 通知（MINOR/MAJOR） | 后台静默 + 启动提示 | MAJOR 需确认 |
| 🟡 认证 | 通知更新，手动确认 | UI 通知 + 邮件 | 所有更新需确认 |
| 🔴 社区 | 仅通知有更新 | UI 红点提示 | 需用户主动操作 |

### 6.3 卸载与清理

```
卸载流程：
1. 停止资源运行（终止进程/关闭连接）
2. 从 MCP 注册表注销
3. 删除插件代码（plugins/wasm|scripts|extensions/{name}/）
4. 可选：删除配置与数据（data/storage/market/{name}/）
5. 可选：删除缓存（data/cache/market/{name}/）
6. 更新安装索引
7. 写入审计日志

清理策略：
• 卸载时默认保留配置数据 30 天（可恢复）
• 30 天后自动清理（可配置）
• 提供"彻底删除"选项（立即删除所有数据）
```

---

## 七、企业级安全治理

### 7.1 三级安全防线

```
┌─────────────────────────────────────────────────────┐
│ 第一防线：安装前安全扫描（静态）                       │
│  • 数字签名验证（官方/认证资源必须）                  │
│  • 恶意代码检测（特征码 + 行为分析）                  │
│  • 硬编码密钥扫描（API Key / Token / 密码）           │
│  • 已知漏洞匹配（CVE 数据库）                         │
│  • 许可证合规检查（GPL 传染 / 商业友好）              │
└──────────────────────┬──────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────┐
│ 第二防线：运行时沙箱隔离（动态）                       │
│  • WASM 沙箱（内存安全 + 能力隔离）                   │
│  • 资源限制（CPU/内存/磁盘/进程数硬限制）             │
│  • 系统调用过滤（seccomp-bpf 风格）                   │
│  • 网络隔离（白名单模式，默认禁止出站）               │
│  • 文件系统隔离（仅允许访问授权目录）                 │
│  • 超时机制（防止死循环/资源耗尽）                    │
└──────────────────────┬──────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────┐
│ 第三防线：行为审计与异常响应（事后）                   │
│  • 全链路操作日志（谁/何时/调用了什么工具/参数）      │
│  • 异常行为检测（异常网络访问/文件操作/进程创建）     │
│  • 自动隔离（检测到恶意行为 → 立即终止 + 隔离）      │
│  • 威胁情报共享（恶意资源加入全局黑名单）             │
│  • 合规报告（等保/ISO27001/GDPR 审计导出）           │
└─────────────────────────────────────────────────────┘
```

### 7.2 权限模型（最小权限原则）

```
权限分类：
├── 📁 文件系统权限
│   ├── fs.read:<path>      # 读取指定路径
│   ├── fs.write:<path>     # 写入指定路径
│   └── fs.exec:<path>      # 执行指定路径
├── 🌐 网络权限
│   ├── net.outbound:<host> # 出站访问指定主机
│   ├── net.inbound:<port>  # 入站监听指定端口
│   └── net.dns              # DNS 解析
├── ⚙️ 进程权限
│   ├── proc.spawn           # 创建子进程
│   ├── proc.kill            # 终止进程
│   └── proc.env             # 读取环境变量
├── 🗄️ 数据库权限
│   ├── db.query:<db>        # 查询指定数据库
│   └── db.mutate:<db>       # 修改指定数据库
└── 🔑 密钥权限
    ├── secret.read:<name>   # 读取指定密钥
    └── secret.use:<name>    # 使用指定密钥（不暴露明文）

授权策略：
• 🟢 官方资源 → 申请即授权（可配置为需确认）
• 🟡 认证资源 → 逐项申请，用户确认
• 🔴 社区资源 → 默认仅 stdout，其他需显式授权 + 二次确认
```

### 7.3 企业私有市场

```
企业部署模式：
├── 公有云模式（SaaS）
│   └── 使用 MOX 官方商场中心，资源来自公开注册中心
├── 私有部署模式（On-Premise）
│   ├── 内部资源市场（企业自研插件）
│   ├── 白名单机制（仅允许安装审核通过的资源）
│   ├── SSO 集成（企业统一身份认证）
│   ├── 审计日志对接企业 SIEM
│   └── 离线安装包支持（内网环境）
└── 混合模式
    ├── 内部资源 + 精选公开资源
    └── 所有公开资源需经企业安全审核后才能安装
```

---

## 八、与现有 MOX 架构的融合

### 8.1 模块映射

| MMC 组件 | 新增/扩展 | 复用现有 |
|---|---|---|
| 市场聚合 API | `mox-market-api` (L2，扩展现有) | — |
| 聚合索引核心 | `mox-market-core` (L3，新增) | — |
| 商场中心服务 | `mox-market-center-svc` (L4，新增) | `mox-market-template-svc` |
| 安装管理 | 扩展 `mox-market-center-svc` | `mox-platform-paths`（路径管理） |
| 安全扫描 | `mox-market-security` (L3，新增) | `mox-data-compliance-svc`（PII 检测） |
| 沙箱执行 | — | `mox-flow-operator-wasm-svc`（WASM 沙箱） |
| 审计日志 | — | `mox-platform-observability` |
| 权限治理 | — | `mox-platform-iam-core`（RBAC） |

### 8.2 与 MOX 总任务中心的协作

```
MOX 总任务中心 (MTC)                    MOX 商场中心 (MMC)
┌─────────────────────┐                ┌─────────────────────┐
│ 任务理解 → 分解 → 路由│ │ 资源聚合 → 分类 → 安装 │
│                     │◄───需要工具───│                     │
│  编排执行            │    时查询可用   │  本地仓库管理        │
│                     │───调用工具────►│                     │
│  治理安全            │    时获取权限   │  安全沙箱           │
└─────────────────────┘                └─────────────────────┘

协作流程：
1. MTC 收到任务 → 分析需要哪些工具/AI/软件
2. MTC 查询 MMC：本地已安装哪些可用工具
3. 若未安装 → MMC 搜索商场 → 推荐最合适的资源
4. 用户确认 → MMC 一键下载安装 → 注册到工具表
5. MTC 调用已安装工具 → 通过 MMC 沙箱执行
6. 执行结果 → MTC 编排后续步骤
```

---

## 九、实施路线图

| 阶段 | 里程碑 | 核心交付 | 时间 |
|---|---|---|---|
| **M0** | 基础框架 | 统一元数据模型 + 本地仓库 + 安装/卸载 CLI | 2 周 |
| **M1** | 聚合搜索 | Official Registry + Smithery + Glama 三源聚合 + 搜索 UI | 2 周 |
| **M2** | 三级分类 | 官方/认证/社区分类体系 + 信任等级 + 不同处理模式 | 2 周 |
| **M3** | 安全治理 | 静态扫描 + 签名验证 + WASM 沙箱 + 权限模型 | 3 周 |
| **M4** | 全生命周期 | 版本管理 + 更新/回滚 + 自动更新 + 审计日志 | 2 周 |
| **M5** | 企业级 | 私有市场 + SSO + 白名单 + 离线安装 + SIEM 对接 | 3 周 |
| **M6** | 生态扩展 | 10+ 注册中心全聚合 + 5万+ 资源索引 + 开源贡献 | 持续 |

---

## 十、总结

### 10.1 核心价值

| 维度 | 价值 |
|---|---|
| **对用户** | 一个商场找遍所有 AI 工具/MCP/插件，一键安装即用，不用再到处找 |
| **对开发者** | 统一安装/更新/卸载/回滚，类似 brew/npm 的包管理体验 |
| **对企业** | 三级信任体系 + 三层安全防线 + 最小权限 + 全链路审计，解决 JetBrains 式的插件安全事件 ["https://mingooland.com/2026/06/jetbrains-marketplace-ecosystem-security-update-addressing-malicious-third-party-ai-plugins/"] |
| **对生态** | 聚合 10+ 注册中心 5万+ 资源，统一分类，反向贡献开源 |

### 10.2 关键设计决策

1. **三级分类（官方/认证/社区）**：不同信任等级 = 不同处理模式，从"自动安装"到"强制最高沙箱"
2. **架构-数据分离**：插件代码放 `plugins/`，配置数据放 `data/`，均不在 `platform/` 架构目录内
3. **标准协议优先**：基于 MCP 标准，不造轮子，聚合整个开源生态
4. **安全左移**：安装前扫描 → 运行时沙箱 → 事后审计，三层防线
5. **企业可私有化**：支持 On-Premise 私有市场 + 白名单 + SSO + 离线安装

### 10.3 一句话定义

**MOX 商场中心 = AI 时代的 "App Store + Homebrew + Docker Hub"**：聚合 10+ 注册中心 5万+ MCP/AI/插件资源，三级分类（官方/认证/社区）差异化处理，一键下载到本地沙箱，全生命周期企业级管理。
