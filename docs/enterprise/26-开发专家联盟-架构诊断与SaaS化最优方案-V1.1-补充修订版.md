# 璇玑·开发专家联盟 — 架构诊断与 SaaS AI 平台化最优方案 V1.1（补充修订版）

> **文档性质**：V1.0 的补充与修订版。基于**第二轮源码深挖**（RBAC policy.rs / AI 编排完整 5 步流水线 / security.js 配额框架 / plugins.json 现状 / service-manager 双源冲突），**修正了 V1.0 对 5 大模块的低估**，并补充：落地执行矩阵（逐文件逐改动点）、风险缓解策略（16 项风险 × 概率 × 影响 × 缓解措施）、ROI 测算表（四阶段投入 vs SaaS 化 MRR 回收）、30 天快速里程碑（D1-D30 按天拆）。
>
> **关键修订结论**：由于 RBAC/AI 编排/配额限流三大核心模块**已经写完且完全符合硬约束**，整体 SaaS 化工作量较 V1.0 **减少约 40%**，首版 SaaS MVP 上线周期从 16 周压缩到 9-10 周。

---

## 十一、V1.0 遗漏的 5 大已实现模块（重大利好，工作量砍 40%）

第二轮源码深挖证实：原 V1.0 以为需要从零写的 5 个 SaaS 核心模块，**代码已存在且质量达标**，只需和 tenant_id 绑定即可复用：

### 11.1 ✅ RBAC 角色体系已完整（mox-expert / rbac/policy.rs）
V1.0 原判断：「RBAC 仅在 mox-expert crate 中有审计+S3 模块，无租户级 RLS」——**只对了一半**。

真实状态：`platform/services/mox-expert/src/rbac/policy.rs` 已实现：

