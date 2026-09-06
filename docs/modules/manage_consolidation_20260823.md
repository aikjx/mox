# 运维脚本整合交付说明（2026-08-23）

## 目标
将仓库根目录原本分散的四个脚本整合为 **一个** 架构化、脚本，放在 `scripts/` 下：
- `service_manager.py`  命令行服务管理器
- `service_monitor.py`   Web 监控面板（带登录/权限）
- `platform_manager.py`  配置驱动 + 权限 + Web 面板 超集
- `verify_axioms.py`     算子统一系统六大公理数学自洽性验证

## 交付物
**`scripts/manage.py`**（stdlib-only，无 Flask 依赖，~53KB）

### 架构（单文件分区）
1. 通用：强制 UTF-8 输出（Windows GBK 控制台兼容）
2. 路径约定：仓库根解析，pid→`.runtime/`，log→`.logs/`，配置→`platform_config.json`
3. 工具函数：PID 读写、进程存活、端口/HTTP 探测、进程树终止、端口释放
4. `ConfigManager`：读取并合并 `platform_config.json`（缺失回退 `DEFAULT_CONFIG`）
5. `ServiceManager`：状态判定（pid+端口跨进程可感知）、start/stop/restart/start_all/stop_all/restart_all、npm 自动安装、日志
6. Web 面板：`AuthManager`（Cookie 会话）+ stdlib `http.server` 实现，登录 + 权限分级 + 单服务/批量操作 + 日志弹窗
7. 公理验证：`cmd_verify` + `_axiom1..6`、`_conservation`（numpy 可选，缺失时报错退出）
8. CLI：argparse 融合全部子命令

### 子命令
`list | start [svc|all] | stop [svc|all] [--force] | restart [svc|all] | status | logs [svc] [--lines N] | dashboard [--host] [--port] [--no-browser] | verify | init`

## 验证结果
- `py_compile` 通过
- `list` / `status`：正确跨进程检测已在运行的服务（端口探测，pid=None 也正常）
- `verify`：exit=0（numpy 可用；全部公理通过）
- Dashboard 冒烟：
  - `GET /login` → 200
  - `GET /` 未登录 → 302 重定向 /login
  - `GET /api/status` 未登录 → 200（admin_only 服务被过滤）
  - `POST /api/login` 正确 → 200 + Set-Cookie
  - 带 cookie 的 `GET /api/status` → 200（含 admin_only）

## 清理动作
删除：`service_monitor.py`、`service_manager.py`、`platform_manager.py`、`verify_axioms.py`（根目录），以及先前已存在但依赖 Flask（且未安装）的部分整合版 `scripts/service_manager.py`。
修正引用：`start.sh`（`verify_axioms.py` → `scripts/manage.py verify`）、`README.md`（目录树 + 验证命令）。

## 备注
- `.workbuddy/memory/2026-08-18.md` 为历史日志，仍提及旧文件名，属记录性质不影响运行，未改动。
- 运行中的 api(17656)/frontend(73572) 由其他进程启动，整合脚本通过端口探测正确识别为「运行中」。
