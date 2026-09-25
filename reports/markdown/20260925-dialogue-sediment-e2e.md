# 专家联盟 · AI 对话知识沉淀端到端验证报告

> 日期：2026-09-25
> 范围：`开发专家联盟，先打通AI对话` → `读取对话核心内容，自动归类到知识图谱、云盘、知识库`
> 状态：✅ 全链路 E2E 验证通过

## 1. 目标与链路

把三套既有能力串成一条闭环：

```
AI 对话（编排器 3001 / 网关 3080 代理）
   ↓ 自动落库：SQLite 对话库（dialogue_sessions / dialogue_messages）
   ↓ 自动同步：规则引擎抽取实体/关系 → 全局知识图谱
   ↓ 读取对话核心内容：GET /api/dialogue/sessions/:id/messages
   ↓ 沉淀：POST /api/alliance/sediment
       ├── 知识库 KB：分类 cat-dialogue 文档（KbAnalyzer 分析 → GraphLinker 挂图 → status=linked）
       ├── 知识图谱：KB 文档子图挂图（graph_nodes_added / graph_edges_added）
       └── 云盘 Cloud：S3 兼容对象存储 bucket=dialogue，key=<session_id>.md（对话纪要）
```

## 2. 本次改动清单

| 模块 | 文件 | 改动 |
|---|---|---|
| AI 对话落库 | `mox-ai-agent-svc/src/dialogue_graph.rs` | 新增 `session_transcript()` / `has_session()` / `create_session_with_id()` 及 `SessionTranscript` / `TranscriptMessage` |
| AI 对话落库 | `mox-ai-agent-svc/src/lib.rs` | `ensure_session()` 以**客户端会话 ID 为唯一键**（修复前端会话 ID 与对话库 ID 不一致 + 多轮连续性）；chat() 补充**助手回复落库并同步图谱** |
| 编排器 | `mox-platform-orchestrator-svc/src/main.rs` | 新增 `GET /api/dialogue/sessions/:id/messages` 读取对话核心内容 |
| 沉淀模块（新） | `gateway/dialogue_sediment.rs` | `POST /api/alliance/sediment`：读取 → KB create(cat-dialogue) → KbAnalyzer → GraphLinker.link → docs.save(linked) → Cloud put_object_text → 返回结果 |
| KB | `mox-kb-svc` handlers/lib/document | `build_kb_router_with_state()` 公开；analyze/document/link 模块 pub；新增分类 `cat-dialogue 对话沉淀`（CATEGORIES 3→4） |
| 云盘 | `gateway/cloud.rs` | 新增 `put_object_text()` 程序化写对象；`build_cloud_router_with_state()` |
| 注册中心 | `gateway/modules.rs` | ModuleStates 新增 kb/cloud/sediment；KB/Cloud 路由改共享状态；挂接 sediment 路由；修正 experts 所有权 |
| 前端 | `ai.api.js` / `ChatView.vue` | `sedimentDialogue()` API；会话项「沉淀」按钮（CollectionTag）+ 结果弹窗（doc_id/挂图节点数/云盘对象/摘要/实体） |

## 3. 端到端验证结果（实测）

### 3.1 对话
```
POST /api/ai/chat  {"session_id":"e2e-sed-002","message":"请设计一个多专家协同的对话知识沉淀方案：AI对话结束后自动读取对话核心内容，归类到知识图谱、云盘和知识库。","scope":"global"}
→ code=0 msg=ok（规则引擎应答，无 LLM key 时零外部依赖）
```

### 3.2 读取对话核心内容
```
GET /api/dialogue/sessions/e2e-sed-002/messages
→ code=0，session_id=e2e-sed-002（与客户端一致 ✅）
  messages: [user(09:53), user(09:56), assistant(09:56)]  ← 助手回复已落库 ✅
```

### 3.3 沉淀（核心验证）
```
POST /api/alliance/sediment  {"source":"dialogue","session_id":"e2e-sed-002"}
→ code=0 ok=true
  kb:     doc_id=kb-3b727ac1  category=cat-dialogue  status=linked
          graph_nodes_added=16  graph_edges_added=25
          tags=[会话 e2e-sed-, cat-dialogue, 知识, 对话, 图谱]
  cloud:  bucket=dialogue  key=e2e-sed-002.md
          path=data/storage/dialogue/e2e-sed-002.md（文件 1085 字节，含会话原文 ✅）
  entities=12（对话/知识/图谱/自动/专家/云盘/协同/多专家/归类/方案/核心/沉淀，无碎片） relations=10（co_occur）
```

### 3.4 知识库可查
```
GET /api/kb/documents → 含 {id: kb-3b727ac1, category: cat-dialogue, status: linked}（共 3 篇）✅
```