| 能力 | 状态 | 代码位置 |
|------|------|---------|
| 6 内置角色 | ✅ 完成 | [policy.rs#L133-L158](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/mox-expert/src/rbac/policy.rs#L133-L158)：admin→editor→viewer 继承链 + safety_approver(仅审批生产) + operator(运维) + auditor(审计只读) |
| 资源级权限（通配符） | ✅ 完成 | [Permission.matches()](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/mox-expert/src/rbac/policy.rs#L33-L48)：`write:db:prod/*` 匹配 `db:prod/citizen_info`，支持 `/*` 尾缀通配 + `*` 全匹配 |
| 继承链展开（防循环） | ✅ 完成 | [BuiltinRoles::resolve_impl()](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/mox-expert/src/rbac/policy.rs#L93-L119)：visited HashSet 防循环继承 DAG |
| 失败自动审计 | ✅ 完成 | [rbac/mod.rs#L12](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/mox-expert/src/rbac/mod.rs#L12)：`check_with_audit` 失败自动打审计 |
| 全局单例策略 | ✅ 完成 | `pub static POLICY: LazyLock<RwLock<RbacPolicy>>` 一次初始化 |

**SaaS 化补量：只需 3 处改动**
- 给 Permission 结构加 `tenant_id` 可选字段（资源跨租户不可见）
- Default 策略里，viewer/editor/admin 三套角色**每租户自动实例化一份**
- `check()` 增加 tenant_scope 参数（默认当前请求租户，跨租户查询需 `cross_tenant:read` 超级权限）

**工作量重估：V1.0 预估 4 人日 → 实际 1 人日（80% 已完成）**

---

### 11.2 ✅ AI 编排层 4 统一入口 + 5 步流水线 100% 对齐 project_memory 硬约束
V1.0 原判断：「AI 编排层是否独立不透明，需确认计量中间件」——实际**完全按硬约束实现完了**！

`platform/backend-node/src/ai-engine-core.js` 对照 project_memory 4 项硬约束：

| project_memory 硬约束 | 代码实现位置 | 状态 |
|----------------------|-------------|------|
| `POST /ai/engine/process`（自动意图识别→能力路由） | [AIEngineCore.process()](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine-core.js#L191) → `detectIntentByGraph()` → `_dispatch()` | ✅ |
| `POST /ai/engine/analyze`（显式能力执行） | [executeCapability()](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine-core.js#L221)：校验 capability ∈ 能力矩阵 | ✅ |
| `GET /ai/engine/capabilities`（能力矩阵自描述） | [getCapabilities()](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine-core.js#L162)：返回 6 能力 + 关键词 + pipeline + 4 不变式 | ✅ |
| `GET /ai/engine/metrics`（成功率/降级率/延迟） | `_recordMetric()` + `engine_core_metrics.json` 持久化（[ai-engine-core.js#L125-L140](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine-core.js#L125-L140)），接口路由已在 `routes/ai-engine.js` | ✅ |

5 步流水线 + 4 不变式 + 6 能力矩阵全部落地：
```
process() 入口
  ↓ ① 意图识别（图谱激活扩散 detectIntentBySpread；显式 capability 覆盖则跳过）
  ↓ ② 能力路由（CAPABILITY_META 6 类：expert/reasoning/memory/graph/workflow/chat）
  ↓ ③ 引擎执行（委托 allianceEngine/ultimateEngine/aiEngine/gateway，不重造）
  ↓ ④ 质量校验（非空判定；失败 → 不变式②降级 chat 兜底，绝不空手）
  ↓ ⑤ 指标反馈（_recordMetric 每次必记，成功失败都记）
```

**SaaS 化补量：只需 2 处改动**
- `_recordMetric(record)` 增加 3 个字段：`tenant_id / org_id / user_id`（从 req 上下文注入），记录到 `llm_usage.jsonl`
- `_dispatch()` 前调一次 SecurityManager 的 `tryAcquireTenantBucket()`（已存在！见下一节），超配额抛 `QuotaExceeded` → 前端提示升级套餐

**工作量重估：V1.0 预估 8 人日 → 实际 1 人日（90% 已完成）**

---

### 11.3 ✅ 安全层已内置多租户配额 + TokenBucket 限流（和 4 档套餐 1:1 对齐）
V1.0 原判断：「零计量 + 零成本归因，计量系统 MVP 从零做」——**大错**。

`platform/backend-node/src/security.js` 已经实现：

| SaaS 化所需能力 | 真实代码现状 | 与 V1.0 阶段四套餐对齐 |
|---------------|-------------|---------------------|
| 4 档租户配额 | `DEFAULT_TENANT_QUOTAS`：VIP(qps200/burst400) / NORMAL(qps20/burst60) / TRIAL(qps5/burst10) / ANONYMOUS(qps2/burst4) | 🔴 完美对应 V1.0 的 Enterprise/Team/Pro/Free 4 档，**命名只差 TRIAL→PRO 别名** |
| 双维度限流 | `_tokenBuckets`（per-api-key）+ `_tenantBuckets`（per-tenant）两个 TokenBucket Map | SaaS 化标准做法（key 限流防单用户刷爆，tenant 限流防整组织超配） |
| 闲置 Bucket 自动 GC | `bucketIdleCleanupMs: 10min` + 每 60s 清理一次 | 内存无限增长风险已排除 |
| API Key 生命周期管理 | `_loadApiKeys()` / `_saveApiKeys()`：key 存 api_keys.json + createdAt + lastUsed + expiry(24h 默认) | 开发者 API 凭证系统底子已在 |
| 环形审计日志 | `auditLogMaxEntries: 10000`：内存环形 + `audit_log.json` 持久化 | 合规审计已有 |
| 输入清洗 | `_setupSanitizers()` Map：可注册 per-route 输入过滤器 | XSS/注入防线已留钩子 |

**SaaS 化补量：只需 3 处改动**
1. api_keys.json 每条 key 加 `tenant_id` 字段（创建 API Key 时绑定租户）
2. SecurityManager 增加 `recordLlmUsage(tenant_id, token_in, token_out, model)` 方法，落到 `llm_usage.jsonl`（append-only JSONL，每 100 条或 10s fsync 一次，避免 IO 抖动）
3. `routes/ai-engine.js` 统一走 `security.checkQuotaAndRecord()` 中间件（一次调用同时完成限流 + 配额 + 计量三件事）

**工作量重估：V1.0 预估 10 人日 → 实际 2 人日（85% 已完成）**

---

### 11.4 ✅ 激活扩散意图识别钩子已留（JS 侧 detectIntentBySpread 委托流程图谱引擎）
V1.0 原判断：「专家匹配算法未见实现，需从零写激活扩散放 Rust mox-intent-core」——**JS 侧钩子已留好，Rust 侧写 spread_activation 即可，FFI 绑定框架已有**。

现状代码链：
```
AIEC.process()
  → detectIntentByGraph(question)
     → try { this.flowGraph.detectIntentBySpread(question) }  // 图谱激活扩散（主路径）
     → catch { 降级关键词打分兜底 }  // 不变式②延伸
```

`project_memory` 硬约束的 spread 参数（method=spread, d=0.85, 30 轮收敛）写进 `platform/crates/mox-intent-core/src/lib.rs` 的 `spread_activation()` 即可，Node 侧通过 `mox-norm-intent-native` napi 绑定调用（绑定 crate 已在 Cargo.toml workspace 注册，不需要新建 napi 脚手架）。

**工作量重估：V1.0 预估 5 人日（Rust 写算法 + FFI + JS 接线）→ 实际 3 人日（JS 钩子已好 + FFI 脚手架已好 → 只写 Rust 算法 + 写 1 个 napi 导出函数 + 写单测）**

---

### 11.5 ✅ 插件体系已有 3 个内置插件（缺 Manifest 标准 + 权限校验）
`plugins.json` 现状：3 条记录
| id | 名称 | 类型 | 已实现的元信息 | 缺失 |
|----|------|------|-------------|------|
| pl_mcp | MCP 兼容层 | protocol | endpoints:5 | version/author/permissions/resources |
| pl_browser | 浏览器自动化 | automation | sessions:0 | 同上 |
| pl_flow | 流程图引擎 | ir | node_types:8 | 同上 |

V1.0 的 Manifest YAML 设计是对的，但**不需要新造插件发现和加载的目录框架**——直接在 `plugins.json` 每条记录**加 manifest 嵌套对象**即可（过渡期兼容旧数据：无 manifest 字段的旧插件默认为 `type=core` 平台内置，权限全开且不可卸载）。

**工作量重估：V1.0 预估 6 人日 → 实际 3 人日（插件存储和加载代码已在 routes/plugins.js，只补 Manifest 校验 + 权限 RPC 拦截）**

---

## 十二、新增 P2-5：服务定义双源冲突（单源真相破坏）

第二轮深挖发现 V1.0 漏掉的工程问题：**`platform_config.json` 和 `service-manager.js` SERVICE_DEFINITIONS 硬编码 双源冲突，内容不一致**。

| 字段 | platform_config.json（3 服务） | service-manager.js SERVICE_DEFINITIONS（2 服务） |
|------|-------------------------------|----------------------------------------------|
| 服务数 | 3（api / frontend / xiaobai_voice） | 2（仅 api / frontend） |
| api 端口 | 3010 | 3010（一致） |
| frontend 端口 | **3020** | **3000（冲突）** |
| frontend command | `npm run dev` (vite) | `node src/server.js`（静态服务，文件不存在） |
| xiaobai_voice | 有（端口 3717，python -m xiaobai_voice serve，auto_start=true） | 完全缺失 |
| startup 顺序 | api(10) → xiaobai(5? 实际上 xiaobai order_hint=5 < api=10，xiaobai 先启) | api(1) → frontend(2)，无 xiaobai |
| health_check path | 每个服务独立定义 | 有，路径一致 |
| 工作目录 | 用 `cwd` 字符串（相对 repo 根） | 用 `path.join(__dirname, '..')` 绝对路径 |

**风险**：
- 新成员按 config.json 说前端跑 3020，service-manager 启在 3000 → 调试半天连不上
- xiaobai_voice 语音服务完全不被 service-manager 管 → `.\scripts\start-all.ps1` 和看门狗 watchdog 不启它 → AI 对话无 TTS/ASR，用户以为坏了
- 以后改端口只改一处 → 另一处没改 → 端口冲突/连不上

**修复方案（1 人日）**：
1. `service-manager.js` **删除 SERVICE_DEFINITIONS 硬编码**，改为 `require('../../platform_config.json').services` 读 JSON（单源真相原则）
2. 代码里做兼容转换：把 platform_config.json 的字段（`command` + `args` 字符串数组 / `cwd` 相对路径 / `port` / `health_check`）**自动映射**到 service-manager 内部需要的字段（workingDir 用 `path.resolve(repoRoot, cwd)`，pidFile 统一放 `.runtime/<id>.pid`）
3. 写单测：启动 service-manager → 调 `list()` → 应返回 3 个服务（含 xiaobai_voice）且端口和 config 完全一致

---

## 十三、落地执行矩阵（逐任务 × 文件 × 改动点 × 验证 × 回滚 × 耗时）

> 基于 V1.1 修订的工作量重估，四阶段总工期 9-10 周，比 V1.0 少 6 周。按优先级排序。

### 阶段一（0.5-1 周 · P1 全清）— 共 7 项任务

| # | 任务名 | 需改文件（绝对路径锚点） | 具体改动点摘要 | 验证方法 | 回滚方案 | 耗时 | 负责人 |
|---|--------|----------------------|-------------|---------|---------|------|--------|
| 1.1 | 清理 47 个根目录垃圾文件 | 根目录 | `Remove-Item *.log / NUL-*.d / *.rmeta / green_log.txt 等` | `git status --short` 只显示预期的已跟踪文件变更；未跟踪文件数=0 | 有回收站的从回收站还原；或 `git checkout .` 不影响（垃圾文件本来就是未跟踪） | 1h | DevOps |
| 1.2 | 检查 graph.json 入仓状态+必要时清历史 | 根目录 + `.gitignore` | ① `git ls-files \| grep graph` → 如有输出则 `git rm --cached` + 可选 `git filter-repo` ② 补 `.gitignore` 漏的规则见 V1.0 1-1 | `git ls-files` 无 graph*.json；`.gitignore` 规则 `git check-ignore graph.json` 输出命中 | filter-repo 前自动做 `git clone --mirror` 冷备份，出问题整仓回退 | 2-4h（取决是否改历史） | DevOps |
| 1.3 | 三目录重命名（projects→showcase-projects 等） | 多处（见 V1.0 1-3 清单） | ① `git mv projects showcase-projects` ② `git mv workspace ai-outputs` ③ `Rename-Item my_projects local-dev` ④ `grep -rl projects/ 各源码 → platform_config.json / Cargo.toml / .github/ workflows/*.yml` → 批量改路径 | `rg "projects/(?!showcase)"` 无残留；`.\scripts\check-all.ps1` 全绿 | `git mv` 反过来再改回去，路径替换有 git 历史可追溯 | 1d + 半天联调 | 全栈 1 人 |
| 1.4 | 强化 .gitignore 50+ 条运行时数据规则 | `.gitignore` | 追加 V1.0 1-1 + 1-3 的全部规则 | 改完后 `git status --ignored` 检查 `platform/backend-node/data/` 下种子数据（experts.json/projects.json 等）仍能显示为跟踪状态，运行时文件（tasks.json/audit_log.json/ous.db*）显示为 ignored | 直接从 git 历史恢复 .gitignore 旧版本 | 2h | DevOps |
| 1.5 | **修复 P2-5 服务定义双源** | [service-manager.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/service-manager.js) + [platform_config.json](file:///d:/a10/aikjx/gitcode/infotopograph/platform_config.json) | ① 删 `SERVICE_DEFINITIONS` 常量 ② 加 `loadServicesFromConfig()` 读 platform_config.json + 字段映射（cwd→workingDir，port/healthCheck 直接用）③ frontend command 统一：若 `command` 是 npm run dev 用 shell 执行 | 单测：`new ServiceManager().services.size === 3` → 含 xiaobai_voice；端口 api=3010/frontend=3020/xiaobai=3717 全对 | 若 platform_config.json 解析失败，回退到嵌入的 `FALLBACK_SERVICE_DEFS`（包含正确的 3 服务定义） | 1d | Node 后端 1 人 |
| 1.6 | 写 scripts 统一脚本 4 件套 | `scripts/setup-dev.ps1` + `scripts/start-all.ps1` + `scripts/stop-all.ps1` + `scripts/check-all.ps1` | start-all 复用修复后的 service-manager 单源读 config；check-all 顺序：cargo check --workspace → platform/backend-node npm test → frontend-ui vitest run → projects/xiaobai_voice pytest -x | 新人 `git clone` → `cd scripts ; .\setup-dev.ps1` → 全环境无报错；`.\start-all.ps1` 后 curl 三个 health 全部 200；`.\check-all.ps1` 退出码=0 | 删除 scripts 目录即可 | 1.5d | DevOps |
| 1.7 | sccache 冷编译缓存 + CI workflow 优化 | `.github/workflows/enterprise-ci.yml` + Cargo.toml 可选 | ① CI 加 actions/cache@v4 缓存 `~/.cargo/registry` + `target/`（key = `${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}`）② 本地文档提示装 sccache 加速冷编译 | `cargo build --workspace` 二次运行 5 分钟内（冷 15-30 分）；GitHub Actions 全量 CI 从 45min 降到 <20min | 删 cache 配置回原 CI 速度 | 0.5d | DevOps |

**阶段一合计**：约 4-5 人日，1 个 DevOps + 1 个全栈并行 1 周可完成。

---

### 阶段二（2-3 周 · P0 全清）— 共 6 项任务

| # | 任务名 | 需改文件锚点 | 改动点摘要 | 验证方法 | 回滚 | 耗时 | 负责人 |
|---|--------|-----------|-----------|---------|------|------|--------|
| 2.1 | 四层身份模型写入 types.js + 后端 domain | [types.js](file:///d:/a10/aikjx/gitcode/infotopograph/frontend-ui/src/types.js) 追加 `TENANT_MODEL` 常量；后端 `platform/backend-node/src/tenant/` 新建（tenant/org/user 三个模型文件） | ① 写 TENANT_MODEL（Tenant→Org→User→Project 四层 + 每级外键）② 迁移脚本 migrate-tenant.js 给所有旧数据补 tenant_id='default'/org_id='default' ③ json-store.js 加 `withTenant(id)` 过滤器 | ① 旧 projects.json 每条记录 `.tenant_id === 'default'`（迁移脚本单测断言）② 新 API 无 X-Tenant-Id header → 返回 401（Postman 验证） | `withTenant` 加开关 `TENANT_ISOLATION_ENABLED`，环境变量设 0 时全关，行为回到单租户 | 1.5d | 架构师 + 后端 1 人 |
| 2.2 | 修复 P0-3：admin 密码改环境变量 scrypt 哈希 | [platform_config.json](file:///d:/a10/aikjx/gitcode/infotopograph/platform_config.json) + service-manager.js 启动前校验 | ① platform_config.json admin 段改为 `${MOX_ADMIN_USER}` / `${MOX_ADMIN_PASSWORD_HASH}` ② 启动前 `envReplace()` 做变量替换 + `crypto.scryptSync` 校验密码哈希格式（64 字节 hex = 128 字符长）③ 未设置时**进程 throw Error 退出**，打印生成密码哈希的命令 | ① 空环境启动 → stderr 输出"请设置 MOX_ADMIN_PASSWORD_HASH…"+退出码=1 ② 正确设置后 curl `/login` 用明文密码登录成功 | 若出问题，临时改代码允许 `admin123`（仅紧急，热修复后立即下线） | 0.5d | 后端 1 人 |
| 2.3 | ProjectPicker 升级为「租户/项目」二级选择器 + projectContext 加 tenant 状态 | [ProjectPicker.vue](file:///d:/a10/aikjx/gitcode/infotopograph/frontend-ui/src/components/ProjectPicker.vue) + [projectContext.js](file:///d:/a10/aikjx/gitcode/infotopograph/frontend-ui/src/composables/projectContext.js) + HTTP 拦截器 | ① picker 顶部加 `tenant-list` el-select（多租户时显示）② projectContext 加 `currentTenant / tenantList / setCurrentTenant()` ③ localStorage 存 `mox.currentTenant.v1` ④ axios 拦截器注入 `X-Tenant-Id` + `X-Project-Id` 两个 header | 创建两个租户 A/B，各自建项目 → A 的 ProjectPicker 列表看不到 B 的项目；浏览器 DevTools Network 看请求 header 带两个 X-* | 加 `useV1PickerOnly=true` 隐藏 tenant 选择器；HTTP 拦截器若 `currentTenant.value==null` 时不注入 X-Tenant-Id（向后兼容） | 2d | 前端 1 人 |
| 2.4 | 23 路由域强制 tenant 校验中间件 | `platform/backend-node/src/routes/*.js`（projects/chat/tasks/graph 等 23 文件）+ 新建 `middleware/tenant-guard.js` | ① 写 `tenantGuard(req, res, next)`：从 JWT.payload.tenant_id 取 + 对比 header.X-Tenant-Id（不一致=403 越权）② 每路由 `router.use(tenantGuard)` ③ `readJSON/writeJSON` 经过 `withTenant(tenant_id)` 过滤 | Postman：① 用租户 A 的 JWT + 租户 B 的 X-Tenant-Id → 403 ② 两 ID 一致 → 返回 A 的数据 ③ 无 JWT → 401 | 中间件内部加 `BYPASS_TENANT_GUARD_FOR_DEV=1` 环境变量开关（本地开发免鉴权） | 2d | 后端 1 人 |
| 2.5 | AI 计量接入 SecurityManager（利用已写好的双 TokenBucket） | [security.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/security.js) + [ai-engine-core.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine-core.js) `_recordMetric` + `routes/ai-engine.js` | ① SecurityManager 加 `recordLlmUsage({tenant, model, in_tokens, out_tokens})` → JSONL 落盘 `llm_usage.jsonl`（100 条批量 fsync）② `_dispatch` 前后 hook：调用前 `tryAcquireTenantBucket()`，拿到 provider 响应后调 `recordLlmUsage()` 解析 token 数（OpenAI 兼容的 usage 字段）③ 两个 GET 接口：`/metering/usage?range=this_month` 和 `/metering/cost?breakdown=tenant` | 连续发 10 次 AI 请求 → `wc -l llm_usage.jsonl` = 10；每条记录包含 6 字段（tenant_id/user_id/project_id/model/tokens_in/tokens_out）；TRIAL 档发第 6 次（burst=5）→ 返回 429 QuotaExceeded | 开关 `DISABLE_METERING=1` 时跳过计量（省 IO，本地开发用） | 2d | 后端 1 人 |
| 2.6 | 专家联盟智能匹配激活扩散算法（Rust + napi 绑定） | `platform/crates/mox-intent-core/src/lib.rs` 写 `spread_activation()` + `platform/crates/bindings/mox-norm-intent-native` 加 1 个 napi 导出 + `ai-engine-core.js detectIntentBySpread` 接线 | ① Rust：按硬约束 `d=0.85 / max_iter=30 / σ̄<0.06 停止` 实现 ② napi：`#[napi] pub fn spread_activation(graph_json: String, seed: Vec<String>) -> napi::Result<Array>`（传图 JSON + 关键词 seed，返回排序后的 (expert_id, score) 数组）③ JS 侧 `allianceEngine.matchExperts(intent)` 直接调 napi 函数 | 单测：输入 50 节点的专家图 + seed 3 个关键词 → 30 轮内收敛（σ̄<0.06）；输出第一名专家 score > 0.6；性能 1000 节点图 < 200ms | `smartMode` 开关关时 → 完全走旧关键词过滤，不碰 Rust 算法 | 3d | Rust 1 人 + Node 接线 0.5 人 |

**阶段二合计**：约 11.5 人日。后端 2 人 + 前端 1 人 + Rust 1 人 并行 2.5-3 周可完成。P0 三项全部清零。

---

### 阶段三（3-4 周 · 剩余 P2/P3）— 共 6 项任务
（略作精简，每项格式同阶段一/二）

| # | 任务名 | 关键文件 | 核心改动 | 验证 | 耗时 |
|---|--------|---------|---------|------|------|
| 3.1 | JSON Store → PostgreSQL 第一步：6 张计量/审计/用户表迁库 | `migrate/001_init_tenant.sql`（RLS + 建表）+ `db.js` 加 pg driver | 迁 users/tenants/orgs/llm_usage/audit_log/login_sessions 共 6 张；每张 `ALTER TABLE ... ENABLE ROW LEVEL SECURITY`；`app.current_tenant` 通过 `SET LOCAL` 每请求注入 | psql 用两个不同租户角色 `SET app.current_tenant = 'A'; SELECT count(*) FROM projects;` → 结果不同 | JSON Store 保留双写 3 天 | 4d |
| 3.2 | RBAC 和多租户绑定（给 Permission 加 tenant_id） | `rbac/policy.rs` + `rbac/check.rs` | Permission 结构加 `tenant_scope: Option<String>`；`check()` 增加参数 `current_tenant: &str`；每租户 admin/editor/viewer 角色自动创建 | 租户 A 的 editor 不能改租户 B 的 `db:test/*`（RBAC + 租户双保险） | 1d |
| 3.3 | 插件 Manifest 标准化 + 权限 RPC 拦截 | `plugins.json` 每条加 `manifest: {...}` 对象 + `routes/plugins.js` install 时校验 Manifest + worker_threads 沙箱 | Manifest 含 permissions/resources/endpoints 三字段；安装时弹 Dialog 列出权限让用户勾选；插件跨域 API 调用经权限中间件 | 装一个声明 `storage:read` 未声明 `graph:write` 的插件 → 它调 writeGraph 返回 403 | 3d |
| 3.4 | OpenTelemetry 三语接入（Node/Rust/Python）+ Grafana 面板配置 | 三语各自加 otel SDK + `deploy/docs/trace-8stages-dashboard.json` 导入 | 前端：@opentelemetry/web；Node：auto-instrumentations；Rust：tracing-opentelemetry；Python：opentelemetry-python；统一汇 Jaeger | Grafana 点任一 trace → 看到 Frontend→Node API→Rust mox-expert→Python xiaobai 全链路 span | 3d |
| 3.5 | 核心业务表逐步迁 PostgreSQL（tasks/projects/experts 等） | 每张表独立 migration + 双写 + 读比例切流 | 优先级顺序：tasks → projects → experts → kb_documents → workflows → graph*；每迁一张读比例从 0%→10%→50%→100%，双写保留 3 天回滚窗口 | 迁 tasks 后读 PostgreSQL 返回结果 = 读 JSON 返回结果（字段级 diff 单测） | 5d |
| 3.6 | 可观测性前端页面（系统监控 / 用量仪表盘 / 我的账单） | `views/MonitorView.vue` + `views/BillingView.vue` 新 | 用量页：当月 Token、API 调用、存储 GB 趋势图；账单页：消费明细、PDF 下载、升级套餐按钮；监控页：嵌入 Grafana iframe（只读模式） | 登录租户 TRIAL→用量页显示"本月已用 82%，建议升级 Pro"；仪表盘 3 张图有数据 | 1.5d |

**阶段三合计**：约 17.5 人日，3.5-4 周。

---

### 阶段四（2-3 周 · P4 + 商业化）— 共 5 项任务

| # | 任务名 | 关键文件 | 核心改动 | 验证 | 耗时 |
|---|--------|---------|---------|------|------|
| 4.1 | Docker Compose 一键部署 + 健康检查/优雅启停脚本 | `deploy/docker-compose.yml`（8 服务：postgres/redis/gateway/api/frontend/otel/jaeger/grafana） | 每服务加 healthcheck；depends_on 用 condition: service_healthy；优雅启停脚本 `deploy/gray-upgrade.ps1`（新实例健康 100% → 10%/50%/100% 切流量 → 旧实例 300s 无请求关停） | `docker compose up -d` 后 3 分钟内所有服务 healthy；浏览器打开 localhost 完成注册→S1→S5 全流程；gray-upgrade 模拟升级，零停机 | 4d |
| 4.2 | SDK 三语补齐（Node/Python/Rust 各 5 个核心 API） | `platform/sdk/nodejs/mox-sdk-cloud/index.js` 等（原都是 .gitkeep 空壳） | 各 SDK 至少实现：`createProject / listProjects / createTask / aiChat / analyzeGraph` 5 API；附 README + example 3 份；统一错误码 | Node SDK 5 行代码：`const mox = new Mox({apiKey: 'xxx'}); await mox.aiChat('建项目')` → 成功 | 3d |
| 4.3 | Webhook 系统（4 类事件 + HMAC 签名） | `routes/webhooks.js` + SecurityManager 的 SigV4（复用 `mox-standards` crate 已实现 SigV4） | 事件：`project.phase.changed / expert.matched / ai.response.generated / task.completed`；签名头 `X-Mox-Signature = HMAC-SHA256(secret, body)`；重试 3 次指数退避 | 注册 webhook → 后台改项目阶段 → 客户方 endpoint 收到 POST + 签名校验通过 | 2d |
| 4.4 | 计费集成（Stripe 测试模式 + 国内支付占位） | `routes/billing.js` + Billing UI | 4 档套餐：Free/Pro/Team/Enterprise；Stripe test mode 对接（订阅 + 发票 + 取消）；国内支付留 Provider 抽象接口；用量超套餐自动发邮件提醒 | Stripe 测试卡支付 → Pro 套餐生效 → 次月自动扣款；Quota 从 TRIAL(5qps)→NORMAL(20qps) 立即生效 | 3.5d |
| 4.5 | 模板市场 MVP（.moxt 包格式 + 上传 + 安装） | `routes/market.js` + `views/MarketView.vue` + `template-market` Rust crate（已在 workspace！） | `.moxt` zip 包规范：`manifest.yaml` + `project.json` + `phase_defs.json` + `seed_workflows/`；上传时解包校验 Manifest；安装时自动创建项目骨架 | 上传"企业信息化系统模板.moxt"→安装成功→专家联盟自动生成项目+5阶段+默认专家池+S1工作流 | 2.5d |

**阶段四合计**：约 15 人日，2.5-3 周。

**四阶段总工作量**：(4-5)+11.5+17.5+15 = **约 48-49 人日**。4 人团队（1后端/1前端/1 Rust/1 DevOps 兼架构）**9-10 周完成 SaaS MVP 全量上线**。

---

## 十四、风险缓解策略矩阵（16 项 × 概率 × 影响 × 措施）

| ID | 风险分类 | 风险描述 | 发生概率 | 影响（1-5） | 风险值（P×I） | 缓解措施（至少 2 条） |
|----|---------|---------|---------|-----------|-------------|-------------------|
| R1 | **技术** | PostgreSQL RLS 策略写错导致跨租户数据泄露 | 中（20%） | 5（致命） | 1 | ① 每类 RLS policy 写**跨租户否定用例**单测（A 的 JWT 查 B 的 id 返回空）② 部署前跑 `npm run test:rls` 必过，失败禁止发布 |
| R2 | **技术** | graph.json 改 git 历史后，协作者本地分支冲突到无法 merge | 中（40%） | 3（中） | 1.2 | ① 周五晚执行 filter-repo，通知所有人**先推完本地所有分支**，然后删仓重新 clone ② 提前做 dry-run：`git filter-repo --analyze` 输出报告给所有人确认 |
| R3 | **技术** | TokenBucket 内存泄漏（虽有 GC 但某些异常路径 bucket 不清理） | 低（5%） | 4（高） | 0.2 | ① 加每小时定时 `console.log(_tenantBuckets.size)` 指标，异常阈值告警 ② 压测 10 万租户 ID 随机切换，24h 内存增长 < 50MB |
| R4 | **技术** | Rust spread_activation 在大图（10k 节点）OOM | 低（10%） | 4 | 0.4 | ① 算法输入前加 `max_nodes=5000` 截断保护；超过返回"图谱过大，按子图分批" ② `mox-intent-core` 内存用 `#[global_allocator]` 挂 tikv-jemalloc + 单测 10k 节点 RSS < 200MB |
| R5 | **进度** | PostgreSQL 迁移双写期数据不一致 | 中（30%） | 4 | 1.2 | ① 双写期每小时跑对账脚本（diff PostgreSQL vs JSON Store 的每表 count + hash 抽样 1%）② 读流量切到 <10% 时停 24h 观察对账差异率 < 0.01% 再全量切 |
| R6 | **进度** | 三目录重命名导致 CI 4 个 workflow 路径错（build 全红） | 高（60%） | 2（低） | 1.2 | ① 改完路径后**立即跑 4 个 workflow 手动触发**（`gh workflow run enterprise-ci.yml`）不等 PR ② 所有 CI `on.push.paths` 含新旧两套路径过渡期 1 个月 |
| R7 | **进度** | OpenTelemetry 三语接入 span 上下文断链（Trace 不连通） | 中（30%） | 3 | 0.9 | ① 先只接入 Node 侧（1 天）→ 验证 Trace 通 → 再 Rust → 再 Python，分 3 步逐步加，不一步到位 ② 每个 SDK 写 1 个"上下文传递"单测：父 span trace_id = 子 span trace_id |
| R8 | **数据** | 迁移脚本 `migrate-tenant.js` 跑时写坏 projects.json 等 50+ 数据文件 | 中（25%） | 5 | 1.25 | ① 脚本开头**自动做全量 data/ 目录 zip 备份**到 `data_backup_YYYYMMDD_HHMMSS.zip` ② 脚本加 `--dry-run` 模式：只打印要改的 ID 列表，不写盘；确认后再正式跑 |
| R9 | **数据** | llm_usage.jsonl 无限增长吃掉磁盘 | 中（20%） | 3 | 0.6 | ① 按天分文件：`llm_usage-YYYYMMDD.jsonl` ② 加 `rotate-logs.ps1` 每日 0 点跑，30 天前的 gzip，90 天前的删；df 监控告警阈值 80% |
| R10 | **安全** | 插件 Worker 沙箱逃逸（恶意插件读根目录） | 低（5%） | 5 | 0.25 | ① worker_threads 启动时 `execArgv: ['--disallow-code-generation-from-strings']` + 白名单 require（仅 fs.readFile，且路径限定 `plugin_data/<plugin_id>/`）② 安装前 Manifest 声明 `permissions: ['storage:read:*']` 这种通配级别超阈值的默认拒绝，需管理员手动审批 |
| R11 | **安全** | JWT secret 用默认值被公网猜解 | 中（25%） | 5 | 1.25 | ① 和 admin 密码同策略：`MOX_JWT_SECRET` 必须从环境变量读，空值启动报错，**禁止写默认值入代码** ② 首次启动若检测到弱 secret（<32 字节或常见字符串），自动生成 64 字节随机串打印到 stdout，供用户保存 |
| R12 | **商业** | 计费模型不合理（Token 单价定太高/太低） | 中（40%） | 3 | 1.2 | ① MVP 前 3 个月 4 档套餐全部 **Free Beta**，只计量不收费，收集真实用量分布（P50/P90/P99 token/客户）再定价格 ② 定价参考 Dify/Coze 国内版价目表，打 8 折切入市场 |
| R13 | **商业** | 市场模板生态冷启动（没人传模板 → 市场空 → 用户走） | 高（70%） | 3 | 2.1 | ① 官方首批出 10+ 高质量模板（企业信息化/知识管理/AI 客服/需求编译/算子开发工作流 等 5 大场景各 2 份）② 上传模板审核通过即送 3 个月 Team 套餐兑换码（激励 UGC） |
| R14 | **用户体验** | 多租户改造后 ProjectPicker 二级选择让老用户不适应 | 中（35%） | 2 | 0.7 | ① 单租户模式下（DEFAULT_TENANT 只有 1 个）→ 自动隐藏租户 Chip，体验和原来一样 ② 首次进入弹 15 秒引导 overlay："现在你可以切换租户啦" |
| R15 | **运维** | Docker Compose 升级时 PostgreSQL 数据文件丢失 | 低（10%） | 5 | 0.5 | ① postgres 用 `volumes: [mox-pg:/var/lib/postgresql/data]` **命名卷**（不 bind mount），`docker compose down` 不加 -v 永远不删 ② 每日 0 点自动 `pg_dump` 到 `backups/pg-YYYYMMDD.dump`，保留 30 天 |
| R16 | **运维** | 灰度升级时新实例健康检查假阳性（端口开但内部未就绪），切流量后 5xx 爆增 | 中（30%） | 4 | 1.2 | ① 健康检查不用 `TCP 端口可连`，用**业务级 `/health/detailed`**：检查 PostgreSQL 可连 + Redis PING + AI gateway 连通 + 最近 1 分钟错误率 < 1%，全部通过才 200 ② 切流量每次只切 10%，观察 5 分钟 5xx < 0.1% 再下一步；任何一步异常自动回切 |

**高风险项（风险值 ≥ 1.2）**：R2/R5/R6/R8/R11/R12/R13/R16 共 8 项，**必须在对应任务开始前写好缓解 Checklist 贴在任务看板顶部**。

---

## 十五、ROI 测算表（四阶段投入 vs SaaS 化回收）

### 15.1 投入成本（按 4 人 × 10 周 = 10 人月 估算，中国一线城市场景）

| 项目 | 单价（¥） | 数量 | 小计（¥） | 备注 |
|------|----------|------|----------|------|
| 人力成本 | 35,000 / 人月 | 10 人月 | 350,000 | 4 人团队（后端1/前端1/Rust1/DevOps1兼架构），平均 ¥35k 月薪含社保公积金 |
| 云服务器（开发+预发+CI） | 2,000 / 月 | 3 台 × 3 月 | 18,000 | 开发 8C16G，预发 8C16G，CI Runner 16C32G，阿里云/腾讯云 |
| 软件许可证 | 2,000 / 月 | 3 月 | 6,000 | JetBrains 全家桶 ×4 + Sentry Business（错误追踪） |
| 第三方服务 | - | - | 2,000 | Stripe 开户 + 短信/邮件验证码服务 + 域名 SSL 证书等杂项 |
| 不可预见费（10%） | - | - | 37,600 | 应对 R1-R16 中任何一项真的发生后的应急成本 |
| **合计投入** | | | **¥413,600** | 约 **¥41 万**，SaaS MVP 从 0 到上线全链路成本 |

### 15.2 SaaS 化后月度收入（MRR）预测（保守/中性/乐观 3 档）

4 档套餐定价（Beta 期 3 个月免费，第 4 个月开始收费）：
- Free：¥0 / 月（5 QPS，1 个项目，社区支持）— 获客入口，不收费
- Pro：¥199 / 月（20 QPS，20 个项目，邮件支持）
- Team：¥999 / 月（200 QPS，100 个项目，工单支持）
- Enterprise：¥9,999 / 月起（定制 QPS/项目，SSO 对接，专属客户经理，SLA 99.9%）

| 客户规模维度 | 保守（12 个月） | 中性（12 个月） | 乐观（12 个月） |
|-------------|---------------|---------------|---------------|
| Free 用户数 | 2,000 | 5,000 | 10,000 |
| → 付费转化率 | 3% | 5% | 8% |
| → 付费用户数 | 60 | 250 | 800 |
| → 付费结构（Pro/Team/Enterprise） | 80/18/2 | 70/25/5 | 60/30/10 |
| **月收入 MRR** | ¥60×(0.8×199+0.18×999+0.02×9999) = ¥29,664 | ¥250×(0.7×199+0.25×999+0.05×9999) = ¥222,100 | ¥800×(0.6×199+0.3×999+0.1×9999) = ¥1,135,200 |
| 年度收入（×12） | ¥355,968 | ¥2,665,200 | ¥13,622,400 |
| LTV 估算（平均客户留存 18 个月） | ¥533,952 | ¥3,997,800 | ¥20,433,600 |
| **12 个月 ROI** | **-14%（略亏）** | **+544%（投资回本 ×6.4）** | **+3195%（投资回本 ×33）** |
| **回本周期** | 第 15 个月（Beta 后 12 个月） | 第 4 个月（Beta 期结束后次月，¥222k MRR > ¥41k 月摊销成本） | 第 2 个月（Beta 中就开始接 Enterprise 定制） |

### 15.3 不做 SaaS 化的机会成本（反向 ROI）

如果保持单实例私有化部署模式（现状）：
- 每个客户部署 + 定制 = 2 人月 = ¥70k
- 一年最多接 10 个客户（人力瓶颈）= 年收入 ¥700k
- **和中性 SaaS 化的 12 个月 ¥266 万收入比，少赚 3.8×**
- 且私有化部署的定制会让代码分支越来越乱，Rust/Node 双栈维护成本线性上涨（SaaS 化是唯一能规模复制的路径）

**结论**：中性场景下 SaaS 化的 ROI 极高，哪怕是保守场景也能在 15 个月内回本，**值得立即投入**。

---

## 十六、30 天快速里程碑（D1-D30 按天拆，每天有验收物）

> 假设 D1 = 2026-08-27（周四）。阶段一（P1 全清）+ 阶段二 60% 工作量，可在 30 天内完成 P0/P1 全部清零 + P2 核心部分上线，**达成「可演示给种子客户的单租户 SaaS Demo 版」里程碑**。

| 天数 | 日期 | 交付物 | 验收人 | 验收标准 |
|------|------|-------|-------|---------|
| D1-D2 | 08-27,28 | 任务 1.1 + 1.2 完成：根目录干净 + graph.json 入仓状态修复 + git 历史瘦身报告（若需要 filter-repo） | DevOps TL | `git status --short` 输出 ≤ 3 行；仓库体积（du -sh .git）比 D0 减少 ≥ 150MB |
| D3 | 08-29 | 任务 1.5 完成：P2-5 服务定义双源冲突修复 + 单测通过 | 后端 TL | `new ServiceManager().list().length === 3`（含 xiaobai_voice）；端口和 platform_config.json 完全一致 |
| D4-D5 | 08-30,31 | 任务 1.6：scripts 4 件套 + 任务 1.7 sccache + CI 优化 | DevOps TL | 新人机器（无 node_modules / target 缓存）：`.\scripts\setup-dev.ps1` 30min 内跑完；`.\check-all.ps1` 全绿退出码 0 |
| D6-D7 | 09-01,02 | 任务 1.3：三目录重命名 + 所有路径引用全量替换 | 全栈 TL | `rg "projects/(?!showcase)" / "my_projects" / "^workspace/" 不在源码中出现；Cargo build / npm run dev / python xiaobai_voice 全正常启动 |
| **周末** | 09-03,04 | 休息日，可选 R2 历史改造窗口（filter-repo 执行日） | — | — |
| D8 | 09-05 | 任务 2.2 + R11 缓解：admin 密码 + JWT secret 环境变量化 + 启动校验 | 安全负责人 | 空环境启动进程退出码=1 且输出「请生成哈希命令」；正确设置后登录成功 |
| D9-D10 | 09-06,07 | 任务 2.1：四层身份模型写入 types.js + migrate-tenant.js 迁移脚本 | 架构师 | Postman：A/B 两租户数据隔离 API 测试通过（见任务 2.1 验证） |
| D11-D13 | 09-08,09,10 | 任务 2.4：23 路由域 tenant_guard 中间件 60%（覆盖 projects/tasks/experts/chat/graph/ai-engine 6 核心路由） | 后端 TL | 6 路由各写 1 个"跨租户访问=403"单测全部通过 |
| D14-D15 | 09-11,12 | 任务 2.3：ProjectPicker 二级选择器 + projectContext 升级 | 前端 TL | UI 测试：切换租户 A→B 后，projects 列表立即刷新为空；A 建的项目 B 看不到 |
| **周末** | 09-13,14 | 休息日 | — | — |
| D16 | 09-15 | R8 缓解 + 任务 2.1 联调：跑完整迁移脚本 dry-run + 正式迁移，所有数据文件打 zip 备份 | 架构师 + DBA | 迁移后 projects.json 每条记录含 tenant_id 字段且值 = 'default'；备份 zip 文件 md5 校验通过 |
| D17-D18 | 09-16,17 | 任务 2.5：AI 计量 + 双 TokenBucket 接入 ai-engine-core | 后端 TL | 10 次 AI 对话后 llm_usage.jsonl 有 10 行记录；TRIAL 档连续发 6 次请求后返回 429 QuotaExceeded |
| D19-D21 | 09-18,19,20 | 任务 2.6：Rust spread_activation 算法实现 + napi 绑定 + JS 侧接线 | Rust TL | 单测：50 节点专家图 + 3 seed 关键词 → 30 轮收敛 σ̄<0.06；输出第一名专家 score>0.6；1000 节点耗时<200ms |
| D22 | 09-21 | 任务 2.4 剩余 40%（剩余 17 路由域的 tenant_guard） | 后端 TL | 23 路由域 100% 全覆盖；`rg "\.use\(tenantGuard" routes/*.js` 行数 = 23 |
| D23-D24 | 09-22,23 | **Demo 日准备**：修 2.1-2.6 的联调 bug + 录屏种子数据（两个示例租户 A 科技公司 / B 金融公司） | QA + 产品 | Demo 脚本走完：注册 A/B → A 建"企业知识图谱项目"走 S1→S3 → B 建"风控系统项目" → 互不可见对方项目 |
| **周末** | 09-25,26 | 休息日 | — | — |
| D25 | 09-27 | **M1 里程碑评审日**：给 3-5 个种子客户演示 SaaS Demo 版 + 收集反馈 | 产品 TL + 管理层 | Demo 过程零崩溃；客户反馈收集表 ≥ 3 份；NPS ≥ 7 分 |
| D26-D27 | 09-28,29 | 修复 Demo 暴露的 Top 5 问题（按客户反馈优先级排序） | 全团队 | Top 5 问题全部 Close；回测 Demo 脚本再走一遍无回归 |
| D28 | 09-30 | 阶段三 3.1 启动：PostgreSQL 6 张基础表 migration 001 写好 + 本地 docker-compose up 启动 PG | DBA | `docker run postgres:16` 后 `\dt` 显示 6 张表，`\d projects` 显示 tenant_id 字段 + RLS 已 ENABLE |
| D29-D30 | 10-01,02 | **国庆期间代码冻结**（避免假期线上出事），只写阶段三剩余任务的技术方案设计文档 + Task 拆分到小时级 | 架构师 | 3.2-3.6 每项任务的技术方案评审通过；11.5 人日的阶段三拆成 46 个 ≤ 4h 的子任务 |

---

## 十七、文档变更记录 + V1.0 → V1.1 修订对比总表

| 文档版本 | 日期 | 变更章节 | 关键修订内容 |
|---------|------|---------|------------|
| V1.0 | 2026-08-26 | 一 ~ 十 | 首版：目录推断 + 原分析校准 + 四阶段路线图（16 周）+ 26 条验收项 |
| V1.1 | 2026-08-26 | **新增 十一**：V1.0 遗漏 5 大已实现模块 | 纠正重大低估：RBAC 完整角色/AI编排4入口100%符合硬约束/配额限流4档已写/激活扩散钩子已留/插件已3个 |
| V1.1 | 2026-08-26 | **新增 十二**：P2-5 服务定义双源冲突 | 新发现问题：service-manager 硬编码和 platform_config.json 冲突（端口/服务数不一致） |
| V1.1 | 2026-08-26 | **新增 十三**：落地执行矩阵 4 阶段 24 子任务 | 逐任务给：文件锚点/改动摘要/验证/回滚/耗时/负责人，48-49 人日可量化 |
| V1.1 | 2026-08-26 | **新增 十四**：风险缓解矩阵 16 项 | P×I 二维打分；8 项高风险（≥1.2）必须前置缓解 Checklist |
| V1.1 | 2026-08-26 | **新增 十五**：ROI 测算 3 档 | ¥41 万投入；中性 12 月 ROI +544%（¥266 万收入）；第 4 个月回本 |
| V1.1 | 2026-08-26 | **新增 十六**：30 天快速里程碑 D1-D30 | 按天拆交付物+验收标准；D25 M1 种子客户 Demo 日 |

**整体预估工期变化**：V1.0（16 周）→ **V1.1（9-10 周）**，压缩 40% 工期直接来源 = 第 11 章证实的 5 大模块已写好，不需要从零开发。

---

## 十八、下一阶段立即执行指令（ExperienceRecall 方法论应用）

根据 ExperienceRecall 中「阶段门禁」原则：**用户当前请求是"总结分析优化文档"，属于设计阶段 → 只输出设计与执行矩阵，不擅自进入代码修改**。

因此 V1.1 文档完成后，下一步的 3 个动作**必须在用户明确下达"开始执行"指令后再启动**：

| # | 动作 | 触发条件 | 预计产出 |
|---|------|---------|---------|
| 1 | 执行任务 1.1 + 1.2（清理垃圾 + graph.json 检查） | 用户回「开工 / 执行阶段一」 | `git status` 干净报告 + 仓库瘦身前后对比表 |
| 2 | 拉技术评审会（2 小时）过 24 项执行矩阵 | 用户回「安排评审」 | 评审会议纪要（含任务调整 + 负责人签字） |
| 3 | 在项目管理工具（飞书任务/Notion/..）中创建 24 张任务卡 + 30 天里程碑看板 | 用户回「拆任务卡」 | 任务看板 URL（每张卡附验证方法 + 截止日期） |

> （ExperienceRecall 失败教训应用：避免把"设计优化方案"误执行为"直接改代码"，导致用户打断回退——严格遵守阶段门禁原则）
