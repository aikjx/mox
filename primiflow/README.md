# PrimiFlow MVP（竖向切片 · 离线可跑）

> 验证「客户语音/文字输入 → 自动出拓扑 → 自己拖拽编辑流程图 → 出 8 份说明书」主链路闭环。
> 用**规则化拓扑生成器**代替 LLM（无需 API Key，离线可跑）。生产环境按 `SPEC.md` 拆 Go+Python 并接真实模型。

## 运行

```bash
cd primiflow/backend
pip install -r requirements.txt
uvicorn main:app --reload --port 8000
# 浏览器打开 http://localhost:8000/
```

前端由后端同一进程托管（`/` 返回 `web/index.html`）。

## 你能做什么（客户视角）

1. **输入**：点 🎤 用语音说，或在文本框打字描述业务（支持电商/用户权限/消息通知等关键词触发 κ 复用资产）。
2. **滑块**：稳定 ↔ 探索（κ/τ 配比）。稳定=优先复用历史资产；探索=自动加监控/实验/审计等节点。
3. **生成**：点「⚡ 生成系统」→ 自动产出拓扑 DAG + 8 份说明书。
4. **改流程图**：画布上「＋加节点 / 🔗连线 / 🗑删选中」自由编辑；点「↻ 重算 ℛ̂」把编辑结果回写后端重算（预算裁剪 + 矛盾环拦截）。
5. **沉淀**：「❄ 冻结资产」把合格拓扑存为可复用资产 Q（生产走 pgvector 语义检索）。

## 接口一览

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/projects` | 建项目 |
| POST | `/api/projects/:id/messages` | 文字+滑块 → 拓扑（ℛ̂ 已裁剪） |
| GET  | `/api/topologies/:id` | 取 DAG |
| POST | `/api/topologies/:id/update` | 画布编辑回写 + 重算 ℛ̂ |
| POST | `/api/projects/:id/generate-docs` | 生成 8 文档 |
| GET  | `/api/projects/:id/artifacts` | 取文档 |
| POST | `/api/topologies/:id/freeze` | 冻结资产 Q |

## 目录

```
primiflow/
├── SPEC.md          # 工程规格（κ/τ/C/Q + ℛ̂ 落地定义）
├── ARCHITECTURE.md  # 主架构设计（客户旅程/插件融合/如何更好开发）
├── backend/
│   ├── engine.py    # 拓扑生成 + ℛ̂ 正则化 + 8 文档 + 六维溯源
│   ├── main.py      # FastAPI 路由 + 内存存储 + 静态托管
│   └── requirements.txt
├── web/index.html   # React+Cytoscape 单文件前端（含 Web Speech 语音输入）
└── README.md
```

## 已知边界（承接核验报告命题3）

- 规则生成器非 LLM：复杂需求的结构化质量有限；接真实模型后质量跃升。
- κ‑τ 自动寻优未做（规则+滑块+日志，V2 接 RL）。
- 仅业务软件域；超域任务未拦截（MVP 简化）。
- 内存存储：重启即丢；生产换 PostgreSQL + pgvector。
