# 一键启动脚本分析与优化 实现计划

## 仓库研究（现状结论）

### 1. 脚本全景
| 位置 | 文件名 | 作用 | 平台 |
|---|---|---|---|
| 仓库根 | [start.sh](file:///d:/a10/aikjx/gitcode/infotopograph/start.sh) | "算子统一系统" bash 启动：检查 Rust → cargo build --release → python verify → 运行 operator-server | Linux/Mac |
| 仓库根/scripts/ | [manage.py](file:///d:/a10/aikjx/gitcode/infotopograph/scripts/manage.py) | 璇玑系统统一运维 CLI：`start/stop/restart/status/logs/dashboard/verify/init` + Web 面板（stdlib http.server） | Windows/Linux（跨平台） |
| 仓库根/scripts/ | [run_enterprise_7gates.ps1](file:///d:/a10/aikjx/gitcode/infotopograph/scripts/run_enterprise_7gates.ps1) / [verify_tests.ps1](file:///d:/a10/aikjx/gitcode/infotopograph/scripts/verify_tests.ps1) 等 | 验收测试类脚本，非服务启动 | Windows |
| platform/backend-node/scripts/ | [run-10task-rubric.ps1](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/scripts/run-10task-rubric.ps1) | 企业 10 类评分 | Windows |

### 2. 真实服务清单（当前 platform_config.json + manage.py DEFAULT_CONFIG）
| 服务 key | 端口 | 命令 | 启动路径 | 依赖检查 |
|---|---|---|---|---|
| api | 3010 | `node src/api-server.js` | platform/backend-node | ✅ npm_deps |
| frontend | 3020 | `npm run dev` | frontend-ui | ✅ npm_deps |
| (gateway rust) | — | cargo run -p runtime | platform/gateway/runtime | ❌ 未登记 |
| dashboard | 3040 | 由 manage.py dashboard 命令直接启动 | scripts/manage.py | 无端口占用检查前清 PID |

### 3. 问题清单（按严重度排序）
**P0 致命 - 启动脚本与真实系统不一致**
- `start.sh` 启动 `operator-server`，但仓库实际服务是 Node `api-server.js` + Vite `frontend`；`operator-server` 在 Cargo.toml 里已不存在（属于算子旧系统遗留，直接 cargo 运行会找不到 target/bin），导致 Linux 用户直接 `./start.sh` 必失败。
- `start.sh` 的 "前端:3000 / API:3000/api" 与真实端口 3010/3020 不一致，打印虚假访问地址。

**P1 高 - 依赖/前置检查缺失**
- `manage.py start` 中 `command: "node ..."`/`"npm ..."`：未检查 `node`、`npm` 是否存在于 PATH（只在 `_ensure_npm_deps` 里查了 npm），Windows 下若 node 缺失会得到 cryptic "系统找不到指定的文件" 且不会 tail 日志。
- `free_port()` 只支持 Windows（`if os.name != "nt": return False`），非 Windows 端口占用会直接启动失败+无清理。
- `_ensure_npm_deps` 以 `node_modules/` 目录存在判定依赖就绪；若 `package-lock.json` 变了而 node_modules 旧版，会静默使用过期依赖。
- dashboard 启动时的"端口占用"检查只做了 `check_port(port)`，未像 ServiceManager 那样调用 `free_port()` 尝试释放。

**P1 高 - 启动顺序/依赖关系/健康判定不严谨**
- 服务间无依赖图：`frontend` 依赖 `api`（Vite 启动要访问 proxy 目标），但 `auto_start` 顺序是字典插入顺序（恰好 api→frontend 靠 JSON 巧合维持），一旦用户改配置在 frontend 前插入其它服务就会启动即挂。
- `start_all_auto()` 是串行 `start(key)`，端口未就绪（wait_time 之后仍未监听）也不失败、不中断后续，导致健康状态 "半启动"。
- `status` 合并了 `alive || port_ok`，**port 存活但 pid 不存在时仍判 running**，这是僵尸端口假阳性；而且会清空 PID 文件但不杀进程。
- 健康检查 `http_ok()` 路径默认 `/`，API 服务是 `/health`；对未声明 health_check 的服务只 check 端口，无法区分 "监听但未初始化完毕"。

**P2 中 - 进程/端口/日志管理鲁棒性**
- `subprocess.Popen(cmd, shell=True, ...)` 用字符串命令启动，Windows 下父 python 进程是 `shell=True` 的包装 PID，但真正的 node 子进程是其孙进程；`taskkill /PID <pid> /T` 有时会无法遍历孙进程（特别是 `npm run dev` 实际再开的 node vite），停服务残留 node 占用 3020。
- 停止流程 `stop_all` 是 `reversed(service_keys)`；没有 "被依赖者先停、依赖者后停" 的拓扑顺序语义。
- 日志按 "追加" 写入单个 `{key}.log`，无限增长无轮转。
- `get_status()` 返回 `url: f"http://localhost:{port}"`，但实际 frontend 是 vite 需 host，API 为 `:3010/api`；URL 不准。

**P2 中 - 跨平台与"一键启动"单一入口缺失**
- Windows 用户无可一键脚本：需要手动记住 `python scripts/manage.py start all` + 再开终端跑 `dashboard`；Linux 有 start.sh 但脚本已过期。
- 无 "一键启动 = 依赖检查 + 停止残留 + start_all + 开 dashboard" 的最短路径命令。
- `start.sh` 用 `set -e` 但 `curl`/`cargo build` 失败后静默停止，也没有失败日志 dump。

**P3 低 - 代码/文档/注释/默认值细节**
- `stop_process_tree` Windows 分支 force 参数被忽略（两行都插 `/F`），注释说"非 force 尝试优雅停止"实际永远强制。
- `free_port` 依赖 `netstat` 输出 GBK 解码，Windows 非中文区域可能漏匹配；建议 fallback `Get-NetTCPConnection`。
- platform_config.json 默认 admin/admin123 硬编码明文，建议首次启动生成随机密码落盘或至少用 env 覆盖。
- start.sh 中 "数学公理验证完成" 失败分支 `echo "⚠️  公理验证完成（部分警告不影响运行）"` 语义矛盾。

---

## Files and Modules

### 修改（按优先级）
1. **[scripts/manage.py](file:///d:/a10/aikjx/gitcode/infotopograph/scripts/manage.py)**（主优化点）
   - 新增 "bootstrap" 统一入口：依赖预检 → 清理残留 → start_all → 可选 dashboard
   - 新增 `depends_on: []` 服务拓扑 + 按依赖排序的 `start_all_sorted` / `stop_all_sorted`
   - 新增 "node/npm/python/cargo 二进制存在性" 预检（`shutil.which` + `--version` 回显）
   - 修复 `free_port`：支持 Linux/Mac `lsof`/`ss` 端口杀进程
   - 修复 `stop_process_tree`：非 force 先优雅 `/T` 再 5s 后强制
   - 新增 日志轮转（单文件 > 5MB 切为 `{key}.1.log`，保留 3 份）
   - status 区分 4 态：STARTING / RUNNING / DEGRADED(port-only) / STOPPED
   - 新增 `--dry-run` 与 `--strict`：strict 模式下 wait_time 内端口未就绪直接失败

2. **[start.sh](file:///d:/a10/aikjx/gitcode/infotopograph/start.sh)**（对齐真实系统）
   - 重写为"POSIX 一键入口"：优先调用 `python3 scripts/manage.py` 子命令组合，保留 cargo build 作为可选 `--build-rust` 开关
   - 修复前端/API 地址打印为真实 3020 / 3010 / 3040 dashboard
   - 失败统一 dump 最近 30 行日志到 stdout

3. **新增 [scripts/start.ps1](file:///d:/a10/aikjx/gitcode/infotopograph/scripts/start.ps1)**（Windows 一键启动对等物）
   - 语义 = start.sh 的 PowerShell 版：调用 `py.exe scripts/manage.py start all --strict` → `dashboard --no-browser`
   - 支持参数：`-NoBuild`、`-OnlyApi`、`-Restart`、`-OpenDashboard`
   - 输出企业级彩色状态清单：✔/✗ + 端口

4. **[platform_config.json](file:///d:/a10/aikjx/gitcode/infotopograph/platform_config.json)**
   - 为 api/frontend 加 `depends_on`（frontend → [api]）
   - 新增可选 `startup_order_hint` 字段
   - 新增 dashboard 服务条目（允许由 start all 自动起 dashboard）

### 新增脚本附属（可选验证）
- `.trae/documents/` 下不新增（遵循 "NEVER proactively create docs"）；但可在优化完成后以 `manage.py verify --self-check` 命令输出 JSON 自检报告。

---

## Implementation Steps（依赖顺序）

1. **分析报告 → 本计划**（已完成 ✔）
2. **改造 manage.py 核心**
   - Step 2a：新增 topo_sort 与 depends_on 解析；start_all/stop_all 改按拓扑顺序
   - Step 2b：新增 依赖二进制预检函数 `ensure_runtime_deps(svc)`；命令启动前 node/npm/cargo 缺失直接 fail 并给出安装链接提示
   - Step 2c：统一 `free_port` 实现，新增 Linux/ss + macOS/lsof 分支；端口杀进程前做 "owner 是否是本项目路径" 白名单校验，避免误杀系统进程
   - Step 2d：重写 `stop_process_tree` 优雅 + 强制两段，force 语义生效
   - Step 2e：status 四态 + DEGRADED 报警；start() 在 --strict 下端口未就绪返回 False 并中断串联
   - Step 2f：日志轮转 + 启动前截断控制（`log_rolling(max_bytes=5*1024*1024, backup=3)`）
   - Step 2g：新增 CLI action `bootstrap` = init + stop_all(force=True) + start_all_sorted(strict=True) + 可选 dashboard（`--with-dashboard`）
3. **重写 start.sh**：去除 operator-server 路径，默认走 manage.py bootstrap（`$PYTHON manage.py bootstrap --with-dashboard`）
4. **新增 start.ps1**：实现 Windows 一键启动，`$ErrorActionPreference='Stop'`，色彩化输出
5. **调整 platform_config.json**：`depends_on` 字段示例；保留向后兼容（缺失即无依赖）
6. **回归验证**
   - python scripts/manage.py init → list → status（全部 STOPPED）
   - python scripts/manage.py bootstrap --strict（缺 node/npm 时要提前 fail 并给出安装路径提示）
   - 有环境下：start all → status 全 RUNNING → stop all → 端口全部释放
   - Linux/Mac 端：`bash start.sh --dry-run` 不做实质动作只打印步骤
   - 验证 scripts/verify_tests.ps1 中与 manage.py 相关的调用不被破坏（如果有）

---

## Dependencies and Considerations

- **平台差异必测**：PowerShell 5.1 vs 7.4；WSL；Python 3.10/3.11；`os.name == "nt"` 分支必须本地真跑一次端口杀。
- **不引入第三方包**：manage.py 必须 stdlib-only；numpy 仍然只是 verify 子命令可选。
- **向后兼容**：原 `start/stop/restart/status/logs/dashboard/verify/init` 命令与参数签名保持不变；新增 `bootstrap` 是纯新增 action。
- **安全**：杀进程白名单（必须由 cwd 或 命令路径命中 PROJECT_ROOT 下的解释器或 package.json 才杀），避免 manage.py 以高权限运行误杀系统 node。
- **shell=True 风险**：Windows 下 `command` 配置字段仍保留字符串（用户常写 `node --trace-warnings src/api-server.js` 含 args）；但新增 `args` 字段支持 list 形式 list→Popen(list, shell=False) 优先。
- **AIS 分层合规**：依赖编排逻辑放在 `ServiceManager`（业务层），底层 `free_port / is_process_alive` 继续在 util 函数（共享层）；不打乱现有分层。

---

## Validation

1. **静态**：PEP8/语法检查 `python -m py_compile scripts/manage.py`
2. **无依赖环境验证**：临时移除 PATH 中 node，运行 `python scripts/manage.py start api` → 预期 [ERROR] "未找到 node，请安装..."（不出栈追踪）
3. **启动闭环**（若本机有 node）：
   - `python scripts/manage.py bootstrap --strict --with-dashboard --no-browser`
   - `curl http://localhost:3010/health` → 200
   - `curl http://localhost:3020/` → 200
   - `python scripts/manage.py status` → api=RUNNING, frontend=RUNNING
   - `python scripts/manage.py stop all --force`
   - `python scripts/manage.py status` → 2 STOPPED；`netstat -ano` 中 3010/3020 不再 LISTENING
4. **start.sh dry-run**：`bash -n start.sh`（语法检查）
5. **start.ps1 语法**：`powershell -NoProfile -Command "& { $ErrorActionPreference='Stop'; . .\scripts\start.ps1 -DryRun }"`

---

## Risks

| 风险 | 影响 | 处理/兜底 |
|---|---|---|
| 杀进程白名单规则过严导致杀不掉残留 vite | stop --force 失败 / 端口仍占 | 兜底在 strict 模式下抛错并让用户手工执行 `Stop-Process -Name node -Force` |
| 配置里用户自定义的 shell 命令含管道 `&` / `&&` 切换 list 形式会失效 | 自定义 command 启动失败 | 仅当用户显式声明 `args: [...]` 用 list；否则保持 shell=True 不变 |
| start.sh 对旧 operator-server 用户不兼容 | 老脚本用户调用失败 | 在 start.sh 顶部打印 "已迁移到 manage.py 新链路"；保留 `--legacy` 参数可走旧 cargo 路径（但默认关闭） |
| 新增 depends_on 字段顺序歧义 | 启动顺序与用户期望错位 | JSON Schema 校验 depends_on 引用必须指向同文件 services 已存在 key，不存在报错 |
