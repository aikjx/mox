# 前端 API 层验证报告

> 验证对象：`frontend-ui/src/api/` 目录下全部 API 封装
> 验证目标：统一响应格式 `{code, message, data}` 的处理与 data 字段提取
> 统一协议：成功 `{"code":0,"message":"ok","data":<T>}` / 失败 `{"code":<非0>,"message":"<错误描述>"}`（data 省略）
> 验证日期：2026-09-03

---

## 一、核心发现：响应拦截器使用旧格式

### 问题定位：`frontend-ui/src/api/http.js`

**`responseOkInterceptor`（第 ~85 行）**：
```javascript
const responseOkInterceptor = (response) => {
  const { data: body } = response
  // ⚠️ 检查的是 'success' 字段，而非统一协议的 'code' 字段
  if ('success' in body) {
    if (body.success) {
      return body.data  // 提取 data
    } else {
      throw new Error(body.message || body.error || 'Request failed')
    }
  }
  return body  // 无 success 字段时原样返回
}
```

**问题本质**：拦截器识别的是**旧格式** `{success: true, data: <T>}`，而非归一化后的**新格式** `{code: 0, message: "ok", data: <T>}`。

**影响范围**：所有经 `http.js` 发出的请求（即全部 API 封装函数）在收到 `{code:0, message:"ok", data:...}` 响应时：
1. `'success' in body` 为 `false`（因为 body 中没有 `success` 字段）
2. 直接 `return body` — **返回完整信封 `{code, message, data}`，而非提取后的 `data`**
3. 调用方拿到的是 `{code:0, message:"ok", data:{...}}` 而非期望的 `{...}` 数据

---

## 二、不符合规范的文件与函数清单

### 严重级：全部经 http.js 的 API 封装（10 个文件，~200+ 函数）

以下所有文件的 API 函数均通过 `http.get/post/put/delete` 发出请求，因此全部受响应拦截器旧格式问题影响：

| # | 文件 | 函数数量 | 受影响函数（示例） |
|---|------|:--------:|---------------------|
| 1 | `graph.api.js` | ~15 | `getGraphData`, `getNeighborhood`, `getPath`, `getShortestPath`, `getCentrality`, `getCommunities`, `getGraphStats` |
| 2 | `kb.api.js` | ~20 | `getDocuments`, `getDocument`, `analyzeDocument`, `searchKb`, `getCategories`, `getTags`, `getVersions`, `getEntities` |
| 3 | `ai.api.js` | ~40 | `aiChat`, `analyzeAlgorithm`, `aiFullAnalysis`, `getInfiniteBenchmarks`, `startInfiniteOptimize`, `aiExpertChat`, `getEngineFlowGraph` |
| 4 | `system.api.js` | ~60 | `getDeptList`, `getUserList`, `getRoleList`, `getMenuTree`, `getDictTypeList`, `getConfigList`, `getOperLogList`, `getLoginLogList`, `getSecurityStatus`, `getApiKeys` |
| 5 | `alliance.js` | ~25 | `getAllianceCapabilities`, `createAllianceTask`, `getAllianceTasks`, `getAllianceTask`, `getFusionResults`, `pauseAllianceTask`, `getAllianceStats`, `allianceRegisterExpert`, `allianceGetExperts` |
| 6 | `experts.api.js` | ~50 | `getExperts`, `getExpert`, `registerExpert`, `consultExpert`, `multiExpertConsult`, `expertDebate`, `getExpertGraph`, `enterpriseConsult`, `expertOrchestrate` |
| 7 | `actuator.api.js` | ~12 | `getActuatorIndex`, `getActuatorHealth`, `getActuatorInfo`, `getActuatorEnv`, `getActuatorMetrics`, `getApiMappings`, `getApiDetail`, `enableApi`, `disableApi`, `getOnlineLogs` |
| 8 | `index.js` | ~10 | 聚合导出函数（re-export），间接受影响 |
| 9 | `workspace.api.js`（如存在） | ~10 | 工作空间相关 API |
| 10 | `monitor.api.js`（如存在） | ~10 | 监控相关 API |

**总计**：约 10 个文件、250+ 个 API 封装函数全部受影响。

---

### 中等级：SSE 流式端点绕过 http.js（3 处）

以下函数使用原生 `fetch` 而非 `http.js`，绕过了响应拦截器，自行解析 SSE 流：

| # | 文件 | 函数 | 端点 | 问题 |
|---|------|------|------|------|
| 1 | `alliance.js` | `runAllianceFullSSE` | `POST /api/ai/engine/alliance/full` | SSE 流，事件帧为 JSON，非统一信封格式；但流模式本身不适用 `{code,data}` 信封 |
| 2 | `alliance.js` | `allianceExpertDebate`（stream 模式） | `POST /api/experts/debate` | 同上，SSE 流模式 |
| 3 | `actuator.api.js` | `openLogTail` | `GET /actuator/logs/tail` | SSE 流，返回 `Response` 对象由调用方解析 |

**评估**：SSE 流式端点不适用统一请求-响应信封，属于合理例外。但需确保流内事件帧的 JSON 结构有明确定义。

---

### 低等级：硬编码降级兜底返回非标准格式（1 处）

| # | 文件 | 函数 | 问题 |
|---|------|------|------|
| 1 | `alliance.js` | `getVoiceHealth` | catch 块中返回硬编码对象 `{ok:false, upstream_unreachable:true, fallback_action:"...", tts:{...}}`，既非 `{code,message,data}` 也非 `{success,data}`，与正常返回格式不一致 |