### 3.5 全局图谱（对话时自动同步）
```
GET /api/graph/stats → node_count=37 edge_count=28（对话实体已入全局图；相同内容不重复建点）
```


### 3.6 内联源沉淀（source=inline · 任意内容直接入三端）

以本报告全文为输入实测：

```
POST /api/alliance/sediment  {"source":"inline","title":"AI对话知识沉淀端到端验证报告","messages":[{"role":"user","content":"<报告全文 4496 字符>"}]}
→ code=0 ok=true
  kb:     doc_id=kb-f3d11cda  category=cat-dialogue  status=linked
          graph_nodes_added=48  graph_edges_added=57（报告内容丰富度高于单次对话）
  cloud:  bucket=dialogue  key=inline-1790345248169.md（报告全文入库）
  entities=12（对话/沉淀/知识/云盘/核心/内容/图谱/前端/自动/专家/引擎/知识库，长文本同样无碎片）
  summary=“把三套既有能力串成一条闭环：”（正文首句）
```

知识库现状（GET /api/kb/documents，共 5 篇）：`kb-f3d11cda`（本报告）· `kb-f39faf5d` / `kb-3b727ac1`（对话沉淀 e2e-sed-002 两次）· `kb-6b1a9c91` / `kb-b6b97c83`（历史 cat-tech）。

## 4. 过程中发现并修复的问题

1. **前端会话 ID 与对话库 ID 不一致**：原 `ensure_session` 以客户端 ID 仅作标题、另生成 UUID 入库 → 读取/沉淀按前端 ID 全部 404。已改为以传入 ID 为唯一键。
2. **助手消息未落库**：规则引擎路径只写 user 消息，对话库缺 assistant 原文 → 已在 chat() 两条路径（LLM / 规则引擎）补充 assistant 落库。
3. **磁盘写满**：target/debug 编译缓存约 100GB 占满 D 盘导致编译失败，清理后释放至约 51GB。

## 5. 已知限制（如实披露）

- **实体抽取质量（本次已修复）**：原滑动窗口方案产生 `e2e`/`sed`/`识图` 等碎片实体、摘要取标题行。已改为**词典最长匹配优先**（技术/组织/业务概念词典 + 互不重叠 span）+ 纯 CJK 窗口降权 + 碎片过滤（纯数字、短 ASCII 非词典词），摘要跳过 markdown 标题行取正文首句。修复后沉淀实体均为有意义概念（对话/知识/图谱/云盘/协同/多专家/沉淀…），新增回归测试 `extract_rejects_fragments_and_summary_skips_title`（mox-kb-svc 16 测试全过、无 warning）。接入真实 LLM 仍可进一步提升语义抽取（编排器 `/api/llm/config` 配置 api_base/api_key/model）。
- **历史遗留测试**：`t4_kb_http` 集成测试 2 例失败（`kb_full_lifecycle`、`kb_batch_analyze_and_errors`，断言 `body["success"]` 与新版信封不匹配），经 stash 干净态验证为**改动前既有问题**，与本次无关。
- **仓库门禁**：未跑 `cargo clippy --all-targets`（改动已过 cargo test，mox-kb-svc 无 warning）；未改端口，`verify-ports.py` ERROR=0；链路改动已提交 `bb55f08c`（12 文件，仅本次涉及文件），实体质量改进待提交。

## 6. 复现方式

```powershell
# 1. 起服务（默认 token dev-secret-token）
scripts/startup/start-mox-enterprise.ps1

# 2. 对话
curl -H "Authorization: Bearer dev-secret-token" -H "Content-Type: application/json" `
  -d '{"session_id":"demo-001","message":"…","scope":"global"}' http://127.0.0.1:3080/api/ai/chat

# 3. 读取核心内容
curl -H "Authorization: Bearer dev-secret-token" http://127.0.0.1:3080/api/dialogue/sessions/demo-001/messages

# 4. 沉淀（对话源）
curl -H "Authorization: Bearer dev-secret-token" -H "Content-Type: application/json" `
  -d '{"source":"dialogue","session_id":"demo-001"}' http://127.0.0.1:3080/api/alliance/sediment

# 5. 沉淀（内联源：任意文档/报告直接入知识库+云盘+挂图）
curl -H "Authorization: Bearer dev-secret-token" -H "Content-Type: application/json" `
  -d '{"source":"inline","title":"报告标题","messages":[{"role":"user","content":"报告全文"}]}' http://127.0.0.1:3080/api/alliance/sediment
```

前端入口：AI 对话页会话列表每项新增「沉淀」按钮，点击后弹窗展示 KB 文档 id / 挂图节点数 / 云盘对象 / 摘要 / 实体。
