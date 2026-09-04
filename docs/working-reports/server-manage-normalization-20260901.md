# server-manage.py mox 模块化系统架构整理·修复·规范标准化报告

- 日期：2026-09-01
- 对象：`scripts/server-manage.py`（璇玑系统统一运维脚本，单文件整合版）
- 类型：mox 模块化系统架构整理 / 修复 / 规范标准化
- 前置：脚本 docstring 已归一化为 `server-manage.py`（版本 3.0），内部 `manage.py` 引用已归一化（白名单双签名保留兼容）

---

## 一、核心问题诊断

`scripts/` 目录实际文件为 `server-manage.py`，但仓库大量外部调用方仍指向 `manage.py`（该文件此前**不存在**），导致运维链路整体断裂：

| 调用方 | 指向 | 处理 |
|---|---|---|
| `start.sh`（10 处） | `scripts/manage.py` | ✅ 改指权威名 |
| `scripts/start-all.ps1` | `scripts/manage.py bootstrap` | ✅ 改指权威名 |
| `scripts/stop-all.ps1` | `scripts/manage.py stop` | ✅ 改指权威名 |
| `scripts/deploy/start.ps1` | `scripts\manage.py` | ✅ 改指权威名 |
| `platform_config.json`（script_catalog） | `"path": "scripts/manage.py"` | ✅ 改指权威名 |
| `scripts/README.md` | 目录树 / 命令示例 | ✅ 主入口标注 + 兼容别名说明 |

另发现 2 处规范性缺陷（非引用断裂）：

1. **`_spawn_command` 日志句柄泄漏**：`stdout=open(...)` 未显式持有/释放，父进程句柄依赖 GC 延迟关闭。
2. **`_spawn_command` 回归 bug（P0）**：`Path(args)` 误将**整个 list** 传给 `pathlib.Path`，在 Windows 上**直接抛 TypeError**（实测 `argument should be a str or an os.PathLike object ... not 'list'`），导致任何配置了相对路径可执行文件 args 列表的服务**启动即失败**，且因 TypeError 非 `FileNotFoundError`，**不会**回退到 command 分支。

---

## 二、修复明细

### 1. 文件名-引用断裂归一化

- **新建 `scripts/manage.py` 兼容别名薄壳**（1202 字节）：检测权威入口存在性 → `subprocess.run([sys.executable, 权威路径] + sys.argv[1:])` 原样转发 → `KeyboardInterrupt` 返回 130。历史命令 / 历史文档不再断链。
- **外部调用方全部改指权威名** `server-manage.py`（上表 6 处）。
- **权威规范文档同步**：`docs/ports/PORT-REGISTRY.md`（§3.1 标题）、`docs/architecture/14-REPOSITORY-FULL-MAP.md`（主入口 + 运维面板）、`docs/enterprise/37-企业级处理流程规范-V1.0.md`（verify 命令）。
- **历史工作报告保留原样**（`service_startup_script_optimization_plan.md` 等 27 处）：为当时快照，且兼容别名保证其命令仍可执行，不篡改历史。

### 2. `_spawn_command` 句柄泄漏修复

- 显式持有 `log_handle`；`Popen` 成功后**立即在父进程侧 `close()`**（子进程已继承句柄继续写日志），消除日志文件被锁 / 轮转失败风险。
- 异常路径统一 `try/except` 兜底关闭句柄。
- 移除 `start()` 中死代码 `proc.stdout.close()`（`proc.stdout` 恒为 `None`，该行从未生效）。

### 3. `Path(args)` → `Path(args[0])` 回归 bug 修复（P0）

- 只对**可执行文件** `args[0]` 判断/转换绝对路径。
- 进一步规范：**仅当 cwd 下确实存在该相对可执行文件时才转绝对路径**（如 `target/release/mox-gateway.exe`）；若不存在则视为 PATH 命令（`python`/`npm` 等），保持原样由系统解析，**避免无谓回退 command 分支**。

### 4. CLI 与 Web 面板语义对齐

- `start all` / `restart all`：由 `start_all_sorted(auto_only=True)`（仅 auto_start 服务）改为 `start_all_configured` / `restart_all_configured`（**全部已配置服务**），与 Web 面板「启动所有 / 重启所有」一致。`bootstrap --with-services` 仍保持 auto_only（轻量默认）。
- `get_status` 的 url：移除 api 特判，统一 `http://localhost:{port}{health_check}`（health_check="/" 等效裸端口；api=/health、voice=/voice/health 自动带上）。

---

## 三、验证结果（全部实测）

| 验证项 | 结果 |
|---|---|
| `python -m py_compile scripts/server-manage.py scripts/manage.py` | ✅ 通过 |
| `python scripts/server-manage.py --help` | ✅ 正常（argparse 显示 `server-manage.py`） |
| `python scripts/server-manage.py init` | ✅ 创建 .runtime/.logs，识别 5 服务 |
| `python scripts/server-manage.py bootstrap --dry-run --no-dashboard` | ✅ 5 服务启动顺序 + 二进制预检全部通过（voice:30010 / api:8080 / frontend:3020 / melody2score:8012 / primiflow:8000） |
| `python scripts/manage.py list`（别名转发等价） | ✅ 正常输出服务列表 |
| `python scripts/server-manage.py status` / `manage.py status` | ✅ 正常，url 统一拼接生效 |
| `_spawn_command` 端到端（PATH 命令 python，args 列表） | ✅ 不再抛异常、不再回退、日志正确写入、进程正常退出 |
| `_spawn_command` 端到端（cwd 相对可执行文件） | ✅ 正确转绝对路径启动 |
| `scripts/verify-ports.py` | ✅ ERROR=0 WARN=0 INFO=92 |

---

## 四、改动清单

| 文件 | 动作 |
|---|---|
| `scripts/server-manage.py` | M：docstring 归一化（版本 3.0）、内部引用归一化、`_spawn_command` 句柄 + args[0] 修复、`start/restart all` 语义对齐、url 统一 |
| `scripts/manage.py` | A：兼容别名薄壳（转发权威入口） |
| `start.sh` / `scripts/start-all.ps1` / `scripts/stop-all.ps1` / `scripts/deploy/start.ps1` / `platform_config.json` / `scripts/README.md` | M：外部调用改指权威名 + 说明 |
| `docs/ports/PORT-REGISTRY.md` / `docs/architecture/14-REPOSITORY-FULL-MAP.md` / `docs/enterprise/37-企业级处理流程规范-V1.0.md` | M：权威文档统一为 `server-manage.py` |

## 五、边界说明（诚实声明）

- **未动**：历史工作报告类文档（`service_startup_script_optimization_plan.md`、`manage_consolidation_20260823.md`、`31-mox 模块化系统架构代码审计与验证报告`、`01-ENTERPRISE-OPTIMIZATION.md`）——保留历史快照，兼容别名保证其命令仍可执行。
- **未动**：`verify-ports.py` 的 EXPECTED 端口（与本脚本 DEFAULT_CONFIG 一致：5 服务 + dashboard 3999，无端口变更需求）。
- **未实际启停服务**：本报告全程仅 dry-run / list / status / 单元级进程测试，未改动用户当前运行中的服务进程状态。