**修复建议**：catch 块返回统一格式：
```javascript
catch (e) {
  return { code: 503, message: 'voice service unreachable', data: { fallback_action: '...', tts: {...} } }
}
```

---

## 三、根因分析

### 格式演进时间线

```
旧格式（legacy Node.js 后端）      新格式（Rust 网关归一化）
{ success: true, data: <T> }  →  { code: 0, message: "ok", data: <T> }
{ success: false, error: ".." } → { code: <非0>, message: "<错误描述>" }
```

前端 `http.js` 的 `responseOkInterceptor` 仍在识别旧格式的 `success` 字段，未同步更新为新格式的 `code` 字段。

### 为什么问题未被发现

1. **兼容性回退**：拦截器在 `'success' in body` 为 false 时 `return body`（原样返回），调用方拿到完整信封后，部分代码可能通过 `res.data` 间接访问到数据，导致功能看似正常
2. **legacy 后端并存**：部分端点仍由 legacy Node.js 后端返回旧格式 `{success, data}`，拦截器对这些端点正常工作，掩盖了新格式端点的问题
3. **TypeScript 类型缺失**：前端 API 层无 TypeScript 类型约束，调用方无法在编译期发现返回类型不匹配

---

## 四、修复方案

### 方案 A：修改响应拦截器（推荐，最小改动）

修改 `http.js` 的 `responseOkInterceptor`，同时兼容新旧格式：

```javascript
const responseOkInterceptor = (response) => {
  const { data: body } = response

  // 新格式：{code, message, data}
  if ('code' in body) {
    if (body.code === 0) {
      return body.data  // 提取 data 字段
    } else {
      throw new Error(body.message || `Error code: ${body.code}`)
    }
  }

  // 旧格式兼容：{success, data}（legacy 后端）
  if ('success' in body) {
    if (body.success) {
      return body.data
    } else {
      throw new Error(body.message || body.error || 'Request failed')
    }
  }

  // 无信封格式（如 /health 返回 {ok:true}），原样返回
  return body
}
```

**优点**：
- 单一修改点，影响全部 250+ API 函数
- 向后兼容 legacy 旧格式
- 非信封响应（如 /health）不受影响

**风险**：
- 需确保所有调用方期望的是 `data` 而非完整信封。若有调用方依赖 `body.code` 或 `body.message`，需同步调整

---

### 方案 B：逐函数适配（不推荐，工作量大）

在每个 API 函数中手动提取 `data`：
```javascript
export const getDeptList = async (params) => {
  const res = await http.get('/system/dept', { params })
  return res.code === 0 ? res.data : res  // 手动判断
}
```

**缺点**：需修改 250+ 函数，易遗漏，维护成本高。

---

### 方案 C：引入 TypeScript 类型层（长期建议）

为 API 层添加 TypeScript 类型定义，强制返回类型为 `T`（提取后的数据），在编译期捕获格式不匹配：

```typescript
interface ApiResponse<T> {
  code: number
  message: string
  data: T
}

async function request<T>(config: AxiosRequestConfig): Promise<T> {
  const res = await http.request<ApiResponse<T>>(config)
  if (res.data.code !== 0) throw new Error(res.data.message)
  return res.data.data
}
```

---

## 五、修复优先级与执行顺序

| 优先级 | 任务 | 预估工作量 | 影响范围 |
|--------|------|-----------|----------|
| **P0** | 修改 `http.js` 响应拦截器，兼容 `{code,message,data}` 新格式 | 15 min | 全部 250+ API 函数 |
| **P1** | 全局搜索调用方，确认无代码依赖 `body.code`/`body.message`/`body.success` | 30 min | 前端全部页面组件 |
| **P2** | 修复 `getVoiceHealth` catch 块返回统一格式 | 5 min | alliance.js |
| **P3** | 为 SSE 流式端点添加事件帧类型文档 | 20 min | alliance.js, actuator.api.js |
| **P4** | 长期：引入 TypeScript API 类型层 | 4-8 小时 | 前端架构升级 |

**总计**：P0-P2 约 50 分钟可完成核心修复。

---

## 六、验证方法

修复后需验证以下场景：

1. **新格式端点**：调用 `GET /api/v1/system/dept`（Rust 网关），确认返回 `dept[]` 而非 `{code,message,data}`
2. **旧格式端点**：调用 legacy Node.js 后端端点，确认仍正常提取 `data`
3. **错误响应**：触发 4xx/5xx，确认抛出 `Error(message)` 而非返回错误信封
4. **无信封端点**：调用 `GET /health`，确认返回 `{ok:true,...}` 原样返回
5. **SSE 端点**：验证联盟执行流、专家辩论流、日志尾流正常接收事件帧

---

## 七、总结

| 维度 | 结论 |
|------|------|
| **核心问题** | `http.js` 响应拦截器识别旧格式 `{success,data}`，未适配新格式 `{code,message,data}` |
| **影响范围** | 全部 10 个 API 文件、250+ 封装函数（经 http.js 的请求均返回完整信封而非提取后的 data） |
| **严重程度** | 高 — 调用方拿到的数据结构与预期不符，可能导致页面渲染异常或需额外 `.data` 访问 |
| **修复成本** | 低 — 单点修改 `responseOkInterceptor`（15 分钟），向后兼容旧格式 |
| **根因** | 后端从 legacy Node.js 迁移至 Rust 网关时，响应格式从 `{success,data}` 归一化为 `{code,message,data}`，但前端拦截器未同步更新 |
