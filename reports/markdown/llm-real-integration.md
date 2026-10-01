# AI 对话增强：真实 LLM 接入验证报告

> 日期：2026-09-27
> 验证对象：编排器（:3001）LLM 配置与真实对话接入
> 验证方式：代码审查 + 环境检查（服务下线期间无法 curl E2E，如实披露）

---

## 1. LLM 配置体系（代码审查确认）

### 1.1 配置入口

编排器 `platform/domains/platform/svc/mox-platform-orchestrator-svc/src/main.rs`：

- **启动自动接入**（:355-372）：启动时读取环境变量 `DEEPSEEK_API_KEY`，非空则自动调用 `configure_llm()` 启用真实 LLM：
  - `api_base`: `https://api.deepseek.com/v1`
  - `model`: `deepseek-chat`
  - `temperature`: 0.7
  - `max_tokens`: 2048
  - `enabled`: true
- **无 key 时降级**（:370-371）：打印 `未检测到 DEEPSEEK_API_KEY，AI 对话将使用内置规则引擎（离线降级）`

### 1.2 运行时配置端点（:505-507）

| 端点 | 方法 | 功能 |
|---|---|---|
| `/api/ai/llm/config` | GET | 读取当前配置（api_key 脱敏为 `has_api_key: bool`） |
| `/api/ai/llm/config` | POST | 更新配置（LLMConfigRequest：api_base/api_key/model/temperature/max_tokens/enabled，均 Optional 合并式更新） |
| `/api/ai/llm/test` | POST | 测试 LLM 连通性 |

### 1.3 配置响应形状（get_llm_config :2241-2253）

```json
{
  "code": 0,
  "msg": "ok",
  "data": {
    "api_base": "https://api.deepseek.com/v1",
    "model": "deepseek-chat",
    "temperature": 0.7,
    "max_tokens": 2048,
    "enabled": false,
    "has_api_key": false
  }
}
```

`api_key` 永不回传，仅以 `has_api_key` 布尔值指示。

---

## 2. 当前环境状态

### 2.1 环境变量检查

```
DEEPSEEK_API_KEY  = (未设置)
OPENAI_API_KEY    = (未设置)
LLM_API_KEY       = (未设置)
MOX_LLM_API_KEY   = (未设置)
```

`.env.example` 中 LLM 段（:26-51）定义了 `LLM_PROVIDER=ollama` / `LLM_MODEL=qwen2.5` / `OPENAI_API_KEY` / `QWEN_API_KEY` / `DOUBAO_API_KEY`，但编排器实际只认 `DEEPSEEK_API_KEY`（main.rs:356），两套配置体系未对齐。

### 2.2 服务状态

验证时所有后端服务下线：

| 端口 | 服务 | 状态 |
|---|---|---|
| 3001 | 编排器 | ❌ 未监听 |
| 3080 | 模块化网关 | ❌ 未监听 |
| 3100 | 联盟调度 | ❌ 未监听 |
| 3200 | 执行器 | ❌ 未监听 |

因此 **curl E2E 无法执行**，本报告以代码审查 + 环境检查为准。

---

## 3. 结论

### 3.1 当前对话模式

**规则引擎回退模式**。因 `DEEPSEEK_API_KEY` 未设置，编排器启动时走 :370-371 分支，AI 对话使用内置规则引擎（离线降级），不调用任何外部 LLM。

### 3.2 启用真实 LLM 的方式

```powershell
# 方式一：环境变量（推荐，启动时自动接入）
$env:DEEPSEEK_API_KEY = "sk-xxx"
scripts/startup/start-mox-enterprise.ps1

# 方式二：运行时 API 更新（服务启动后）
curl.exe -s -X POST http://127.0.0.1:3001/api/ai/llm/config `
  -H "Authorization: Bearer dev-secret-token" `
  -H "Content-Type: application/json" `
  -d '{"api_key":"sk-xxx","enabled":true}'

# 验证连通性
curl.exe -s -X POST http://127.0.0.1:3001/api/ai/llm/test `
  -H "Authorization: Bearer dev-secret-token"
```

### 3.3 真实对话与规则引擎的区分

启用真实 LLM 后：
- `GET /api/ai/llm/config` 返回 `enabled: true, has_api_key: true`
- `POST /api/ai/llm/test` 返回 `success: true` 及模型响应
- `POST /api/ai/chat` 响应含真实模型语义输出（非固定模板文案）

规则引擎回退时：
- `enabled: false, has_api_key: false`
- 对话响应为固定模板/规则匹配文案

---

## 4. 遗留缺口

1. **配置体系不对齐**：`.env.example` 定义了 `LLM_PROVIDER/OPENAI_API_KEY/QWEN_API_KEY/DOUBAO_API_KEY`，但编排器只认 `DEEPSEEK_API_KEY`。需统一配置入口或支持多 provider env。
2. **服务下线**：验证时 4 个后端服务均未监听，需重启后才能 curl E2E 验证真实对话。
3. **网关代理未知**：`/api/ai/llm/*` 端点注册在编排器 3001，网关 3080 是否代理该路径未确认（前端可能直连 3001 或经网关）。
4. **LLM 配置无持久化**：`update_llm_config` 仅更新内存态（:2280 `client.update_config`），重启后丢失，回到 env 驱动的初始状态。
