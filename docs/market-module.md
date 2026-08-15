# 算子商城模块（Operator Market Module）· 企业级设计文档

> 版本：v1.0 ｜ 状态：已落地（后端 `market.rs` + 前端 `MarketView.vue` / `MarketDetailView.vue`）
> 定位：将"**需求 + 业务流程图（结构化、可编辑）**"作为可复用资产（算子包，OperatorPackage）沉淀到商城，供他人**随机浏览、克隆、继续编辑**，实现"需求确定后其他皆可快速改"的企业级知识复用闭环。
> 与 `architecture.md` 的关系：本模块是 §18「开放生态与算子市场」的**需求/流程图资产侧**补充，与 §28「业务流程设计模块」构成"设计↔市场"双向飞轮。

---

## 1. 模块定位与价值

### 1.1 为什么需要算子商城（资产层 vs 执行层）

OUS 的 `flow-ai` / `optimizer` / `ai-agent` 解决的是"**怎么跑**一个流程"，
但企业最稀缺的是"**该建什么流程、需求怎么定**"的经验资产。
算子商城把"对话中反复纠结、很深的需求与业务流程知识"固化下来，使：

- **上传方（沉淀）**：把需求描述、业务流程图、功能点清单打包成算子包上传；
- **浏览方（发现）**：支持列表/分类/搜索/**随机**浏览，一键拉取；
- **使用方（复用）**：克隆（fork）后进入可视化编辑器**继续编辑**，需求定了其他改起来极快。

> 核心原则（项目最高优先级）：**需求一旦确定，流程图与功能点都可据此快速调整**。
> 因此需求（`requirement`）是算子包的**必填核心字段**，流程图/功能点为可演进的结构化数据，**绝不存成死图片**。

### 1.2 与既有市场概念的区别

| 维度 | §18 算子市场（WASM 算子） | 本模块（需求/流程图资产） |
|------|--------------------------|--------------------------|
| 资产形态 | 可执行 `.wasm` 算子 | 需求 + 流程图 + 功能点（JSON） |
| 目标用户 | 算子开发者 | 业务架构师 / 产品 / 任何想复用流程的人 |
| 核心交付 | 可运行能力 | **可编辑、可理解的业务知识** |
| 编辑方式 | 编译进 WASM | 前端 SVG 可视化拖拽编辑器（零依赖） |

两者互补：本模块产出的"流程图"可经 §28 流程 DSL 转换为可执行的 `BusinessWorkflow`。

---

## 2. 数据模型（OperatorPackage）

后端 `crates/runtime/src/market.rs` 定义，持久化为 `./data/market/<id>.json`（文件型，无需数据库）。

### 2.1 字段定义

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 包唯一 ID（UUID 前段），服务端生成 |
| `name` | string | 是 | 算子包名称 |
| `category` | string | 否 | 分类（如「平台/编排」「AI」「数据」） |
| `author` | string | 否 | 作者（克隆后可改写） |
| `version` | string | 否 | 语义版本，默认 `1.0.0` |
| `summary` | string | 否 | 一句话简介 |
| `requirement` | string | **是** | ★ 需求描述，最核心字段 |
| `nodes` | `FlowNode[]` | 否 | 流程图节点（可编辑） |
| `edges` | `FlowEdge[]` | 否 | 流程图连线（可编辑） |
| `features` | `FeatureItem[]` | 否 | 功能点清单 |
| `tags` | `string[]` | 否 | 标签 |
| `created_at` | string | 是 | RFC3339 创建时间 |
| `updated_at` | string | 是 | RFC3339 更新时间 |
| `clone_count` | u64 | 是 | 被克隆次数 |
| `forked_from` | string? | 否 | 派生源包 ID（克隆溯源） |

### 2.2 内嵌结构

```rust
// 流程图节点
struct FlowNode {
    id: String, label: String,
    node_type: String,   // start|end|process|decision|io|operator
    x: f64, y: f64,      // 画布坐标
    note: String,        // 节点备注
}
// 流程图连线
struct FlowEdge {
    id: String, source: String, target: String, label: String,
}
// 功能点
struct FeatureItem {
    id: String, title: String, description: String,
    priority: String,    // high|medium|low
    status: String,      // todo|doing|done
}
```

> 数据流契约：`nodes`/`edges` 是结构化 JSON，**前端 SVG 编辑器直接读写**，因此克隆后别人可继续编辑——这是"资产可演进"的关键。

---

## 3. API 契约（REST）

前缀 `/api/market`，全部由 `market::market_routes()` 挂载（见 `crates/runtime/src/main.rs:430`）。
**读取类 GET 接口免登录白名单**（见 `main.rs:523`），写操作（上传/更新/克隆/删除）受 `auth_middleware` 鉴权。

| 方法 | 路径 | 说明 | 请求/响应要点 |
|------|------|------|--------------|
| GET | `/api/market/` | 列表（支持 `?category=` `?tag=` `?q=` 过滤） | 返回 `{success,total,packages:[PackageMeta]}` |
| GET | `/api/market/random` | 随机返回一个包（"随机剪饮"） | 302 到对应 `/:id` 内容 |
| GET | `/api/market/:id` | 获取完整包（含需求/流程图/功能点） | `{success,package:OperatorPackage}` |
| POST | `/api/market/upload` | 上传新包 | Body: `CreatePackageRequest`，**`name`+`requirement` 必填** |
| POST | `/api/market/:id` | 全量更新包核心字段 | Body: `UpdatePackageRequest`（各字段可选） |
| POST | `/api/market/:id/clone` | 克隆（fork）：源包 `clone_count+1`，生成新包并溯源 | 返回新包 `{success,id,package}` |
| DELETE | `/api/market/:id` | 删除包 | 删除 `./data/market/<id>.json` |

### 3.1 错误约定

- 名称/需求为空 → `400 {success:false,error:"..."}`（需求为空时提示"这是最核心的部分"）
- 包不存在 → `{success:false,error:"算子包不存在"}`
- 写入失败 → `500 {success:false,error:"保存失败: ..."}`

---

## 4. 前端（Vue3 + Element Plus）

| 页面 | 路由 | 职责 |
|------|------|------|
| `MarketView.vue` | `/market` | 列表/分类/搜索/随机/上传弹窗 |
| `MarketDetailView.vue` | `/market/:id` | 需求编辑 + 功能点编辑 + **可拖拽 SVG 流程图编辑器** + 克隆/保存 |

导航项已在 `src/types.js` 的 `NAV_MODULES` 增加 `{key:'market',label:'算子商城',icon:'Shop',path:'/market'}`。
API 封装已在 `src/api/index.js` 增加 7 个函数（`marketList/marketRandom/marketGet/marketUpload/marketUpdate/marketDelete/marketClone`）。

### 4.1 可编辑流程图编辑器（零依赖实现）

`MarketDetailView.vue` 用原生 SVG + DOM 实现，无第三方流程图库依赖：

- **拖拽**：`mousedown` 节点记录偏移，`mousemove` 实时改 `n.x/n.y`，`mouseup` 释放；
- **节点类型**：`process/start/end/decision/io/operator`，样式按类型区分（如 start=绿、end=红、decision=橙菱形）；
- **增删**：「加节点」按当前 `newNodeType` 生成；选中后「删节点」同时清理相关边；
- **连线**：「连线模式」下先点起点再点终点，自动建 `FlowEdge`，连线渲染带箭头 marker + 标签；
- **编辑属性**：节点内 `el-popover` 可改 `label`/`note`；
- **持久化**：所有改动经 `marketUpdate` 写回 `OperatorPackage.nodes/edges`。

> 该编辑器产出的就是 §2.2 的结构化数据，可直接对接 §28 流程 DSL 转换为可执行 `BusinessWorkflow`。

---

## 5. 种子数据与启动

`ensure_seed()`（market.rs:551）在首次启动、且 `./data/market/` 为空时，写入一份示例包：
`seed-ous-full-flow`「算子统一系统·全业务流程」，含完整需求、7 节点流程图、4 功能点，
作为"需求确定→流程图可快速改"的示范资产。

启动命令（与全局一致）：
```bash
cargo run -p runtime --bin operator-server   # 默认 3000 端口
# 前端 npm run dev 后，侧边栏「算子商城」进入
```

---

## 6. 与企业级架构的衔接

| 架构章节 | 本模块落点 |
|----------|-----------|
| §18 开放生态与算子市场 | 补"需求/流程图资产市场"侧，与 WASM 算子市场并列 |
| §27 路径与运行态隔离 | 数据落 `./data/market/`，属 WORK_PATH（运行态），**不入源码树**（与 §27.4 规范一致，应迁移到 `$OUS_HOME/market`） |
| §28 业务流程设计模块 | 商城是流程资产的"市场/版本化/复用"出口；克隆即"流程的组合递归" |
| §23 记忆与知识管理 | 商城可视为"程序性/程序化知识"的外部化资产库 |

### 6.1 后续演进建议（待办）

1. **路径合规**：将 `./data/market` 改为 `$OUS_HOME/market`（遵循 §27），避免写源码目录；
2. **版本化**：克隆时生成 `name@version`，支持同包多版本（呼应 §26）；
3. **导出/导入**：算子包 JSON 支持导出文件 + 导入，便于跨实例分发（呼应 §18 跨形态同步）；
4. **流程图→可执行**：`MarketDetailView` 增加「导出为 §28 FlowDefinition DSL」按钮，一键转 `BusinessWorkflow`；
5. **权限/归属**：上传时绑定 `author` 与租户（`agent.ctx` 作用域），支持"我的上传"管理视图。

---

*本模块是 OUS「需求驱动、资产复用、可演进」理念的前端落地：把最贵的"业务知识"变成可浏览、可克隆、可编辑的一等资产。*
