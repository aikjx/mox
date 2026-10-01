# LLM 配置持久化（重启不丢）报告 — 编排器 mox-platform-orchestrator-svc

> 日期：2026-09-27 · 编排器 operator-server（3001）
> 构建：`cargo build -p mox-platform-orchestrator-svc` 通过（2m45s，修复 unused import 后干净）
> 单测：`cargo test -p mox-platform-orchestrator-svc llm_persist` → 4 passed
> 硬约束：禁桩（真实文件 IO）；不破坏 env 启动配置（读盘优先，无盘回退 env）；api_key 永不回传明文。

---

## 0. 结论

- 新增 `llm_persist.rs`：配置文件 `<cwd>/.runtime/llm-config.json`，原子写盘（临时文件→rename）、容错读回。
- `update_llm_config` 成功后同步落盘；启动时先读盘（存在则覆盖 env 默认），无盘文件回退 `DEEPSEEK_API_KEY`。
- api_key 磁盘保留（LLM 客户端运行需要），HTTP 响应只回 `has_api_key: bool`。
- **重启实测通过**：POST `model=persist-probe` → 磁盘文件生成 → 重启 operator-server → GET 仍返回 `model=persist-probe`。

---

## 1. 现状（改造前）

- `GET/POST /api/ai/llm/config`、`POST /api/ai/llm/test` 为内存态（`AIAgent.llm_client()` RwLock）。
- 启动时 `main.rs:355-372` 仅读 `DEEPSEEK_API_KEY` env；进程退出即丢配置。

## 2. 持久化设计

| 项 | 值 |
| --- | --- |
| 文件路径 | `<cwd>/.runtime/llm-config.json`（运行态目录，与 data/ 同族，gitignore） |
| 写 | `persist_llm_config(&LLMConfig)`：序列化 pretty JSON → 写 `*.json.tmp` → `rename`（原子替换） |
| 读 | `load_persisted_llm_config() -> Option<LLMConfig>`：读文件+反序列化；缺失/损坏一律 `None` |
| 优先级 | 启动：**盘文件 > env**。盘存在即用盘；否则回退 `DEEPSEEK_API_KEY` |
| 脱敏 | `masked_view()` 出参只含 `api_base/model/temperature/max_tokens/enabled/has_api_key`，不含 `api_key` |

## 3. 改动文件

```
platform/domains/platform/svc/mox-platform-orchestrator-svc/src/llm_persist.rs   (新增：persist/load/masked_view + 4 单测)
platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs          (mod 注册；启动读盘优先；update 落盘+脱敏)
```

## 4. 单测结果（4/4 通过）

```text
test llm_persist::tests::persist_then_load_roundtrip ... ok      # 写盘→读回字段一致
test llm_persist::tests::masked_view_never_leaks_key ... ok       # 响应视图不含 api_key 明文
test llm_persist::tests::corrupt_file_falls_back_to_none ... ok   # 损坏 JSON → None（回退 env）
test llm_persist::tests::missing_file_returns_none ... ok         # 无文件 → None
```

## 5. 重启实测（operator-server :3001，2026-09-27）

```text
[重启后基线] GET /api/ai/llm/config
{"api_base":"https://api.openai.com/v1","model":"gpt-3.5-turbo","enabled":false,"has_api_key":false,"max_tokens":2048,"temperature":0.7}
# （无盘文件、无 DEEPSEEK_API_KEY env → 默认）

[POST update] body={api_base:deepseek, api_key:"sk-probe-abc123", model:"persist-probe", temperature:0.5, max_tokens:1024, enabled:true}
响应: {"success":true,"persisted":true,"config":{"model":"persist-probe","has_api_key":true,...}}  # 无明文 Key

[磁盘文件] .runtime/llm-config.json 已生成（含 api_key，运行态）

[再次重启 operator-server] GET /api/ai/llm/config
{"api_base":"https://api.deepseek.com/v1","model":"persist-probe","enabled":true,"has_api_key":true,"max_tokens":1024,"temperature":0.5}
# ↑ 跨重启保持：证明启动读盘生效
```

## 6. 遗留缺口

1. **多 Provider 持久化**：当前编排器 `LLMClient` 为单配置；上轮网关侧 `MOX_LLM_PROVIDERS` 多 provider 与编排器单配置尚未统一（两套配置源）。
2. **并发写**：落盘为同步文件写，在 async handler 内执行；高并发更新时为短临界区，暂未引入文件锁。
3. **文件加密**：`.runtime/llm-config.json` 明文存 api_key；如需合规（静态加密）需后续接入密钥保管。
4. **配置导出/导入**：管理面尚无"备份/迁移"端点。
