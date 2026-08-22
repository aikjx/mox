# 引擎内核（Engine Kernel）——一切皆可插件化

> 版本：1.0 · 状态：已上线 · 架构遵循 AINA-STD-001（域包模式：routes → application → domain ← infrastructure）

## 1. 设计哲学

**一切皆是项目，一切皆可插件化。** 系统的每类能力定义为一个**槽位（Slot）**，每个槽位有一份**标准契约（Contract）**——方法签名 + 输入输出规范。任何满足契约的引擎都可插入槽位：

- **切换引擎 = 换绑定**：指定架构路径（槽位 + 引擎 ID）即决定用哪个引擎，零代码改动，瞬间生效，无需删除旧引擎。
- **代码之间只是调用**：调用方只依赖槽位契约，不依赖具体引擎实现。换 AI 引擎、换存储、换搜索引擎、换音高检测后端，都是同一个动作：`POST /engine-kernel/switch`。
- **三层商城供给插件**：系统内置（随版本发布）/ 云端目录（registryUrl 可指向任意注册表）/ 本地清单（JSON 声明安装）。
- **AI 可代替人配置**：自然语言需求 → LLM 决策（候选合法性机器校验）→ 引擎绑定方案 → 可自动应用。

## 2. 槽位契约（当前 4 槽位）

| 槽位 | 能力 | 候选引擎（动态） | 切换落点 |
|---|---|---|---|
| `ai-chat` | 大模型对话 | OpenAI/Claude/豆包/千问/Kimi/DeepSeek/智谱/Gemini/Ollama/自装 | `llm-gateway.setActiveProvider` |
| `storage` | 数据持久化 | SQLite/MySQL/PostgreSQL | `config.switchProvider` |
| `web-search` | 联网搜索 | Bing/DuckDuckGo/Tavily/博查/SearXNG | `webSearchService.updateConfig` |
| `pitch-detection` | 音高检测 | auto/crepe_onnx/pyin/torchcrepe | 绑定持久化 + 代理注入 `backend` 查询参数（Python 端 `cfg.preferred_backend`） |

契约文档即代码：`src/engine-kernel/domain/contract-registry.js`（GET `/engine-kernel/contracts/:slot` 原样输出）。

## 3. 瞬间切换流程（algo-switch-rollback）

```
校验（槽位存在 + 引擎在候选清单）
  → 应用（适配器 apply：改网关/改配置/改绑定）
  → 契约探活（health：真实调用引擎验证）
  → 失败自动回滚原绑定（银行级不宕机）
  → 成功则持久化 engine_bindings.json
```

## 4. 三层插件商城

| 层次 | 名称 | 来源 | 安装语义 |
|---|---|---|---|
| L1 | 系统商城 | 槽位内置引擎（自动生成） | 无需安装，直接切换 |
| L2 | 云端商城 | 云端目录（LLM 预设 + 密钥型搜索引擎 + registryUrl 外部注册表） | `kind=llm-provider` → `gateway.addProvider`；`kind=web-search-key` → 写密钥并切换 |
| L3 | 本地商城 | 本地 JSON 清单（`manifest: {id, slot, kind, installConfig}`） | 落盘 engine_plugins.json + 应用安装落点 |

云端不可达时静默降级为预置目录（银行级可用性）。

## 5. AI 自动配置

`POST /engine-kernel/ai-configure {"requirement": "...", "dryRun": true}`

流程：槽位上下文（契约+候选+当前绑定）注入 LLM → 输出严格 JSON 方案 → **候选合法性校验**（AI 只能在机器验证过的候选清单内决策，不能凭空造引擎）→ dryRun 仅出方案 / autoApply 逐项切换。

## 6. API 一览

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/engine-kernel/slots` | 槽位全景（契约+当前绑定+候选） |
| GET | `/engine-kernel/contracts/:slot` | 契约文档原文 |
| GET | `/engine-kernel/bindings` | 持久化绑定 vs 实时绑定一致性 |
| POST | `/engine-kernel/switch` | 瞬间切换 `{slot, engineId, verify?}` |
| POST | `/engine-kernel/validate` | 契约预检（探活不切换） |
| GET | `/engine-kernel/marketplace` | 三层商城总览 |
| GET/POST | `/engine-kernel/marketplace/config` | 云端注册表 registryUrl 配置 |
| POST | `/engine-kernel/marketplace/install` | 安装插件（cloud/local） |
| POST | `/engine-kernel/marketplace/uninstall` | 卸载插件（system 内置不可卸载） |
| POST | `/engine-kernel/ai-configure` | AI 自动配置 |

## 7. 扩展新槽位（开发者指南）

1. `domain/contract-registry.js` 登记 SLOT_CONTRACTS（契约即文档）
2. `infrastructure/plugin-repository.js` 增加适配器（list/current/apply/health）并挂入 ADAPTERS
3. 调用方按契约调用槽位——无需感知具体引擎

## 8. 数据资产

| 文件 | 用途 |
|---|---|
| `engine_bindings.json` | 槽位绑定（含实时一致性视图） |
| `engine_plugins.json` | 本地安装插件清单 |
| `engine_marketplace.json` | 云端注册表配置 |

## 9. 图谱关联

- 业务域：`engine-kernel`（routes/engine-kernel.js）
- 引擎节点：`engine-kernel`（+复用 llm-gateway、web-search-service）
- 算法：`algo-slot-contract`（槽位契约路由）、`algo-switch-rollback`（切换探活回滚）
- 无破窗：W1 路由域/W2 数据资产均已动态比对覆盖
