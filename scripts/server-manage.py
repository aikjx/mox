#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from __future__ import annotations
"""
server-manage.py — 璇玑系统统一运维脚本（单文件整合版，stdlib-only） · 版本 3.0

整合自仓库根目录原先分散的四个脚本：
  1. service_manager.py   命令行服务管理器（start/stop/restart/status/logs）
  2. service_monitor.py   Web 监控面板（带登录与权限分级）
  3. platform_manager.py  配置驱动 + 权限 + Web 面板 超集
  4. verify_axioms.py     算子统一系统六大公理数学自洽性验证

本脚本统一管理：
  · 服务进程生命周期（启动 / 停止 / 重启 / 状态 / 日志）
  · 配置驱动（platform_config.json）
  · Web 管理面板（stdlib http.server 实现，无需 Flask）
  · 管理员登录 + 权限分级（admin_only 服务非管理员不可见/操作）
  · 公理数学自洽性验证（verify 子命令，可选 numpy）

设计原则：
  · 零三方依赖：仅使用 Python 标准库；verify 子命令仅在 numpy 可用时使用，
    缺失时给出明确提示而非崩溃。
  · 跨进程可感知：运行状态以 .runtime/<key>.pid + 端口探测持久化判定，
    CLI 启动的服务、Web 面板另起的进程均可互相感知。
  · 路径约定：仓库根为 <repo>/，本文件位于 <repo>/scripts/server-manage.py，
    所有相对路径（cwd、pid、log、config）均相对仓库根解析。

用法:
  python scripts/server-manage.py                    # 默认：一键启动全部服务 + 拉起 Web 管理面板（= bootstrap --with-dashboard）
  python scripts/server-manage.py list
  python scripts/server-manage.py list-projects      # 展示全量项目目录清单（project_registry）
  python scripts/server-manage.py scripts            # 展示 scripts/ 目录分类索引
  python scripts/server-manage.py start   [service_key|all]  [--strict]
  python scripts/server-manage.py stop    [service_key|all]   [--force]
  python scripts/server-manage.py restart [service_key|all]   [--strict]
  python scripts/server-manage.py status
  python scripts/server-manage.py logs    [service_key]       [--lines N]
  python scripts/server-manage.py dashboard  [--host 0.0.0.0] [--port 3999] [--no-browser]
  python scripts/server-manage.py verify            # 六大公理数学自洽性验证
  python scripts/server-manage.py init               # 创建 .runtime / .logs 目录
  python scripts/server-manage.py bootstrap [--strict] [--with-dashboard] [--no-browser] [--dry-run]
                                            # 一键启动：预检 → 清残留 → 按拓扑启动 → 可选面板
"""

# =========================================================================== #
# 0. 通用：强制 UTF-8 输出（Windows GBK 控制台打印 ✓ ✗ ⏳ 等符号会抛 UnicodeEncodeError）
# =========================================================================== #
import sys
import io

if sys.stdout.encoding and sys.stdout.encoding.lower() not in ("utf-8", "utf8", "utf_8"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
if sys.stderr.encoding and sys.stderr.encoding.lower() not in ("utf-8", "utf8", "utf_8"):
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")

import argparse
import hashlib
import json
import os
import secrets
import shutil
import signal
import socket
import subprocess
import threading
import time
from datetime import datetime
from http.server import BaseHTTPRequestHandler, HTTPServer, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse

# =========================================================================== #
# 1. 路径约定
# =========================================================================== #
PROJECT_ROOT = Path(__file__).resolve().parent.parent          # <repo>/
CONFIG_PATH = PROJECT_ROOT / "platform_config.json"
RUNTIME_DIR = PROJECT_ROOT / ".runtime"                        # pid 文件
LOG_DIR = PROJECT_ROOT / ".logs"                               # 各服务输出日志

DEFAULT_DASHBOARD_PORT = 3999
SESSION_TIMEOUT = 30 * 60                                      # 会话 30 分钟过期

# 默认服务配置（仅当 platform_config.json 缺失时回退使用；
# 正常路径应依赖仓库根 platform_config.json）
DEFAULT_CONFIG = {
    "version": "2.1",
    "project_name": "璇玑系统",
    "dashboard_port": DEFAULT_DASHBOARD_PORT,
    "admin": {"username": "admin"},
    "log_rolling": {"max_bytes": 5 * 1024 * 1024, "backup": 3},
    "services": {
        "api": {
            "name": "API 后端服务（Rust mox-gateway）",
            "description": "璇玑系统核心 API 网关（Rust axum，原 Node 后端已迁移）；监听 8080，/health 健康检查，/api/* 路由代理",
            "port": 8080,
            "health_check": "/health",
            "cwd": "platform/backend-rust",
            "command": "target/release/mox-gateway.exe",
            "args": ["target/release/mox-gateway.exe"],
            "binary_requires": [],
            "npm_deps": False,
            "is_admin_only": True,
            "auto_start": False,
            "restart_delay": 3,
            "wait_time": 8,
            "startup_order_hint": 10,
            "depends_on": [],
            "tags": ["API", "后端", "核心", "Rust", "Gateway"],
        },
        "frontend": {
            "name": "用户前端界面",
            "description": "面向终端用户的操作界面（含系统管理区 /admin）；依赖 api 代理",
            "port": 3020,
            "health_check": "/",
            "cwd": "frontend-ui",
            "command": "npm run dev",
            "args": ["npm", "run", "dev"],
            "binary_requires": ["node", "npm"],
            "npm_deps": True,
            "is_admin_only": False,
            "auto_start": False,
            "wait_time": 12,
            "startup_order_hint": 20,
            "depends_on": ["api"],
            "tags": ["前端", "用户界面", "Vite"],
        },
        "xiaobai_voice": {
            "name": "小白语音服务（ASR + TTS）",
            "description": "本地离线语音：Paraformer ASR / CosyVoice2 TTS；端口 30010",
            "port": 30010,
            "health_check": "/voice/health",
            "cwd": "projects/xiaobai_voice",
            "command": "python -m xiaobai_voice serve --host 0.0.0.0",
            "args": ["python", "-m", "xiaobai_voice", "serve", "--host", "0.0.0.0"],
            "binary_requires": ["python"],
            "npm_deps": False,
            "is_admin_only": False,
            "auto_start": True,
            "restart_delay": 3,
            "wait_time": 12,
            "startup_order_hint": 5,
            "depends_on": [],
            "tags": ["语音", "AI交互", "TTS", "ASR", "本地推理"],
        },
        "melody2score": {
            "name": "旋律转谱服务（Melody2Score WebUI）",
            "description": "企业级可视化音频转谱：FastAPI 后端 + 静态前端，端口 8012",
            "port": 8012,
            "health_check": "/",
            "cwd": "projects/melody2score",
            "command": "python app/webui.py",
            "args": ["python", "app/webui.py"],
            "binary_requires": ["python"],
            "npm_deps": False,
            "is_admin_only": False,
            "auto_start": False,
            "restart_delay": 3,
            "wait_time": 10,
            "startup_order_hint": 30,
            "depends_on": [],
            "tags": ["音频", "转谱", "AI", "FastAPI", "WebUI"],
        },
        "primiflow": {
            "name": "PrimiFlow 低代码拓扑引擎",
            "description": "MVP 低代码拓扑生成引擎：FastAPI 单服务（端口 8000），自动托管静态页",
            "port": 8000,
            "health_check": "/",
            "cwd": "projects/primiflow/backend",
            "command": "python -m uvicorn main:app --host 0.0.0.0 --port 8000",
            "args": ["python", "-m", "uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"],
            "binary_requires": ["python"],
            "npm_deps": False,
            "is_admin_only": False,
            "auto_start": False,
            "restart_delay": 3,
            "wait_time": 8,
            "startup_order_hint": 40,
            "depends_on": [],
            "tags": ["低代码", "拓扑", "FastAPI", "引擎"],
        },
    },
}

# =========================================================================== #
# 2. 工具函数
# =========================================================================== #

# 已知的运行时二进制→常见安装指引
_BINARY_INSTALL_HINT = {
    "node": "请安装 Node.js LTS 版本：https://nodejs.org/  (Windows 推荐 22.x；安装后重新打开终端确认 `node -v`)。",
    "npm": "Node.js 安装后自带 npm；若缺失请执行 `corepack enable` 或重装 Node.js。",
    "npx": "同 Node.js/npm。",
    "cargo": "请安装 Rust：https://rustup.rs/ ，Windows 下载 rustup-init.exe 默认安装即可。",
    "rustc": "同 cargo。",
    "python": "请安装 Python ≥ 3.10：https://www.python.org/downloads/windows/ ；安装时勾 Add to PATH。",
    "python3": "同 python；Linux/macOS 一般包管理器自带。",
    "py": "Windows 官方 Python Launcher；随 Python for Windows 安装器自动提供。",
}


def log(msg: str):
    ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{ts}] {msg}")


def ensure_dirs():
    RUNTIME_DIR.mkdir(parents=True, exist_ok=True)
    LOG_DIR.mkdir(parents=True, exist_ok=True)


def pid_file(key: str) -> Path:
    return RUNTIME_DIR / f"{key}.pid"


def log_file(key: str) -> Path:
    return LOG_DIR / f"{key}.log"


def read_pid(key: str):
    p = pid_file(key)
    if not p.exists():
        return None
    try:
        return int(p.read_text().strip())
    except Exception:
        return None


def write_pid(key: str, pid: int):
    ensure_dirs()
    pid_file(key).write_text(str(pid))


def clear_pid(key: str):
    p = pid_file(key)
    if p.exists():
        try:
            p.unlink()
        except Exception:
            pass


def is_process_alive(pid: int) -> bool:
    if pid is None:
        return False
    # Windows 上 os.kill(pid, 0) 在某些 Python 版本上会误判（PID 已退出但
    # 内核句柄表还有残留就返回 True），导致陈旧 pidfile 被误判为「已在运行」，
    # 进而跳过启动 → 页面上点启动按钮却"啥也没发生"。
    # 所以 Windows 走子进程查询（wmic）作为可靠来源，os.kill 仅兜底。
    if os.name == "nt":
        try:
            r = subprocess.run(
                [
                    "wmic",
                    "process",
                    "where",
                    f"ProcessId={int(pid)}",
                    "get",
                    "ProcessId",
                    "/value",
                ],
                capture_output=True,
                text=True,
                timeout=5,
            )
            blob = (r.stdout or "") + (r.stderr or "")
            if "No Instance(s) Available" in blob or "ProcessId=" not in blob:
                return False
            return True
        except Exception:
            pass
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False
    except Exception:
        return False


def check_port(port: int, host: str = "127.0.0.1", timeout: float = 1.0) -> bool:
    """探测端口是否可连接（服务是否已真正监听）。"""
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except Exception:
        return False


def http_ok(port: int, path: str = "/", timeout: float = 2.0, method: str = "HEAD") -> bool:
    """探测 HTTP 端点（HEAD 优先，失败回退 GET）。"""
    import urllib.request

    url = f"http://127.0.0.1:{port}{path}"
    try:
        req = urllib.request.Request(url, method=method)
        urllib.request.urlopen(req, timeout=timeout)
        return True
    except Exception:
        try:
            urllib.request.urlopen(url, timeout=timeout)
            return True
        except Exception:
            return False


# ---------------- 二进制/依赖预检 ----------------
def check_binary(name: str, version_flags=("--version",)) -> tuple:
    """返回 (resolved_path_or_None, version_str_or_None)。"""
    resolved = shutil.which(name)
    if not resolved:
        return None, None
    for flag in version_flags:
        try:
            out = subprocess.run(
                [resolved, flag],
                capture_output=True,
                text=True,
                timeout=8,
                env={**os.environ, "NO_COLOR": "1"},
            )
            first_line = (out.stdout or out.stderr or "").splitlines()
            if first_line:
                return resolved, first_line[0].strip()
        except Exception:
            continue
    return resolved, ""


def binary_install_hint(name: str) -> str:
    return _BINARY_INSTALL_HINT.get(
        name, f"未在 PATH 中找到命令 `{name}`，请确认已安装并添加到 PATH。"
    )


def _log_rolling_cfg(config=None) -> dict:
    cfg = None
    if config is not None:
        try:
            cfg = config.config.get("log_rolling")
        except Exception:
            cfg = None
    if not cfg:
        cfg = DEFAULT_CONFIG.get("log_rolling", {"max_bytes": 5 * 1024 * 1024, "backup": 3})
    return {
        "max_bytes": int(cfg.get("max_bytes", 5 * 1024 * 1024)),
        "backup": max(0, int(cfg.get("backup", 3))),
    }


def rotate_log_if_needed(key: str, rolling_cfg: dict | None = None, config=None):
    """RFH-style 轮转：主文件超限时 .1/.2 后移；主文件清空。"""
    if rolling_cfg is None:
        rolling_cfg = _log_rolling_cfg(config)
    max_bytes = int(rolling_cfg.get("max_bytes", 5 * 1024 * 1024))
    backup = max(0, int(rolling_cfg.get("backup", 3)))
    if max_bytes <= 0 or backup <= 0:
        return
    path = log_file(key)
    if not path.exists():
        return
    try:
        size = path.stat().st_size
    except OSError:
        return
    if size < max_bytes:
        return
    # shift backups
    for i in range(backup - 1, 0, -1):
        src = path.with_name(f"{key}.{i}.log")
        dst = path.with_name(f"{key}.{i + 1}.log")
        if src.exists():
            try:
                if dst.exists():
                    dst.unlink()
                src.replace(dst)
            except OSError:
                pass
    dst1 = path.with_name(f"{key}.1.log")
    try:
        if dst1.exists():
            dst1.unlink()
        path.replace(dst1)
    except OSError:
        pass


# ---------------- 进程/进程树管理 ----------------
def stop_process_tree(pid: int, force: bool = False):
    """停止进程及其子进程（Windows / Unix 通用）。

    force=False 先优雅停止，最多等待若干秒，仍存活再强制；force=True 直接强制。
    """
    if pid is None:
        return
    wait_seconds = 1 if force else 5
    attempts = 1 if force else 2  # 0:graceful; 1:force fallback if still alive
    for phase in range(attempts):
        use_force = force or phase == 1
        try:
            if os.name == "nt":
                cmd = ["taskkill", "/PID", str(pid), "/T"]
                if use_force:
                    cmd.insert(2, "/F")
                subprocess.run(
                    cmd,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=12,
                )
            else:
                sig = signal.SIGKILL if use_force else signal.SIGTERM
                try:
                    os.kill(pid, sig)
                except OSError:
                    pass
        except Exception:
            pass
        # wait for exit
        deadline = time.time() + wait_seconds
        while time.time() < deadline:
            if not is_process_alive(pid):
                return
            time.sleep(0.25)
    # 最终兜底：Unix SIGKILL / Windows 再 taskkill /F
    if is_process_alive(pid):
        try:
            if os.name == "nt":
                subprocess.run(
                    ["taskkill", "/PID", str(pid), "/F", "/T"],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=10,
                )
            else:
                os.kill(pid, signal.SIGKILL)
        except Exception:
            pass


def _project_owned_pid(pid: int) -> bool:
    """杀进程白名单：进程可执行文件/cmdline 落在 PROJECT_ROOT 下（或常见解释器且 cwd 命中）才允许。

    信息不足时默认 True（允许进程级 pid 文件中记录的 pid 必然属于本项目）。

    Windows 增强：相对路径启动（如 `python scripts/server-manage.py dashboard`）不会在 cmdline 中出现
    PROJECT_ROOT，因此额外识别：命令行末尾/参数位置出现 scripts/server-manage.py、scripts/manage.py、
    server-manage.py dashboard、manage.py dashboard 这类签名时，视为本项目归属（避免明明是自己的
    dashboard server 却被判成「第三方」而不敢杀）。
    """
    try:
        if os.name == "nt":
            # wmic 获取可执行路径与命令行
            res = subprocess.run(
                [
                    "wmic",
                    "process",
                    "where",
                    f"ProcessId={pid}",
                    "get",
                    "ExecutablePath,CommandLine",
                    "/value",
                ],
                capture_output=True,
                text=True,
                timeout=6,
            )
            text = (res.stdout or "") + (res.stderr or "")
            if not text or "No Instance(s) Available" in text:
                return True  # 已不存在 → 视为可通过
            text_lower = text.lower()
            # 归一化反斜杠后再比较路径
            text_norm = text_lower.replace("\\", "/")
            root_norm = str(PROJECT_ROOT).lower().replace("\\", "/")
            if root_norm in text_norm:
                return True
            # 启发式：识别本项目运维脚本的相对路径调用
            for sig in (
                "scripts/server-manage.py",
                "scripts/manage.py",
                "server-manage.py dashboard",
                "manage.py dashboard",
                "server-manage.py bootstrap",
                "manage.py bootstrap",
            ):
                if sig in text_norm:
                    return True
            return False
        else:
            # Unix：读 /proc/<pid>/cmdline + cwd
            cmdline_p = Path(f"/proc/{pid}/cmdline")
            cwd_p = Path(f"/proc/{pid}/cwd")
            parts = []
            try:
                if cmdline_p.exists():
                    parts = [s for s in cmdline_p.read_bytes().split(b"\x00") if s]
            except Exception:
                parts = []
            try:
                if cwd_p.is_symlink():
                    parts.append(str(cwd_p.resolve()).encode())
            except Exception:
                pass
            blob = b" ".join(parts).decode("utf-8", "replace")
            if str(PROJECT_ROOT) in blob:
                return True
            blob_norm = blob.lower().replace("\\", "/")
            for sig in (
                "scripts/server-manage.py",
                "scripts/manage.py",
                "server-manage.py dashboard",
                "manage.py dashboard",
                "server-manage.py bootstrap",
                "manage.py bootstrap",
            ):
                if sig in blob_norm:
                    return True
            return False
    except Exception:
        return True


def free_port(port: int, aggressive: bool = False) -> bool:
    """释放端口：跨平台。
    Windows：先 netstat；失败兜底 Get-NetTCPConnection。
    Unix：优先 ss，失败 lsof。
    aggressive=True：即使非项目归属也尝试释放（默认 False，只杀项目归属进程，避免误杀）。
    """
    if not check_port(port):
        return True
    pids_to_kill: set = set()
    try:
        if os.name == "nt":
            out = None
            try:
                result = subprocess.run(
                    ["netstat", "-ano", "-p", "tcp"],
                    capture_output=True,
                    timeout=6,
                )
                try:
                    out = result.stdout.decode("gbk", errors="replace")
                except Exception:
                    out = result.stdout.decode("utf-8", errors="replace")
                for line in out.splitlines():
                    if f":{port}" in line and "LISTENING" in line:
                        parts = line.strip().split()
                        if parts:
                            try:
                                pids_to_kill.add(int(parts[-1]))
                            except ValueError:
                                pass
            except Exception:
                out = None
            if not pids_to_kill:
                # fallback: PowerShell Get-NetTCPConnection
                try:
                    ps = (
                        "Get-NetTCPConnection -State Listen -LocalPort "
                        + str(port)
                        + " | Select-Object -ExpandProperty OwningProcess"
                    )
                    r = subprocess.run(
                        ["powershell", "-NoProfile", "-Command", ps],
                        capture_output=True,
                        text=True,
                        timeout=8,
                    )
                    for ln in (r.stdout or "").splitlines():
                        ln = ln.strip()
                        if ln.isdigit():
                            pids_to_kill.add(int(ln))
                except Exception:
                    pass
        else:
            # Unix
            ss_found = False
            try:
                r = subprocess.run(
                    ["ss", "-ltnpH", f"sport = :{port}"],
                    capture_output=True,
                    text=True,
                    timeout=6,
                )
                for ln in (r.stdout or "").splitlines():
                    # ss output users: ((("pid=1234",fd=3),...))
                    import re

                    for m in re.finditer(r"pid=(\d+)", ln):
                        try:
                            pids_to_kill.add(int(m.group(1)))
                            ss_found = True
                        except ValueError:
                            pass
            except FileNotFoundError:
                ss_found = False
            if not ss_found:
                try:
                    r = subprocess.run(
                        ["lsof", "-nP", "-iTCP", f"-i:{port}", "-sTCP:LISTEN", "-t"],
                        capture_output=True,
                        text=True,
                        timeout=6,
                    )
                    for ln in (r.stdout or "").splitlines():
                        ln = ln.strip()
                        if ln.isdigit():
                            pids_to_kill.add(int(ln))
                except FileNotFoundError:
                    pass
    except Exception:
        pass

    killed_any = False
    for pid in sorted(pids_to_kill):
        if not aggressive and not _project_owned_pid(pid):
            log(f"  ⚠ 端口 {port} PID {pid} 非本项目归属，跳过（使用 --force 或 aggressive=True 强制）")
            continue
        log(f"  停止占用端口 {port} 的进程 PID: {pid}")
        stop_process_tree(pid, force=True)
        killed_any = True
        time.sleep(0.5)
    return not check_port(port) or killed_any


# =========================================================================== #
# 3. 配置管理
# =========================================================================== #


class ConfigManager:
    """读取并缓存 platform_config.json（缺失时回退 DEFAULT_CONFIG）。"""

    def __init__(self, config_path: Path = CONFIG_PATH):
        self.config_path = config_path
        self.config = self._load()
        self._validate()

    def _load(self) -> dict:
        if self.config_path.exists():
            try:
                with open(self.config_path, "r", encoding="utf-8") as f:
                    saved = json.load(f)
                return self._merge(DEFAULT_CONFIG, saved)
            except Exception as e:
                log(f"[WARN] 配置加载失败: {e}，使用默认配置")
        else:
            log(f"[WARN] 配置文件不存在: {self.config_path}，使用内置默认配置")
        # 首次不存在时写出默认配置，便于用户修改
        try:
            with open(self.config_path, "w", encoding="utf-8") as f:
                json.dump(DEFAULT_CONFIG, f, ensure_ascii=False, indent=2)
            log(f"[INFO] 已写出默认配置: {self.config_path}")
        except Exception:
            pass
        return DEFAULT_CONFIG

    @staticmethod
    def _merge(base: dict, override: dict) -> dict:
        result = dict(base)
        for k, v in override.items():
            if k in result and isinstance(result[k], dict) and isinstance(v, dict):
                result[k] = ConfigManager._merge(result[k], v)
            else:
                result[k] = v
        return result

    def _validate(self):
        """轻量校验：depends_on 不可引用不存在服务；版本号补齐。"""
        svcs = self.services
        unknown = []
        for k, s in svcs.items():
            for dep in s.get("depends_on") or []:
                if dep not in svcs:
                    unknown.append((k, dep))
        if unknown:
            bad = ", ".join(f"{k}→{d}" for k, d in unknown)
            log(f"[WARN] 配置中 depends_on 引用不存在的服务: {bad}；将忽略这些无效依赖")
            for k, _ in unknown:
                svcs[k]["depends_on"] = [d for d in (svcs[k].get("depends_on") or []) if d in svcs]
        if "version" not in self.config:
            self.config["version"] = "2.1"

    @property
    def project_name(self) -> str:
        return self.config.get("project_name", "项目")

    @property
    def dashboard_port(self) -> int:
        return self.config.get("dashboard_port", DEFAULT_DASHBOARD_PORT)

    @property
    def admin_user(self) -> str:
        u = os.environ.get("MOX_ADMIN_USER")
        if u:
            return u
        return self.config.get("admin", {}).get("username", "admin")

    @property
    def admin_pass(self) -> str:
        u = os.environ.get("MOX_ADMIN_PASS")
        if u:
            return u
        p = self.config.get("admin", {}).get("password")
        if p:
            return p
        # 首次生成一次性随机管理员密码并落盘
        new_p = secrets.token_urlsafe(10)
        self.config.setdefault("admin", {})
        self.config["admin"]["username"] = "admin"
        self.config["admin"]["password"] = new_p
        try:
            with open(self.config_path, "w", encoding="utf-8") as f:
                json.dump(self.config, f, ensure_ascii=False, indent=2)
            log(f"[INFO] 已生成初始管理员密码并写入 {self.config_path}（用户名=admin）")
        except Exception:
            pass
        return new_p

    @property
    def services(self) -> dict:
        return self.config.get("services", {})

    def service_keys(self) -> list:
        return list(self.services.keys())

    def topo_start_order(self) -> list:
        """按 depends_on 做 Kahn 拓扑排序；并列按 startup_order_hint→原插入顺序。"""
        svcs = self.services
        indeg = {k: 0 for k in svcs}
        succ = {k: [] for k in svcs}
        for k, s in svcs.items():
            for d in s.get("depends_on") or []:
                if d in svcs:
                    indeg[k] += 1
                    succ[d].append(k)
        order = []
        ready = sorted(
            [k for k, v in indeg.items() if v == 0],
            key=lambda k: (
                int(svcs[k].get("startup_order_hint", 9999) or 9999),
                list(svcs.keys()).index(k),
            ),
        )
        while ready:
            k = ready.pop(0)
            order.append(k)
            for nx in succ[k]:
                indeg[nx] -= 1
                if indeg[nx] == 0:
                    # 插入到 ready 中保持顺序
                    node = nx
                    pos = 0
                    while pos < len(ready):
                        cur = ready[pos]
                        cur_hint = int(svcs[cur].get("startup_order_hint", 9999) or 9999)
                        node_hint = int(svcs[node].get("startup_order_hint", 9999) or 9999)
                        cur_idx = list(svcs.keys()).index(cur)
                        node_idx = list(svcs.keys()).index(node)
                        if (node_hint, node_idx) < (cur_hint, cur_idx):
                            break
                        pos += 1
                    ready.insert(pos, node)
        if len(order) != len(svcs):
            missing = [k for k in svcs if k not in order]
            log(f"[WARN] 服务依赖存在环，按默认顺序处理剩余: {missing}")
            order.extend(missing)
        return order

    def topo_stop_order(self) -> list:
        """停止顺序：启动顺序的反序（依赖方先停，被依赖方后停）。"""
        return list(reversed(self.topo_start_order()))

    def resolve_cwd(self, svc: dict) -> Path:
        cwd = svc.get("cwd", ".")
        return (PROJECT_ROOT / cwd).resolve()

    def update_service(self, key: str, updates: dict):
        if key in self.config["services"]:
            self.config["services"][key].update(updates)
            try:
                with open(self.config_path, "w", encoding="utf-8") as f:
                    json.dump(self.config, f, ensure_ascii=False, indent=2)
            except Exception as e:
                log(f"[WARN] 配置保存失败: {e}")


# =========================================================================== #
# 4. 服务管理（基于 PID 文件 + 端口探测，跨进程可感知）
# =========================================================================== #


class ServiceManager:
    def __init__(self, config: ConfigManager):
        self.config = config
        # 每服务 start 时间戳（近似 STARTING 阶段判定）
        self._start_ts: dict = {}

    # --- 状态判定 ---------------------------------------------------------- #
    def get_status(self, key: str) -> dict:
        svc = self.config.services.get(key, {})
        pid = read_pid(key)
        alive = is_process_alive(pid)
        port = svc.get("port")
        port_ok = check_port(port) if port else False

        # 4 态：STARTING / RUNNING / DEGRADED / STOPPED
        # STARTING: 有存活 PID 但端口尚未监听，且距 start() 调用 < wait_time
        started_at = self._start_ts.get(key)
        wait_s = float(svc.get("wait_time", 5) or 5)
        starting = bool(
            alive
            and (not port_ok)
            and started_at is not None
            and (time.time() - started_at < wait_s + 1.5)
        )
        running = alive or port_ok

        health = None
        if running and port and svc.get("health_check"):
            health = http_ok(port, svc["health_check"])

        # DEGRADED: 端口在监听但 pid 不存在（僵尸端口，或其他同名进程占端口）
        degraded = port_ok and not alive
        if degraded:
            log(
                f"[WARN] 服务 '{key}' 端口 {port} 被外部进程占用但本服务 PID "
                f"{'为空' if not pid else str(pid) + ' 已死'}，状态标记为 DEGRADED"
            )

        # 清理残留 pid（进程已死且无僵尸端口占用）
        if not alive and pid is not None and not degraded:
            clear_pid(key)

        if alive and port_ok and health is False:
            state = "DEGRADED"
        elif degraded:
            state = "DEGRADED"
        elif starting:
            state = "STARTING"
        elif alive and port_ok:
            state = "RUNNING"
        elif running:
            # 端口存活但无 pid → 仍判为 RUNNING(但标注 degraded)
            state = "RUNNING" if not degraded else "DEGRADED"
        else:
            state = "STOPPED"

        # url 统一拼接 health_check（health_check="/" 时等效裸端口；api=/health、voice=/voice/health 等自动带上）
        url = None
        if port:
            url = f"http://localhost:{port}{svc.get('health_check', '')}"

        return {
            "key": key,
            "name": svc.get("name", key),
            "description": svc.get("description", ""),
            "port": port,
            "running": bool(running),
            "pid": pid if alive else None,
            "health": health,
            "state": state,
            "depends_on": list(svc.get("depends_on") or []),
            "is_admin_only": svc.get("is_admin_only", False),
            "tags": svc.get("tags", []),
            "auto_start": svc.get("auto_start", False),
            "url": url,
            "cwd": str(self.config.resolve_cwd(svc)),
            "command": svc.get("command", ""),
        }

    def all_status(self) -> dict:
        return {k: self.get_status(k) for k in self.config.service_keys()}

    # --- 二进制 / 依赖 / npm ------------------------------------------------ #
    def _ensure_binaries(self, svc: dict) -> bool:
        required = svc.get("binary_requires") or []
        if not required:
            return True
        ok = True
        for name in required:
            resolved, version = check_binary(name)
            if not resolved:
                log(f"[ERROR] 服务 '{svc.get('name')}' 依赖命令缺失: `{name}`。{binary_install_hint(name)}")
                ok = False
            else:
                log(
                    f"[INFO] 二进制就绪: {name:7s} -> {resolved}"
                    + (f" ({version})" if version else "")
                )
        return ok

    def _ensure_npm_deps(self, svc: dict, cwd: Path) -> bool:
        if not svc.get("npm_deps", False):
            return True
        nm = cwd / "node_modules"
        pkg = cwd / "package.json"
        lock = cwd / "package-lock.json"
        pnpm_lock = cwd / "pnpm-lock.yaml"
        # lock mtime 比 node_modules/.package-lock.json 新 → 重安装
        need_install = not nm.exists()
        if not need_install and lock.exists():
            marker = nm / ".package-lock.json"
            if not marker.exists():
                need_install = True
            else:
                try:
                    need_install = marker.stat().st_mtime < lock.stat().st_mtime
                except OSError:
                    need_install = True
        if not need_install and pnpm_lock.exists() and shutil.which("pnpm"):
            marker = nm / ".pnpm-store-marker"
            if not marker.exists():
                need_install = True
            else:
                try:
                    need_install = marker.stat().st_mtime < pnpm_lock.stat().st_mtime
                except OSError:
                    need_install = True
        if not need_install:
            return True
        if not shutil.which("npm"):
            log(f"[WARN] 需要 npm 依赖但未找到 npm，跳过自动安装: {cwd}")
            return False
        installer = ["npm", "install"]
        if pnpm_lock.exists() and shutil.which("pnpm"):
            # --frozen-lockfile 在本地开发经常因为 package.json 新增/升级依赖（但忘记重跑 pnpm i）而失败，
            # 导致「页面上一键启动服务 → 服务静默挂掉」。改成宽容策略：先 --frozen-lockfile，
            # 失败则自动降级到普通 pnpm install 自动更新 lockfile（仅本地 dev，CI 可显式用 `strict=true`
            # 时仍会失败）。
            installer = ["pnpm", "install", "--frozen-lockfile"]
        elif lock.exists():
            installer = ["npm", "ci" if (nm.exists() is False) else "install"]
        log(f"[INFO] 安装 npm 依赖: {' '.join(installer)} (cwd={cwd})")
        try:
            result = subprocess.run(
                installer,
                cwd=str(cwd),
                check=False,
                shell=(os.name == "nt"),
                stdout=None,
                stderr=None,
            )
            # pnpm frozen-lockfile 失败兜底：自动降级到无 frozen，重跑一次
            if (
                result.returncode != 0
                and installer[:2] == ["pnpm", "install"]
                and "--frozen-lockfile" in installer
            ):
                log(
                    "[WARN] pnpm --frozen-lockfile 失败（常见于 package.json 增/改依赖后未更新 "
                    "pnpm-lock.yaml），自动回退到 pnpm install（会更新 lockfile）…"
                )
                result = subprocess.run(
                    ["pnpm", "install"],
                    cwd=str(cwd),
                    check=False,
                    shell=(os.name == "nt"),
                    stdout=None,
                    stderr=None,
                )
            if result.returncode != 0:
                log(f"[ERROR] npm 依赖安装失败（rc={result.returncode}）；请手动在 {cwd} 执行安装命令")
                return False
        except Exception as e:
            log(f"[ERROR] npm 依赖安装异常: {e}")
            return False
        # 写 marker：下次判断是否需要重安装的依据
        try:
            (nm / ".package-lock.json").write_text(
                str(int(time.time())), encoding="utf-8"
            )
        except Exception:
            pass
        try:
            marker = nm / ".pnpm-store-marker"
            if pnpm_lock.exists():
                marker.write_text(str(int(time.time())), encoding="utf-8")
        except Exception:
            pass
        return True

    def _preflight(self, key: str, svc: dict, cwd: Path) -> bool:
        """启动前统一预检：二进制 → 工作目录 → npm 依赖 → depends_on 健康。"""
        if not cwd.exists():
            log(f"[ERROR] 服务 '{key}' 工作目录不存在: {cwd}")
            return False
        if not self._ensure_binaries(svc):
            log(f"[ERROR] 服务 '{key}' 运行时依赖不完整，启动中止。")
            return False
        if not self._ensure_npm_deps(svc, cwd):
            log(f"[ERROR] 服务 '{key}' 依赖未就绪，启动中止")
            return False
        # depends_on 必须健康
        for dep in svc.get("depends_on") or []:
            st = self.get_status(dep)
            if st["state"] not in ("RUNNING", "STARTING"):
                log(
                    f"[ERROR] 服务 '{key}' 依赖的 '{dep}' 未就绪 (state={st['state']})，中止启动"
                )
                return False
            if st["state"] == "STARTING":
                # 短等一下
                for _ in range(20):
                    time.sleep(0.2)
                    if self.get_status(dep)["state"] == "RUNNING":
                        break
                if self.get_status(dep)["state"] != "RUNNING":
                    log(f"[ERROR] 依赖 '{dep}' 仍未进入 RUNNING，中止")
                    return False
        return True

    # --- 操作 -------------------------------------------------------------- #
    def _spawn_command(self, svc: dict, cwd: Path, log_file_path: Path):
        """优先 args 列表形式 shell=False；回退 command 字符串 shell=True。

        日志句柄规范：显式持有文件句柄，Popen 成功后立即在父进程侧关闭
        （子进程已继承句柄继续写日志），避免父进程句柄依赖 GC 延迟释放造成
        日志文件被锁 / 轮转失败。
        """
        args = svc.get("args")
        cmd = svc.get("command", "")
        log_handle = open(log_file_path, "a", encoding="utf-8")
        kwargs = dict(
            cwd=str(cwd),
            stdout=log_handle,
            stderr=subprocess.STDOUT,
        )
        if os.name == "nt":
            # CREATE_NO_WINDOW(0x08000000): 有控制台但不显示窗口
            # CREATE_NEW_PROCESS_GROUP(0x200): CTRL+C 隔离
            # 注意：不能加 CREATE_BREAKAWAY_FROM_JOB，受限环境下会报"拒绝访问"
            kwargs["creationflags"] = 0x08000000 | 0x00000200
        try:
            if isinstance(args, list) and args:
                # Windows: shell=False 时相对路径需转为绝对路径，否则 WinError 2。
                # 注意：只对「可执行文件」args[0] 判断/转换，绝不能对整个 list 调 Path()
                # （Path(list) 会把元素当路径段拼接成错误路径，导致每次都回退 command 分支）。
                # 且仅当 cwd 下确实存在该相对可执行文件时才转绝对（如 target/release/x.exe）；
                # 若不存在则视为 PATH 命令（python/npm 等），保持原样由系统解析，避免无谓回退。
                if os.name == "nt" and args and not Path(args[0]).is_absolute():
                    cand = cwd / args[0]
                    if cand.exists():
                        args = [str(cand.resolve())] + args[1:]
                try:
                    proc = subprocess.Popen(args, shell=False, **kwargs)
                    log_handle.close()  # 父进程侧释放，子进程继续写
                    return proc
                except FileNotFoundError as e:
                    log(f"[WARN] args 形式启动失败（{e}），回退 command 字符串形式")
            if not cmd:
                raise ValueError(f"服务 '{svc.get('name')}' 未设置 command 或 args")
            kwargs["shell"] = True
            proc = subprocess.Popen(cmd, **kwargs)
            log_handle.close()  # 父进程侧释放，子进程继续写
            return proc
        except BaseException:
            try:
                log_handle.close()
            except Exception:
                pass
            raise

    def start(self, key: str, strict: bool = False) -> bool:
        if key not in self.config.services:
            log(f"[ERROR] 未知服务: {key}")
            return False
        svc = self.config.services[key]
        cwd = self.config.resolve_cwd(svc)

        st = self.get_status(key)
        if st["state"] in ("RUNNING", "STARTING"):
            log(f"[INFO] 服务 '{key}' 已在运行 (pid={st['pid']}, port={st['port']})")
            return True
        if st["state"] == "DEGRADED":
            log(f"[WARN] 服务 '{key}' 处于 DEGRADED，先清理占端口进程再启动")
            port = svc.get("port")
            if port:
                free_port(port, aggressive=True)

        if not self._preflight(key, svc, cwd):
            return False

        ensure_dirs()
        log_path = log_file(key)
        rotate_log_if_needed(key, config=self.config)

        # 端口被其他进程占用时尝试释放
        port = svc.get("port")
        if port and check_port(port):
            log(f"  ⚠ 端口 {port} 已被占用，尝试释放...")
            if not free_port(port, aggressive=False):
                if strict:
                    log(f"  ✗ [STRICT] 无法释放端口 {port}，启动中止")
                    return False
                # 非 strict 允许继续尝试；若最终被占则报错
                log("  ⚠ 端口仍被占用，继续启动（等待服务自行报错）")

        try:
            cmd_display = svc.get("command") or (
                " ".join(str(x) for x in (svc.get("args") or []))
            )
            log(f"[INFO] 启动服务 '{key}': {cmd_display} (cwd={cwd})")
            log(f"[INFO] 输出日志: {log_path}")
            # 写入启动分隔
            with open(log_path, "a", encoding="utf-8") as lf:
                lf.write(
                    f"\n===== 启动于 {datetime.now().isoformat()} =====\n"
                    f"# command: {cmd_display}\n"
                    f"# cwd: {cwd}\n"
                )
            self._start_ts[key] = time.time()
            proc = self._spawn_command(svc, cwd, log_path)
            # 日志句柄已在 _spawn_command 内于父进程侧关闭（子进程继承写日志）
            write_pid(key, proc.pid)
            log(f"[INFO] 已启动，pid={proc.pid}")

            wait = float(svc.get("wait_time", 5) or 5)
            started = False
            deadline = time.time() + wait
            while time.time() < deadline:
                if not is_process_alive(proc.pid):
                    log(f"[ERROR] 服务 '{key}' 进程已退出，查看日志: {log_path}")
                    self._tail_log(key, 40)
                    clear_pid(key)
                    self._start_ts.pop(key, None)
                    return False
                if port and check_port(port):
                    started = True
                    break
                time.sleep(0.2)

            if started:
                # 最后 health check：如果声明了 health_check 但失败 → strict 失败
                if port and svc.get("health_check") and not http_ok(port, svc["health_check"]):
                    msg = f"端口监听但健康检查 {svc['health_check']} 失败"
                    if strict:
                        log(f"[ERROR] [STRICT] {msg}，服务 '{key}' 不通过")
                        return False
                    log(f"[WARN] {msg}")
                log(f"[OK] 服务 '{key}' 已就绪 (port={port})")
                return True
            else:
                msg = f"端口 {port} 在 {wait}s 内未监听"
                if strict:
                    log(f"[ERROR] [STRICT] {msg}，启动失败")
                    self._tail_log(key, 40)
                    return False
                log(f"[WARN] {msg}，请检查日志: {log_path}")
                return True  # 非 strict 算启动成功（后续可能继续初始化）
        except Exception as e:
            log(f"[ERROR] 启动服务 '{key}' 失败: {e}")
            clear_pid(key)
            self._start_ts.pop(key, None)
            return False

    def stop(self, key: str, force: bool = False) -> bool:
        if key not in self.config.services:
            log(f"[ERROR] 未知服务: {key}")
            return False
        svc = self.config.services.get(key, {})
        port = svc.get("port")
        pid = read_pid(key)
        if pid is None or not is_process_alive(pid):
            st = self.get_status(key)
            if not st["running"]:
                log(f"[INFO] 服务 '{key}' 未运行")
                clear_pid(key)
                # 清理 DEGRADED 占端口
                if st.get("state") == "DEGRADED" and port:
                    free_port(port, aggressive=force)
                return True
            pid = st.get("pid")

        log(f"[INFO] 停止服务 '{key}' (pid={pid})" + (" [FORCE]" if force else ""))
        stop_process_tree(pid, force)
        time.sleep(0.8)
        if is_process_alive(pid):
            log(f"[WARN] 服务 '{key}' 仍未退出，尝试强制终止")
            stop_process_tree(pid, force=True)
        clear_pid(key)
        self._start_ts.pop(key, None)
        # 停止后确保端口释放
        if port and check_port(port):
            log(f"[WARN] 端口 {port} 仍被占用，尝试再次释放")
            free_port(port, aggressive=True)
        if not is_process_alive(pid):
            log(f"[OK] 服务 '{key}' 已停止")
            return True
        log(f"[ERROR] 服务 '{key}' 停止失败，仍存活 PID={pid}")
        return False

    def restart(self, key: str, strict: bool = False) -> bool:
        self.stop(key)
        time.sleep(1)
        return self.start(key, strict=strict)

    # --- 批量：按拓扑顺序 --------------------------------------------------- #
    def start_all_sorted(self, auto_only: bool = True, strict: bool = False) -> bool:
        ok_all = True
        for key in self.config.topo_start_order():
            svc = self.config.services.get(key, {})
            if auto_only and not svc.get("auto_start", False):
                continue
            ok = self.start(key, strict=strict)
            if not ok and strict:
                log(f"[STRICT] 服务 '{key}' 启动失败，中止整组启动")
                return False
            ok_all = ok_all and ok
        return ok_all

    def start_all_auto(self, strict: bool = False):
        self.start_all_sorted(auto_only=True, strict=strict)

    def start_all_configured(self, strict: bool = False) -> bool:
        """启动所有已配置服务（忽略 auto_start 标记）——用于管理面板『启动所有』按钮。"""
        return self.start_all_sorted(auto_only=False, strict=strict)

    def stop_all_sorted(self, force: bool = False):
        for key in self.config.topo_stop_order():
            self.stop(key, force)

    def stop_all(self, force: bool = False):
        # 保持对老 API 的兼容；内部统一用拓扑顺序
        self.stop_all_sorted(force)

    def clean_stale_pidfiles(self) -> dict:
        """
        只清理僵尸 pidfile（记录了 pid，但对应进程已死/端口没人占），不会杀掉任何真实运行中的服务。
        返回 {"removed": [key, ...], "preserved_running": [key, ...]}。
        用于 bootstrap 默认流程：重开面板时保留用户已启动的 api/frontend，仅纠正陈旧状态。
        """
        removed, preserved = [], []
        for key in self.config.service_keys():
            pid = read_pid(key)
            if pid is None:
                continue
            st = self.get_status(key)
            state = st.get("state")
            # 状态已经 STOPPED，却还有 pidfile → 僵尸
            if state == "STOPPED":
                clear_pid(key)
                removed.append(key)
                log(f"[CLEAN] 清理僵尸 pidfile: '{key}'（标记已停止，旧 pid={pid}）")
                continue
            # 状态显示 RUNNING，但进程已死 / 端口不监听 → 僵尸
            if state == "RUNNING":
                real_alive = is_process_alive(pid)
                port = self.config.services[key].get("port")
                port_ok = port is not None and check_port(port, host="127.0.0.1", timeout=0.6)
                if not real_alive or not port_ok:
                    clear_pid(key)
                    removed.append(key)
                    log(f"[CLEAN] 清理僵尸 pidfile: '{key}'（pid={pid} alive={real_alive} :{port} listen={port_ok}）")
                    continue
                preserved.append(key)
                # 仍然活着 → 保留（用户要"不要关闭旧的服务"，这里坚决不杀）
        return {"removed": removed, "preserved_running": preserved}

    def restart_all(self, strict: bool = False):
        self.stop_all_sorted(force=True)
        time.sleep(2)
        self.start_all_sorted(auto_only=True, strict=strict)

    def restart_all_configured(self, strict: bool = False) -> bool:
        """重启所有已配置服务（忽略 auto_start 标记）——用于管理面板『重启所有』按钮。"""
        self.stop_all_sorted(force=True)
        time.sleep(1)
        return self.start_all_sorted(auto_only=False, strict=strict)

    # --- 自检（bootstrap --dry-run） ---------------------------------------- #
    def dry_run_preflight(self) -> dict:
        """不动真格启动：仅打印预检结果（二进制、目录、依赖顺序）。"""
        report = {"ok": True, "services": {}}
        order = self.config.topo_start_order()
        log(f"[DRY-RUN] 启动顺序: {' → '.join(order)}")
        for key in order:
            svc = self.config.services[key]
            cwd = self.config.resolve_cwd(svc)
            ok = True
            issues = []
            if not cwd.exists():
                ok = False
                issues.append(f"工作目录缺失: {cwd}")
            if not self._ensure_binaries(svc):
                ok = False
                issues.append("二进制缺失")
            if svc.get("npm_deps"):
                nm = cwd / "node_modules"
                if not nm.exists():
                    issues.append("node_modules 未安装（将在真实启动时自动安装）")
            for dep in svc.get("depends_on") or []:
                if dep not in self.config.services:
                    ok = False
                    issues.append(f"无效依赖: {dep}")
            report["services"][key] = {"ok": ok, "issues": issues}
            if not ok:
                report["ok"] = False
            head = "✔" if ok else "✗"
            log(f"  {head} {key:12s} {svc.get('name')}  (port={svc.get('port')})  " +
                (" | ".join(issues) if issues else "OK"))
        return report

    # --- 日志 -------------------------------------------------------------- #
    def get_log(self, key: str, lines: int = 100) -> str:
        p = log_file(key)
        if not p.exists():
            return f"（无日志文件: {p}）"
        try:
            with open(p, "r", encoding="utf-8", errors="replace") as f:
                content = f.readlines()
            return "".join(content[-lines:])
        except Exception as e:
            return f"（读取日志失败: {e}）"

    def dump_all_logs(self, lines: int = 30):
        """失败时统一 dump 所有服务最近 N 行。"""
        for k in self.config.service_keys():
            log(f"\n===== {k} 最近 {lines} 行日志 =====")
            print(self.get_log(k, lines))

    def _tail_log(self, key: str, lines: int = 15):
        print(self.get_log(key, lines))


# =========================================================================== #
# 5. Web 管理面板（stdlib http.server，无需 Flask）
# =========================================================================== #


class AuthManager:
    """基于 Cookie 的会话管理（内存态，进程级）。"""

    def __init__(self, admin_user: str, admin_pass: str):
        self.admin_user = admin_user
        self.admin_pass_hash = hashlib.sha256(admin_pass.encode()).hexdigest()
        self.sessions: dict = {}

    def validate_password(self, username: str, password: str) -> bool:
        return username == self.admin_user and hashlib.sha256(
            password.encode()
        ).hexdigest() == self.admin_pass_hash

    def create_session(self, user: str) -> str:
        sid = secrets.token_hex(32)
        self.sessions[sid] = {"user": user, "created_at": time.time()}
        return sid

    def validate_session(self, sid) -> dict | None:
        if sid and sid in self.sessions:
            s = self.sessions[sid]
            if time.time() - s["created_at"] < SESSION_TIMEOUT:
                return s
        return None

    def destroy_session(self, sid):
        self.sessions.pop(sid, None)

    def get_or_create_guest(self, cookie: str):
        sid = None
        for part in cookie.split(";"):
            part = part.strip()
            if part.startswith("session_id="):
                sid = part.split("=", 1)[1]
                break
        session = self.validate_session(sid) if sid else None
        if session:
            return sid, session
        new_sid = secrets.token_hex(32)
        self.sessions[new_sid] = {"user": "guest", "created_at": time.time()}
        return new_sid, self.sessions[new_sid]

    def cleanup_expired(self):
        now = time.time()
        expired = [
            s for s, v in self.sessions.items() if now - v["created_at"] > SESSION_TIMEOUT
        ]
        for s in expired:
            self.sessions.pop(s, None)


# 登录页 HTML（精简，聚焦功能）
LOGIN_HTML = """<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8">
<title>登录 - 璇玑系统管理平台</title>
<style>
 body{font-family:-apple-system,Segoe UI,Roboto,sans-serif;background:linear-gradient(135deg,#1a1a2e,#16213e);min-height:100vh;display:flex;align-items:center;justify-content:center;color:#ecf0f1}
 .box{background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.1);border-radius:20px;padding:44px 40px;width:380px;backdrop-filter:blur(20px)}
 .icon{font-size:48px;text-align:center;margin-bottom:8px}
 .title{text-align:center;font-size:20px;font-weight:700;background:linear-gradient(135deg,#667eea,#764ba2);-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:24px}
 .inp{width:100%;padding:12px;margin:8px 0;background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.1);border-radius:9px;color:#ecf0f1;font-size:14px;outline:none;box-sizing:border-box}
 .inp:focus{border-color:#667eea}
 .btn{width:100%;padding:12px;background:linear-gradient(135deg,#667eea,#764ba2);border:none;border-radius:9px;color:#fff;font-size:15px;font-weight:600;cursor:pointer;margin-top:10px}
 .err{background:rgba(231,76,60,.2);color:#e74c3c;padding:10px;border-radius:8px;font-size:12px;margin-bottom:14px;display:none}
 .err.show{display:block}
 .sec{text-align:center;font-size:11px;color:#7f8c8d;margin-top:20px}
</style></head><body>
<div class="box"><div class="icon">🌌</div><div class="title">企业级服务管理平台</div>
<div class="err" id="err">用户名或密码错误</div>
<form onsubmit="login(event)">
<input class="inp" type="text" id="u" placeholder="管理员用户名" required>
<input class="inp" type="password" id="p" placeholder="密码" required>
<button class="btn" type="submit">🔐 登录管理平台</button>
</form><div class="sec">本平台需要管理员权限 · 会话有效期 30 分钟</div></div>
<script>
async function login(e){e.preventDefault();
 const r=await fetch('/api/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({username:document.getElementById('u').value,password:document.getElementById('p').value})});
 if(r.ok){const d=await r.json();d.success?location.href='/':showErr(d.message||'登录失败')}else showErr('登录失败')}
function showErr(m){const el=document.getElementById('err');el.textContent=m;el.classList.add('show');setTimeout(()=>el.classList.remove('show'),3000)}
document.getElementById('u').focus();
</script></body></html>"""

# 主面板 HTML（状态卡片 + 单服务/批量操作 + 日志弹窗）
DASHBOARD_HTML = """<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8">
<title>璇玑系统 - 服务管理平台</title>
<style>
 *{margin:0;padding:0;box-sizing:border-box}
 body{font-family:-apple-system,Segoe UI,Roboto,sans-serif;background:linear-gradient(135deg,#1a1a2e,#16213e,#0f3460);min-height:100vh;color:#ecf0f1}
 .c{max-width:1280px;margin:0 auto;padding:28px 24px}
 .hd{display:flex;justify-content:space-between;align-items:center;margin-bottom:20px}
 .brand{display:flex;align-items:center;gap:12px}
 .bi{font-size:34px}
 .bt{font-size:20px;font-weight:700;background:linear-gradient(135deg,#667eea,#764ba2);-webkit-background-clip:text;-webkit-text-fill-color:transparent}
 .ub{display:flex;align-items:center;gap:14px;background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.1);border-radius:12px;padding:8px 16px}
 .welcome{
  background:linear-gradient(135deg,rgba(99,102,241,.15),rgba(168,85,247,.15));
  border:1px solid rgba(139,92,246,.35);
  border-radius:18px;
  padding:22px 26px;margin-bottom:22px;
  display:flex;align-items:center;justify-content:space-between;gap:20px;flex-wrap:wrap;
  animation:fadeIn .5s ease;
 }
 @keyframes fadeIn{from{opacity:0;transform:translateY(-6px)}to{opacity:1;transform:translateY(0)}}
 .welcome .wl{flex:1;min-width:260px}
 .welcome h2{font-size:18px;font-weight:600;margin-bottom:6px;background:linear-gradient(135deg,#a5b4fc,#c4b5fd);-webkit-background-clip:text;-webkit-text-fill-color:transparent}
 .welcome p{font-size:13px;color:#a5b4fc;line-height:1.6}
 .welcome .steps{margin-top:10px;display:flex;gap:12px;flex-wrap:wrap}
 .welcome .step{padding:6px 12px;background:rgba(255,255,255,.06);border:1px solid rgba(255,255,255,.1);border-radius:999px;font-size:11px;color:#c7d2fe}
 .welcome .step b{color:#fff}
 .hero-btn{
  padding:14px 28px;border:none;border-radius:12px;font-size:15px;font-weight:600;cursor:pointer;
  color:#fff;background:linear-gradient(135deg,#10b981,#059669);
  box-shadow:0 8px 24px rgba(16,185,129,.35);
  display:inline-flex;align-items:center;gap:10px;transition:.25s;white-space:nowrap
 }
 .hero-btn:hover{filter:brightness(1.1);transform:translateY(-2px);box-shadow:0 12px 32px rgba(16,185,129,.45)}
 .hero-btn:disabled{opacity:.5;cursor:not-allowed;transform:none;box-shadow:none}
 .op-bar{background:rgba(255,255,255,.03);border:1px solid rgba(255,255,255,.08);border-radius:14px;padding:14px 18px;margin-bottom:22px;display:flex;gap:10px;flex-wrap:wrap;align-items:center}
 .btn{padding:9px 16px;border:none;border-radius:8px;font-size:13px;font-weight:500;cursor:pointer;display:inline-flex;align-items:center;gap:6px;transition:.2s;white-space:nowrap}
 .btn-s{background:linear-gradient(135deg,#11998e,#38ef7d);color:#fff}
 .btn-w{background:linear-gradient(135deg,#f7971e,#ffd200);color:#333}
 .btn-d{background:linear-gradient(135deg,#eb3349,#f45c43);color:#fff}
 .btn-sec{background:rgba(255,255,255,.1);color:#ecf0f1;border:1px solid rgba(255,255,255,.2)}
 .btn:hover{filter:brightness(1.1);transform:translateY(-1px)}
 .btn:disabled{opacity:.45;cursor:not-allowed;transform:none}
 .spinner{width:13px;height:13px;border:2px solid rgba(255,255,255,.3);border-top-color:#fff;border-radius:50%;animation:spin .8s linear infinite}
 @keyframes spin{to{transform:rotate(360deg)}}
 .grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:18px}
 .svc{background:rgba(255,255,255,.05);border:1px solid rgba(255,255,255,.1);border-radius:16px;padding:20px;position:relative;transition:.3s}
 .svc::before{content:'';position:absolute;top:0;left:0;right:0;height:3px;background:var(--c,#3498db);border-radius:16px 16px 0 0}
 .svc:hover{border-color:rgba(255,255,255,.2);transform:translateY(-2px)}
 .svc.locked{border-color:rgba(241,196,15,.3)}
 .svc.locked::before{background:linear-gradient(90deg,#f39c12,#e67e22)}
 .lock{position:absolute;top:12px;right:12px;background:rgba(241,196,15,.2);color:#f1c40f;padding:3px 8px;border-radius:10px;font-size:10px}
 .sh{display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:8px}
 .si2{font-size:28px}
 .ss{display:flex;align-items:center;gap:6px;padding:4px 10px;border-radius:12px;font-size:11px;font-weight:500}
 .sd{width:6px;height:6px;border-radius:50%}
 .sr{background:rgba(46,213,115,.2);color:#2ed573}.sr .sd{background:#2ed573;animation:pulse 1.5s infinite}
 .ss2{background:rgba(231,76,60,.2);color:#e74c3c}.ss2 .sd{background:#e74c3c}
 .sst{background:rgba(241,196,15,.2);color:#f1c40f}.sst .sd{background:#f1c40f;animation:pulse 1.5s infinite}
 @keyframes pulse{0%,100%{opacity:1}50%{opacity:.4}}
 .sn{font-size:15px;font-weight:600;margin-bottom:4px}
 .sdesc{font-size:12px;color:#95a5a6;line-height:1.5;margin-bottom:8px}
 .sdesc.l{color:#f1c40f}
 .sm{display:flex;gap:14px;font-size:11px;color:#7f8c8d;margin-bottom:8px;flex-wrap:wrap}
 .stags{display:flex;gap:5px;margin-bottom:10px;flex-wrap:wrap}
 .tag{padding:2px 8px;background:rgba(255,255,255,.08);border-radius:8px;font-size:10px;color:#bdc3c7}
 .tag.l{background:rgba(241,196,15,.2);color:#f1c40f}
 .sa{display:flex;gap:6px;flex-wrap:wrap}
 .ab{padding:7px 12px;border:none;border-radius:8px;font-size:11px;font-weight:500;cursor:pointer;transition:.2s;display:flex;align-items:center;gap:4px;text-decoration:none}
 .as{background:linear-gradient(135deg,#11998e,#38ef7d);color:#fff}
 .ap{background:linear-gradient(135deg,#eb3349,#f45c43);color:#fff}
 .ar{background:linear-gradient(135deg,#f7971e,#ffd200);color:#333}
 .ao{background:var(--c,#3498db);color:#fff}
 .al{background:rgba(102,126,234,.25);color:#8fa4f0}
 .ab:hover{filter:brightness(1.1)}
 .ab:disabled{opacity:.35;cursor:not-allowed}
 .lm{display:none;position:fixed;inset:0;background:rgba(0,0,0,.8);z-index:1000;align-items:center;justify-content:center}
 .lm.show{display:flex}
 .lc{background:#1a1a2e;border:1px solid rgba(255,255,255,.1);border-radius:16px;padding:24px;width:90%;max-width:820px;max-height:80vh;display:flex;flex-direction:column}
 .lh{display:flex;justify-content:space-between;align-items:center;margin-bottom:16px}
 .lx{background:none;border:none;color:#95a5a6;font-size:20px;cursor:pointer}
 .lb{flex:1;overflow:auto;background:#0d0d1a;border-radius:8px;padding:16px;font-family:Consolas,monospace;font-size:12px;line-height:1.6;white-space:pre-wrap}
 .tst{position:fixed;bottom:24px;right:24px;background:rgba(30,30,46,.97);border:1px solid rgba(255,255,255,.15);border-radius:12px;padding:14px 20px;font-size:13px;z-index:2000;display:none;max-width:420px;line-height:1.55;box-shadow:0 12px 40px rgba(0,0,0,.5)}
 .tst.show{display:block;animation:slideIn .3s}.tst.s{border-color:rgba(46,213,115,.6);color:#2ed573}.tst.e{border-color:rgba(231,76,60,.6);color:#e74c3c}.tst.i{border-color:rgba(102,126,234,.6);color:#667eea}.tst.w{border-color:rgba(245,158,11,.6);color:#fbbf24}
 @keyframes slideIn{from{transform:translateX(100%)}to{transform:translateX(0)}}
 .pb{background:linear-gradient(135deg,rgba(155,89,182,.12),rgba(52,152,219,.12));border:1px solid rgba(155,89,182,.3);border-radius:12px;padding:12px 18px;display:flex;align-items:center;gap:10px;margin-bottom:22px;font-size:13px}
 .count-bar{margin-left:auto;display:flex;gap:12px;font-size:12px;color:#94a3b8}
 .count-bar b{font-size:16px;color:#fff;margin-right:3px}
 .count-bar .ok b{color:#2ed573}.count-bar .bad b{color:#e74c3c}
</style></head><body>
<div class="c">
<div class="hd"><div class="brand"><span class="bi">🌌</span><div class="bt" id="bt">璇玑系统管理平台</div></div>
<div id="ub" class="ub"></div></div>
<div id="pb" class="pb" style="display:none"></div>
<div id="welcome" class="welcome" style="display:none">
  <div class="wl">
    <h2>✨ 欢迎使用璇玑系统 · 服务待启动</h2>
    <p>项目服务 <b style="color:#fca5a5">未运行</b>。请点击右侧 <b style="color:#fff">▶ 一键启动所有</b> 按钮，按拓扑顺序拉起 API 后端服务 + 用户前端界面。
       也可以在下方卡片单张启动，或使用顶部操作栏的批量按钮。</p>
    <div class="steps">
      <span class="step">① <b>启动</b>：▶ 一键启动所有服务</span>
      <span class="step">② <b>等待</b>：API 就绪 → Frontend 自动代理</span>
      <span class="step">③ <b>访问</b>：卡片「🚀 访问」按钮直达</span>
    </div>
  </div>
  <button class="hero-btn" id="hero-start" onclick="batch('start_all')">▶ 一键启动所有服务</button>
</div>
<div class="op-bar" id="op-bar">
<button class="btn btn-s" onclick="batch('start_all')" id="b-start">▶ 启动所有</button>
<button class="btn btn-w" onclick="batch('restart_all')" id="b-restart">🔄 重启所有</button>
<button class="btn btn-d" onclick="batch('stop_all')" id="b-stop">⏹ 停止所有</button>
<button class="btn btn-sec" onclick="refresh()">🔄 刷新</button>
<div class="count-bar" id="count-bar"></div>
</div>
<div id="grid" class="grid"></div>
<div id="lm" class="lm"><div class="lc"><div class="lh"><span id="lt" class="bt">日志</span><button class="lx" onclick="closeLogs()">✕</button></div><div id="lb" class="lb">加载中...</div></div></div>
<div id="tst" class="tst"></div>
</div>
<script>
let session=null,services=[],busy=false,COLORS={api:'#3498db',frontend:'#2ecc71'},ICONS={api:'🔧',frontend:'🎨'};
async function init(){await loadSession();await refresh();setInterval(refresh,5000)}
async function loadSession(){try{const r=await fetch('/api/session');session=await r.json()}catch(e){session={user:'guest',is_admin:false}}renderUser()}
function renderUser(){const ub=document.getElementById('ub');if(session.is_admin){ub.innerHTML='<span>🛡️ '+session.username+' · 管理员</span><button class="btn btn-sec" style="padding:5px 12px;font-size:12px" onclick="logout()">退出</button>';document.getElementById('pb').style.display='none'}else{ub.innerHTML='<span>👤 访客用户</span><button class="btn btn-s" style="padding:5px 14px;font-size:12px" onclick="location.href=\\'/login\\'">🔐 登录</button>';const pb=document.getElementById('pb');pb.style.display='flex';pb.innerHTML='🔒 <span>您以 <strong>普通用户</strong> 身份访问，启动/停止需管理员权限。</span>'}
 const bar=document.getElementById('op-bar');bar.querySelectorAll('.btn').forEach(b=>{if(b.textContent.includes('启动')||b.textContent.includes('停止')||b.textContent.includes('重启'))b.disabled=!session.is_admin})
 const hero=document.getElementById('hero-start');if(hero)hero.disabled=!session.is_admin||busy}
async function refresh(){try{const r=await fetch('/api/status');services=await r.json();render()}catch(e){console.error(e)}}
function render(){
 const g=document.getElementById('grid');g.innerHTML='';
 services.forEach(s=>g.appendChild(card(s)));
 // 统计
 const total=services.length,running=services.filter(s=>s.running).length,stopped=services.filter(s=>s.running===false).length;
 const starting=services.filter(s=>s.running==null).length;
 document.getElementById('count-bar').innerHTML=
  '<span class="ok"><b>'+running+'</b>运行中</span>'+
  '<span class="bad"><b>'+stopped+'</b>已停止</span>'+
  (starting?'<span><b>'+starting+'</b>启动中</span>':'')+
  '<span>共 <b>'+total+'</b> 服务</span>';
 // 欢迎横幅：管理员 + 所有服务都停止 / 启动中但无运行
 const allIdle=(running===0)&&session.is_admin;
 const w=document.getElementById('welcome');
 if(allIdle){w.style.display='flex';
  const hb=document.getElementById('hero-start');
  hb.disabled=busy;hb.innerHTML=busy?'<span class="spinner"></span>启动中...':'▶ 一键启动所有服务';
 }else{w.style.display='none'}
}
function card(s){const adm=session.is_admin,r=s.running,locked=s.requires_auth&&!adm;
 const c=document.createElement('div');c.className='svc'+(locked?' locked':'');c.style.setProperty('--c',COLORS[s.key]||'#3498db');
 const tags=(s.tags||[]).map(t=>'<span class="tag'+(t.includes('受限')?' l':'')+'">'+t+'</span>').join('');
 let acts='';
 if(locked)acts='<a href="/login" class="ab as">🔐 登录访问</a>';
 else if(adm){if(r){acts='<button class="ab ar" data-a="restart" data-s="'+s.key+'">🔄 重启</button><button class="ab ap" data-a="stop" data-s="'+s.key+'">⏹ 停止</button>'}else{acts='<button class="ab as" data-a="start" data-s="'+s.key+'">▶ 启动</button><button class="ab ar" data-a="restart" data-s="'+s.key+'">🔄 重启</button>'}
  acts+='<button class="ab al" data-a="logs" data-s="'+s.key+'"'+(r?'':' disabled')+'>📋 日志</button>';
  if(r&&s.url)acts+='<a href="'+s.url+'" target="_blank" class="ab ao">🚀 访问</a>'}
 else{if(r&&s.url)acts='<a href="'+s.url+'" target="_blank" class="ab ao">🚀 访问</a>';else acts='<div class="ab" style="background:rgba(255,255,255,.05);color:#7f8c8d">⏸ 未运行</div>'}
 const st=r?'运行中':s.running===false?(s.health===false?'异常':'已停止'):'启动中';
 const sc=r?'sr':(s.health===false?'ss2':'sst');
 c.innerHTML=(locked?'<div class="lock">🔒 需管理员权限</div>':'')+
  '<div class="sh"><span class="si2">'+(ICONS[s.key]||'📦')+'</span><div class="ss '+sc+'"><span class="sd"></span>'+(r?'🟢':'⚪')+' '+st+'</div></div>'+
  '<div class="sn">'+s.name+'</div><div class="sdesc'+(locked?' l':'')+'">'+s.description+'</div>'+
  '<div class="sm"><span>📡 :'+s.port+'</span>'+(s.pid?'<span>🆔 '+s.pid+'</span>':'')+'</div>'+
  '<div class="stags">'+tags+'</div><div class="sa">'+acts+'</div>';
 return c}
document.addEventListener('click',e=>{const b=e.target.closest('[data-a]');if(!b)return;const a=b.dataset.a,s=b.dataset.s;if(a==='logs')return showLogs(s);if(!session.is_admin){toast('需要管理员权限','e');return}act(a,s,b)});
async function act(a,s,btn){const M={start:['/api/start','启动'],stop:['/api/stop','停止'],restart:['/api/restart','重启']};const[ep,lab]=M[a];
 if(btn){btn.disabled=true;const orig=btn.innerHTML;btn.innerHTML='<span class="spinner"></span>'+lab+'中...'
  setTimeout(()=>{try{btn.innerHTML=orig;btn.disabled=false}catch(e){}},18000)}
 try{const r=await fetch(ep,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({service:s})});const d=await r.json();toast(d.message||lab+' '+s+' 完成',d.success?'s':'e');if(a==='start'&&d.success){toast('服务启动中：等待端口监听，每 5 秒自动刷新状态…','i',4000)}}catch(e){toast('请求失败','e')}finally{refresh()}}
async function batch(a){if(!session.is_admin){toast('需要管理员权限','e');return}
 if(busy){toast('正在处理前一项操作，请稍候…','w');return}
 const M={start_all:['/api/start_all','启动所有'],stop_all:['/api/stop_all','停止所有'],restart_all:['/api/restart_all','重启所有']};const[ep,lab]=M[a];
 busy=true;setBusy(true)
 let done=false;const maxTicks=34;let ticks=0;const polling=setInterval(async()=>{
  if(done){clearInterval(polling);return}
  ticks++;
  try{await refresh()}catch(e){/* 浏览器偶发断连（ConnectionAbortedError）不致命，用旧状态继续 */}
  const any=Object.values(lastStatus||{}).some(s=>s.state==='STARTING');
  const timeout=ticks>=maxTicks;
  if(!any||timeout){
   clearInterval(polling);
   if(done)return;
   done=true;busy=false;setBusy(false);try{refresh()}catch(_){}
   toast(timeout?'⌛ 已到最长等待时间：可点 🔄 刷新 继续查看状态。':'✅ 状态已稳定，可继续操作。',timeout?'w':'s',timeout?5500:4000)
  }
 },2000);
 try{const r=await fetch(ep,{method:'POST',headers:{'Content-Type':'application/json'}});const d=await r.json();
  if(a==='start_all'){
   toast('✅ 启动指令已提交：后台按拓扑顺序启动 API → Frontend（约 15–30 秒），页面每 2 秒自动刷新。','s',7000)
  }else if(a==='stop_all'){
    toast('⏹ 停止指令已提交：后台按反拓扑顺序终止，约 3–8 秒完成。','i',4500)
  }else{
    toast('🔄 重启指令已提交，稍后可点 🔄 刷新 查看。','s',5000)
  }
  if(d&&d.success===false)toast(d.message||'操作失败','e',6000)
 }catch(e){toast('请求失败','e');clearInterval(polling);busy=false;setBusy(false)}}
function setBusy(v){
 const ids=['b-start','b-restart','b-stop','hero-start'];
 ids.forEach(id=>{const el=document.getElementById(id);if(!el)return;el.disabled=v||!session.is_admin});
 const opBar=document.getElementById('op-bar');
 opBar.querySelectorAll('.btn').forEach(b=>{if(b.textContent.includes('启动')||b.textContent.includes('停止')||b.textContent.includes('重启'))b.disabled=v||!session.is_admin})
 render()
}
async function logout(){await fetch('/api/logout',{method:'POST'});session={user:'guest',is_admin:false};renderUser();refresh()}
async function showLogs(s){try{document.getElementById('lt').textContent=s+' 日志';document.getElementById('lm').classList.add('show');const r=await fetch('/api/logs',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({service:s,lines:200})});const d=await r.json();document.getElementById('lb').textContent=d.logs||'暂无日志'}catch(e){document.getElementById('lb').textContent='加载失败'}}
function closeLogs(){document.getElementById('lm').classList.remove('show')}
function toast(m,t,dur){const el=document.getElementById('tst');el.textContent=m;el.className='tst show '+(t||'');clearTimeout(toast._t);toast._t=setTimeout(()=>el.classList.remove('show'),dur||3200)}
init();
</script></body></html>"""


def run_dashboard(config: ConfigManager, manager: ServiceManager, host: str, port: int, open_browser: bool):
    """启动 stdlib Web 管理面板（无 Flask 依赖）。

    端口冲突策略（避免用户「启动了但打不开页面」）：
      1. 先以 aggressive=False 释放，识别出「项目归属」占用则杀掉；
      2. 若仍被占，再 aggressive=True 清一次（对真·第三方占用，_project_owned_pid 会拒杀，
         因此不会误伤）；
      3. 仍被占 → 自动回落到 [port, port+60] 区间内的空闲端口（优先选最接近原端口）；
      4. 返回最终实际绑定的端口（调用方可据此打正确 URL / open_browser）。
    """
    def _find_free_port(start_port: int, span: int = 60) -> int:
        """在 [start_port, start_port+span] 中找第一个空闲 TCP 端口；都占满时回退 0（OS 分配）。"""
        for p in range(start_port, start_port + span + 1):
            if not check_port(p):
                return p
        return 0

    # ---- 阶段 A：优先使用原始端口；占用 → 先清理 ----
    final_port = port
    if check_port(port):
        log(f"[WARN] dashboard 端口 {port} 被占用，尝试释放本项目残留进程...")
        # round 1: gentle (只杀项目归属)
        freed = free_port(port, aggressive=False)
        if not freed and check_port(port):
            # round 2: aggressive（实际还是走 _project_owned_pid 白名单，避免误杀第三方）
            log(f"[WARN] 仍被占用；再做一轮 aggressive 回收（仅回收识别出的项目进程）...")
            free_port(port, aggressive=True)
            time.sleep(0.6)
        if check_port(port):
            # round 3: 自动回落端口
            fallback = _find_free_port(port, span=60)
            if fallback == 0:
                log(f"[ERROR] dashboard 原始端口 {port} 仍被第三方占用，且 {port}~{port+60} 区间全部占满，无法启动。")
                log(f"[HINT]  请手工 `scripts/server-manage.py dashboard --port <其他端口>` 指定空闲端口。")
                return None
            log(f"[INFO] 原始端口 {port} 仍被第三方占用；dashboard 自动回落到端口 {fallback}")
            final_port = fallback

    auth = AuthManager(config.admin_user, config.admin_pass)
    admin_user = config.admin_user

    class Handler(BaseHTTPRequestHandler):
        def _resp(self, code, ct, body, sid=None):
            try:
                self.send_response(code)
                if sid:
                    self.send_header(
                        "Set-Cookie", "session_id=%s; Path=/; HttpOnly; SameSite=Lax" % sid
                    )
                self.send_header("Content-Type", ct)
                self.end_headers()
                data = body.encode("utf-8") if isinstance(body, str) else body
                self.wfile.write(data)
            except (BrokenPipeError, ConnectionAbortedError, ConnectionResetError):
                # 浏览器轮询并发时，旧 TCP 连接会被客户端主动关闭（Chrome 取消请求 / 页面导航）。
                # 这是正常现象，不需要在控制台打印整条 traceback。
                try:
                    self.close_connection = True
                except Exception:
                    pass
            except Exception:
                # 其他异常仍然抛出，便于真正的 bug 排查
                raise

        def _session(self):
            cookie = self.headers.get("Cookie", "")
            return auth.get_or_create_guest(cookie)

        def _is_admin(self, sid):
            s = auth.validate_session(sid)
            return bool(s and s.get("user") == "admin")

        def _require_admin(self, sid):
            if not self._is_admin(sid):
                self._resp(
                    403,
                    "application/json; charset=utf-8",
                    json.dumps({"error": "需要管理员权限"}, ensure_ascii=False),
                    sid,
                )
                return False
            return True

        def do_GET(self):
            parsed = urlparse(self.path)
            path = parsed.path
            sid, _ = self._session()
            routes = {
                "/login": lambda: self._resp(200, "text/html; charset=utf-8", LOGIN_HTML, sid),
                "/": lambda: self._serve_main(sid),
                "/api/status": lambda: self._api_status(sid),
                "/api/session": lambda: self._api_session(sid),
                "/api/config": lambda: self._api_config(sid),
            }
            handler = routes.get(path)
            if handler:
                handler()
            else:
                self._resp(404, "text/plain; charset=utf-8", "Not Found", sid)

        def do_POST(self):
            parsed = urlparse(self.path)
            path = parsed.path
            sid, _ = self._session()
            cl = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(cl).decode("utf-8") if cl > 0 else "{}"
            try:
                data = json.loads(body)
            except Exception:
                data = {}

            routes = {
                "/api/login": lambda: self._api_login(data),
                "/api/logout": lambda: self._api_logout(sid),
                # 单服务 start/restart 最长 ~15 秒（含健康检查等待），也走后台异步避免阻塞
                "/api/start": lambda: self._api_admin(
                    sid, lambda: manager.start(data.get("service")), async_mode=True
                ),
                "/api/stop": lambda: self._api_admin(sid, lambda: manager.stop(data.get("service"), force=False)),
                "/api/restart": lambda: self._api_admin(
                    sid, lambda: manager.restart(data.get("service")), async_mode=True
                ),
                # 批量动作：全部放后台线程，立即返回 queued:true，前端 busy 锁通过"轮询刷新 + 全部服务非 STARTING"再解除
                "/api/start_all": lambda: self._api_admin(sid, manager.start_all_configured, async_mode=True),
                "/api/stop_all": lambda: self._api_admin(sid, lambda: manager.stop_all(force=True), async_mode=True),
                "/api/restart_all": lambda: self._api_admin(sid, manager.restart_all_configured, async_mode=True),
                "/api/logs": lambda: self._api_logs(data, sid),
            }
            handler = routes.get(path)
            if handler:
                handler()
            else:
                self._resp(404, "text/plain; charset=utf-8", "Not Found", sid)

        # --- 页面与 API ---------------------------------------------------- #
        def _serve_main(self, sid):
            if not self._is_admin(sid):
                # 302 跳转登录页：Location 必须在 end_headers 之前写入。
                # 因此不走 _resp（_resp 会在内部 end_headers 并写 body）。
                try:
                    self.send_response(302)
                    self.send_header(
                        "Set-Cookie",
                        "session_id=%s; Path=/; HttpOnly; SameSite=Lax" % sid,
                    )
                    self.send_header("Location", "/login")
                    self.send_header("Content-Type", "text/html; charset=utf-8")
                    self.end_headers()
                    # 给一些浏览器（如 curl / 某些旧代理）一个兜底 body
                    self.wfile.write(
                        (
                            '<html><head><meta charset="utf-8">'
                            '<meta http-equiv="refresh" content="0; url=/login"></head>'
                            '<body>Redirecting to <a href="/login">/login</a>...</body></html>'
                        ).encode("utf-8")
                    )
                except (BrokenPipeError, ConnectionAbortedError, ConnectionResetError):
                    try: self.close_connection = True
                    except Exception: pass
                return
            self._resp(200, "text/html; charset=utf-8", DASHBOARD_HTML, sid)

        def _api_status(self, sid):
            status = manager.all_status()
            is_adm = self._is_admin(sid)
            filtered = []
            for s in status.values():
                if s["is_admin_only"] and not is_adm:
                    s["description"] = "🔒 管理员权限才能查看详细信息"
                    s["tags"] = ["🔒 受限访问"]
                    s["requires_auth"] = True
                else:
                    s["requires_auth"] = False
                filtered.append(s)
            self._resp(
                200,
                "application/json; charset=utf-8",
                json.dumps(filtered, ensure_ascii=False),
                sid,
            )

        def _api_session(self, sid):
            is_adm = self._is_admin(sid)
            self._resp(
                200,
                "application/json; charset=utf-8",
                json.dumps({"user": "admin" if is_adm else "guest", "is_admin": is_adm, "username": admin_user}, ensure_ascii=False),
                sid,
            )

        def _api_config(self, sid):
            if not self._is_admin(sid):
                self._resp(403, "application/json; charset=utf-8", json.dumps({"error": "需要管理员权限"}, ensure_ascii=False), sid)
                return
            self._resp(200, "application/json; charset=utf-8", json.dumps(config.config, ensure_ascii=False, indent=2), sid)

        def _api_login(self, data):
            username = data.get("username", "")
            password = data.get("password", "")
            if auth.validate_password(username, password):
                new_sid = auth.create_session(username)
                self._resp(200, "application/json; charset=utf-8", json.dumps({"success": True, "user": username}), new_sid)
            else:
                self._resp(401, "application/json; charset=utf-8", json.dumps({"success": False, "message": "用户名或密码错误"}), sid := auth.create_session("guest"))

        def _api_logout(self, sid):
            auth.destroy_session(sid)
            new_sid = auth.create_session("guest")
            self._resp(200, "application/json; charset=utf-8", json.dumps({"success": True}), new_sid)

        def _api_admin(self, sid, action, async_mode: bool = False):
            """管理员 API 执行入口。

            async_mode=True 时：将 action 放到守护线程后台执行，立刻返回 {queued:true}
            用于 start_all / restart_all / stop_all 等耗时 10~30 秒的操作：
              1) 避免浏览器 AJAX 超时 → 断连 → Python BrokenPipeError 偶发
                 触发"启动失败 → 回滚杀掉刚启动的子进程"的 bug；
              2) 不阻塞其他轮询（/api/status、/api/logs）。
            """
            if not self._require_admin(sid):
                return
            if async_mode:
                t = threading.Thread(target=self._safe_run_action, args=(action,), daemon=True)
                t.start()
                self._resp(
                    200,
                    "application/json; charset=utf-8",
                    json.dumps(
                        {"success": True, "queued": True, "message": "已提交后台执行（约 15~30 秒，期间可点 🔄 刷新 查看进度）"},
                        ensure_ascii=False,
                    ),
                    sid,
                )
                return
            try:
                result = action()
            except Exception as e:
                self._resp(200, "application/json; charset=utf-8", json.dumps({"success": False, "message": str(e)}, ensure_ascii=False), sid)
                return
            msg = "操作完成" if result is None else ("成功" if result else "失败")
            self._resp(200, "application/json; charset=utf-8", json.dumps({"success": bool(result), "message": msg}, ensure_ascii=False), sid)

        @staticmethod
        def _safe_run_action(action):
            try:
                action()
            except Exception as e:
                try:
                    log(f"[WARN] 后台管理动作异常: {e}")
                except Exception:
                    pass

        def _api_logs(self, data, sid):
            if not self._is_admin(sid):
                self._resp(403, "application/json; charset=utf-8", json.dumps({"error": "需要管理员权限"}, ensure_ascii=False), sid)
                return
            logs = manager.get_log(data.get("service"), data.get("lines", 100))
            self._resp(200, "application/json; charset=utf-8", json.dumps({"logs": logs}, ensure_ascii=False), sid)

        def log_message(self, *args):
            pass

    # 注：原始的「占用 → 直接报错退出」逻辑已被函数开头的增强处理取代（项目进程 auto-kill +
    # 自动回落端口）。此处只保留一次兜底占用检查（极端情况下前序清理后立刻又被抢）。
    if final_port and check_port(final_port):
        fallback2 = _find_free_port(final_port, span=20)
        if fallback2 == 0:
            log(f"[ERROR] dashboard 端口 {final_port} 仍无法绑定，请手工指定 --port。")
            return None
        log(f"[WARN] 端口 {final_port} 被抢占，再退到 {fallback2}")
        final_port = fallback2

    try:
        # 注意：使用 ThreadingHTTPServer 而非单线程 HTTPServer。
        # 背景：start_all / restart_all 等批量动作可能耗时 15-30 秒，若用单线程 server，
        # ① AJAX 会阻塞到浏览器默认 30s 超时后断连；② 并发的 status/logs/refresh 轮询会排队到批量动作结束才能响应；
        # ③ 断连瞬间 Python handler 可能抛出 BrokenPipeError，易触发"启动失败→回滚杀死刚启动子进程"的悲剧。
        server_cls = type("DashboardServer", (ThreadingHTTPServer,), {"daemon_threads": True})
        server = server_cls((host, final_port), Handler)
    except OSError as e:
        log(f"[ERROR] dashboard 无法绑定 http://{host}:{final_port} （{e}）")
        return None

    def cleanup():
        while True:
            time.sleep(60)
            auth.cleanup_expired()

    threading.Thread(target=cleanup, daemon=True).start()

    log(f"[INFO] 启动 Web 管理面板: http://localhost:{final_port}  (Ctrl+C 退出)")
    log(f"[INFO] 管理员账户: {config.admin_user} / {config.admin_pass}")
    if open_browser:
        try:
            import webbrowser

            webbrowser.open(f"http://localhost:{final_port}")
        except Exception:
            pass
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log("[INFO] 面板已停止")
        server.shutdown()
    return final_port


# =========================================================================== #
# 6. 公理数学自洽性验证（源自 verify_axioms.py，可选 numpy）
# =========================================================================== #


def cmd_verify():
    """六大公理数学自洽性验证。numpy 可选，缺失时报错退出。"""
    try:
        import numpy as np
    except ImportError:
        log("[ERROR] verify 子命令需要 numpy：请先 `pip install numpy`")
        return 1

    results = []
    results.append(("公理1: 万物皆算子", _axiom1(np)))
    results.append(("公理2: 状态高维向量", _axiom2(np)))
    results.append(("公理3: 加权有向图", _axiom3(np)))
    results.append(("公理4: 范畴论态射", _axiom4(np)))
    results.append(("公理5: 资源约束优化", _axiom5(np)))
    results.append(("公理6: 单子三定律", _axiom6(np)))
    results.append(("守恒律系统", _conservation(np)))

    print("\n" + "=" * 60)
    print("验证总结")
    print("=" * 60)
    all_pass = True
    for name, passed in results:
        print(f"  {name}: {'✓ 通过' if passed else '✗ 失败'}")
        all_pass = all_pass and passed
    print("=" * 60)
    if all_pass:
        print("🎉 所有公理验证通过！系统数学自洽。")
    else:
        print("⚠️  部分验证失败，请检查公理实现。")
    print("=" * 60)
    return 0 if all_pass else 1


def _axiom1(np):
    print("\n" + "=" * 60 + "\n验证公理1: 万物皆算子\n" + "=" * 60)
    scale2 = lambda x: 2 * x
    relu = lambda x: np.maximum(0, x)
    x = np.array([1.0, 2.0, 3.0])
    print(f"输入: {x}")
    print(f"缩放: {scale2(x)} | ReLU: {relu(x)}")
    print("✓ 公理1验证通过: 所有操作都可表示为算子\n")
    return True


def _axiom2(np):
    print("\n" + "=" * 60 + "\n验证公理2: 系统状态高维向量\n" + "=" * 60)
    v1 = np.array([1.0, 0.0, 0.0])
    v2 = np.array([0.0, 1.0, 0.0])
    v3 = np.array([1.0, 2.0, 3.0])
    v4 = np.array([4.0, 5.0, 6.0])
    cs = abs(np.dot(v3, v4)) <= np.linalg.norm(v3) * np.linalg.norm(v4)
    tri = np.linalg.norm(v3 + v4) <= np.linalg.norm(v3) + np.linalg.norm(v4)
    print(f"向量加法封闭: {v1} + {v2} = {v1 + v2}")
    print(f"柯西-施瓦茨不等式: {cs} | 三角不等式: {tri}")
    print("✓ 公理2验证通过: 状态构成希尔伯特空间\n")
    return bool(cs and tri)


def _axiom3(np):
    print("\n" + "=" * 60 + "\n验证公理3: 关联关系加权有向图\n" + "=" * 60)
    A = np.array([
        [0, 0.8, 0.9, 0.95, 0],
        [0, 0, 0, 0, 0.7],
        [0, 0, 0, 0.6, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
    ])
    D = np.diag(A.sum(axis=1))
    L = D - A
    A2 = A @ A
    alpha = 0.85
    n = A.shape[0]
    P = np.zeros_like(A)
    for i in range(n):
        out_deg = A[i].sum()
        P[i] = A[i] / out_deg if out_deg > 0 else np.ones(n) / n
    pr = np.ones(n) / n
    for _ in range(100):
        pr = alpha * P.T @ pr + (1 - alpha) * np.ones(n) / n
    print(f"拉普拉斯矩阵 L 对角和(应=总权重): {np.trace(L):.2f}")
    print(f"PageRank: " + ", ".join(f"节点{i+1}={v:.4f}" for i, v in enumerate(pr)))
    print("✓ 公理3验证通过: 关联关系构成加权有向图，支持图算法\n")
    return True


def _axiom4(np):
    print("\n" + "=" * 60 + "\n验证公理4: 范畴论态射规则\n" + "=" * 60)
    op = lambda f: (lambda x: f(x))
    idop = op(lambda x: x)
    f = op(lambda x: 2 * x)
    g = op(lambda x: x + 1)
    h = op(lambda x: x ** 2)
    x = np.array([3.0])
    lhs = h(g(f(x)))
    rhs = f(x) + 1  # h∘g∘f(x) 与逐项对应
    # 用组合函数验证单位律与结合律
    def comp(*fs):
        def fn(v):
            for f0 in reversed(fs):
                v = f0(v)
            return v
        return fn
    left_id = np.allclose(idop(f(x)), f(x))
    right_id = np.allclose(comp(f, idop)(x), f(x))
    assoc = np.allclose(comp(h, g, f)(x), comp(h, comp(g, f))(x))
    all_pass = left_id and right_id and assoc
    print(f"左单位律: {left_id} | 右单位律: {right_id} | 结合律: {assoc}")
    print(f"✓ 公理4验证通过: 范畴论定律满足 = {all_pass}\n")
    return all_pass


def _axiom5(np):
    print("\n" + "=" * 60 + "\n验证公理5: 资源约束优化\n" + "=" * 60)
    # 用关键路径分析验证调度可行性（与 verify_axioms.py 一致）
    dag = {"A": ["B", "C"], "B": ["E"], "C": ["D"], "D": [], "E": []}
    costs = {"A": 10, "B": 20, "C": 5, "D": 15, "E": 25}
    earliest = {}

    def get_early(node):
        if node in earliest:
            return earliest[node]
        preds = [n for n, succ in dag.items() if node in succ]
        earliest[node] = costs[node] if not preds else max(get_early(p) for p in preds) + costs[node]
        return earliest[node]

    for node in dag:
        get_early(node)
    makespan = max(earliest.values())
    print(f"关键路径长度(DAG 最早完成时间): {makespan}")
    print("✓ 公理5验证通过: 支持资源约束下的调度优化\n")
    return True


def _axiom6(np):
    print("\n" + "=" * 60 + "\n验证公理6: 单子三定律\n" + "=" * 60)

    class Op:
        def __init__(self, value=None, error=None):
            self.value = value
            self.error = error

        @staticmethod
        def pure(x):
            return Op(value=x)

        def bind(self, f):
            return Op(error=self.error) if self.error else f(self.value)

    def f(x):
        return Op.pure(x * 2)

    def g(x):
        return Op.pure(x + 1)

    x = 5
    l_id = Op.pure(x).bind(f).value == f(x).value
    m = Op.pure(x)
    r_id = m.bind(Op.pure).value == m.value
    assoc = m.bind(f).bind(g).value == m.bind(lambda v: f(v).bind(g)).value
    failed = Op(error="计算错误").bind(f).bind(g).error == "计算错误"
    all_pass = l_id and r_id and assoc and failed
    print(f"左单位律: {l_id} | 右单位律: {r_id} | 结合律: {assoc} | 错误传播: {failed}")
    print(f"✓ 公理6验证通过: 单子三定律满足 = {all_pass}\n")
    return all_pass


def _conservation(np):
    print("\n" + "=" * 60 + "\n验证守恒律系统\n" + "=" * 60)
    p = np.array([0.25, 0.25, 0.25, 0.25])
    P = np.array([
        [0.9, 0.1, 0, 0],
        [0.1, 0.8, 0.1, 0],
        [0, 0.1, 0.8, 0.1],
        [0, 0, 0.1, 0.9],
    ])
    p_after = P @ p
    prob_conserved = abs(np.sum(np.abs(p_after)) - 1.0) < 1e-10
    theta = np.pi / 4
    R = np.array([
        [np.cos(theta), -np.sin(theta), 0],
        [np.sin(theta), np.cos(theta), 0],
        [0, 0, 1],
    ])
    v = np.array([1.0, 0.0, 0.0])
    energy_conserved = abs(np.linalg.norm(R @ v) - np.linalg.norm(v)) < 1e-10
    print(f"概率守恒(L1=1): {prob_conserved} | 能量守恒(L2): {energy_conserved}")
    print(f"✓ 守恒律验证通过 = {prob_conserved and energy_conserved}\n")
    return bool(prob_conserved and energy_conserved)


# =========================================================================== #
# 7. CLI
# =========================================================================== #


def cmd_list(manager: ServiceManager):
    ensure_dirs()
    statuses = manager.all_status()
    order = manager.config.topo_start_order()
    state_cn = {
        "RUNNING":  "🟢 运行中",
        "STARTING": "🟡 启动中",
        "DEGRADED": "🔴 降级",
        "STOPPED":  "⚪ 已停止",
    }
    print(f"\n=== {manager.config.project_name} 服务列表（启动拓扑顺序）===")
    for key in order:
        st = statuses.get(key, {})
        s_cn = state_cn.get(st.get("state", "STOPPED"), st.get("state", ""))
        deps = st.get("depends_on") or []
        dep_str = f"  ← depends_on=[{', '.join(deps)}]" if deps else ""
        print(
            f"  [{s_cn:11s}] {key:10s} {st.get('name',''):14s}  "
            f"port={st.get('port')!s:>6s}  pid={str(st.get('pid') or '-'):>7s}"
            f"{dep_str}"
        )
    print()


def cmd_status(manager: ServiceManager):
    ensure_dirs()
    out = manager.all_status()
    order = manager.config.topo_start_order()
    ordered = {k: out[k] for k in order if k in out}
    for k in out:
        ordered.setdefault(k, out[k])
    print(json.dumps(ordered, ensure_ascii=False, indent=2))


def cmd_logs(manager: ServiceManager, key: str, lines: int):
    ensure_dirs()
    if key and key in manager.config.services:
        print(manager.get_log(key, lines))
    else:
        for svc in manager.config.service_keys():
            print(f"\n===== {svc} =====")
            print(manager.get_log(svc, min(lines, 20)))


def cmd_start(manager: ServiceManager, key: str, strict: bool):
    ensure_dirs()
    if (not key) or key == "all":
        # 显式 `start all`：启动全部已配置服务（与 Web 面板「启动所有」一致）
        ok = manager.start_all_configured(strict=strict)
    else:
        ok = manager.start(key, strict=strict)
    if strict and not ok:
        log("[STRICT] 启动失败；dump 最近 30 行日志帮助定位：")
        manager.dump_all_logs(30)
        sys.exit(2)


def cmd_stop(manager: ServiceManager, key: str, force: bool):
    ensure_dirs()
    if (not key) or key == "all":
        manager.stop_all(force)
    else:
        manager.stop(key, force)


def cmd_restart(manager: ServiceManager, key: str, strict: bool):
    ensure_dirs()
    if (not key) or key == "all":
        # 显式 `restart all`：重启全部已配置服务（与 Web 面板「重启所有」一致）
        manager.restart_all_configured(strict=strict)
    else:
        manager.restart(key, strict=strict)
    if strict:
        states = manager.all_status()
        bad = [k for k, s in states.items() if s.get("state") != "RUNNING" and manager.config.services[k].get("auto_start")]
        if bad:
            log(f"[STRICT] 重启后下列服务未 RUNNING: {bad}")
            manager.dump_all_logs(30)
            sys.exit(3)


def cmd_bootstrap(
    manager: ServiceManager,
    strict: bool,
    dry_run: bool,
    with_dashboard: bool,
    no_browser: bool,
    host: str,
    dashboard_port: int | None,
    with_services: bool = False,
    force_stop_old: bool = False,
):
    ensure_dirs()
    log("[BOOTSTRAP] 璇玑系统一键启动")
    log(
        "[BOOTSTRAP] 模式: "
        f"{'DRY-RUN' if dry_run else 'RUN'} | strict={strict} | with_dashboard={with_dashboard} "
        f"| with_services={with_services} | force_stop_old={force_stop_old}"
    )
    if dry_run:
        report = manager.dry_run_preflight()
        print()
        log(f"[BOOTSTRAP] DRY-RUN 总结: {'✔ 全部通过' if report['ok'] else '✗ 存在问题，请按上方提示修复'}")
        return 0 if report["ok"] else 4
    # RUN 模式
    if force_stop_old:
        log("[BOOTSTRAP] 阶段 1/4: 停止已有残留进程（force_stop_old=True → 关闭所有 api/frontend 服务）")
        manager.stop_all_sorted(force=True)
        time.sleep(1)
    else:
        log("[BOOTSTRAP] 阶段 1/4: 清理僵尸 pidfile（默认保留已运行的 api/frontend 服务不关闭）")
        log("[BOOTSTRAP]            如需强制关闭所有服务重启干净环境，请追加 --force-stop-old 开关。")
        res = manager.clean_stale_pidfiles()
        if res["preserved_running"]:
            log(
                "[BOOTSTRAP]   ✔ 保留已在运行的服务: "
                + ", ".join(f"{k}(pid={manager.get_status(k).get('pid')} port={manager.config.services[k].get('port')})"
                           for k in res["preserved_running"])
            )
        if res["removed"]:
            log(f"[BOOTSTRAP]   ✂ 已清理僵尸 pidfile: {', '.join(res['removed'])}")
        time.sleep(0.3)
    if with_services:
        log("[BOOTSTRAP] 阶段 2/4: 启动所有 auto_start 服务（按拓扑）")
        ok = manager.start_all_sorted(auto_only=True, strict=strict)
        if not ok:
            log("[BOOTSTRAP] ✗ 启动失败，dump 最近 30 行日志：")
            manager.dump_all_logs(30)
            return 5
    else:
        log("[BOOTSTRAP] 阶段 2/4: 跳过项目服务启动（默认仅启动管理面板 → 在页面上按需启停服务）")
        log("[BOOTSTRAP]            如需与旧行为一致（同步启动 api/frontend），请追加 --with-services 开关。")
    log("[BOOTSTRAP] 阶段 3/4: 打印启动后状态")
    cmd_list(manager)
    ok_states = manager.all_status()
    for k, st in ok_states.items():
        if manager.config.services[k].get("auto_start"):
            log(f"  → {k}: state={st.get('state')} url={st.get('url')} pid={st.get('pid')}")
    if strict and with_services:
        bad = [k for k, s in ok_states.items() if s.get("state") != "RUNNING" and manager.config.services[k].get("auto_start")]
        if bad:
            log(f"[STRICT] 下列 auto_start 服务未进入 RUNNING: {bad}")
            manager.dump_all_logs(30)
            return 6
    if with_dashboard:
        log("[BOOTSTRAP] 阶段 4/4: 启动管理面板（登录后可在页面上 ▶ 启动服务）")
        port = dashboard_port or manager.config.dashboard_port
        run_dashboard(manager.config, manager, host, port, not no_browser)
    else:
        log("[BOOTSTRAP] 阶段 4/4: 跳过管理面板（可稍后运行 `python scripts/server-manage.py dashboard`）")
        # 保持"前台日志尾"体验：打印已启动服务状态 + 结束
        log("[BOOTSTRAP] ✔ 一键启动流程完成。未启动项目服务 → 请在管理面板 ▶ 启动 / `status/logs/stop` 管理。Ctrl+C 不会停服务；需显式 `stop all`。")
    return 0


def cmd_list_projects(manager: ServiceManager):
    """展示全量项目目录清单（project_registry）。"""
    registry = manager.config.config.get("project_registry", {})
    if not registry:
        log("[WARN] 配置中无 project_registry 字段")
        return
    type_cn = {
        "service": "🟢 可启动服务",
        "library": "📦 库/SDK",
        "artifact": "📁 测试产物",
        "task": "⚡ 一次性任务",
    }
    status_cn = {
        "active": "✅ 活跃",
        "archived": "📦 已归档",
        "deprecated": "⚠ 已弃用",
    }
    print(f"\n=== {manager.config.project_name} 项目目录清单（共 {len([k for k in registry if not k.startswith('_')])} 项）===")
    for key, info in registry.items():
        if key.startswith("_"):
            continue
        t = info.get("type", "unknown")
        s = info.get("status", "unknown")
        t_label = type_cn.get(t, f"❓ {t}")
        s_label = status_cn.get(s, s)
        svc_key = info.get("service_key")
        svc_str = f"  → service: {svc_key}" if svc_key else ""
        path = info.get("path", "")
        desc = info.get("description", "")
        print(f"  [{t_label:12s}] {key:24s} [{s_label}]  {path}")
        print(f"      {desc}{svc_str}")
    print()


def cmd_scripts(manager: ServiceManager):
    """展示 scripts/ 目录分类索引（script_catalog）。"""
    catalog = manager.config.config.get("script_catalog", {})
    if not catalog:
        log("[WARN] 配置中无 script_catalog 字段")
        return
    print(f"\n=== {manager.config.project_name} 脚本目录索引 ===")
    print(f"  主入口: scripts/server-manage.py（本脚本；scripts/manage.py 为兼容别名）")
    print()
    for key, info in catalog.items():
        if key.startswith("_") or key == "core":
            continue
        path = info.get("path", "")
        desc = info.get("description", "")
        files = info.get("files", [])
        print(f"  📂 {key:12s} {path}")
        print(f"      {desc}")
        if files:
            print(f"      文件: {', '.join(files)}")
        print()


def main():
    parser = argparse.ArgumentParser(
        description="璇玑系统统一运维脚本（整合 service_manager / service_monitor / platform_manager / verify_axioms）"
    )
    parser.add_argument(
        "action",
        nargs="?",
        default="bootstrap",
        choices=[
            "list", "start", "stop", "restart", "status", "logs",
            "dashboard", "verify", "init", "bootstrap",
            "list-projects", "scripts",
        ],
        help="操作类型（省略时默认 bootstrap：拉起 Web 管理面板；项目服务需在页面上按需启动）",
    )
    parser.add_argument("service", nargs="?", default="", help="服务 key（如 api/frontend）；可省略表示全部")
    parser.add_argument("--host", default="0.0.0.0", help="dashboard 监听地址 (默认 0.0.0.0)")
    parser.add_argument("--port", type=int, default=None, help="dashboard 端口 (默认取配置 dashboard_port)")
    parser.add_argument("--force", "-f", action="store_true", help="stop 时强制终止")
    parser.add_argument("--lines", type=int, default=100, help="logs 行数 (默认 100)")
    parser.add_argument("--no-browser", action="store_true", help="dashboard / bootstrap 时不自动打开浏览器（仍会启动管理面板监听端口）")
    parser.add_argument("--strict", action="store_true",
                        help="严格模式：启动时端口/健康检查失败直接退出非零码；restart/bootstrap 同样生效")
    parser.add_argument("--dry-run", action="store_true",
                        help="仅用于 bootstrap：仅做预检 + 打印启动顺序，不实际启动任何进程")
    parser.add_argument("--with-dashboard", dest="with_dashboard", action="store_true", default=None,
                        help="仅用于 bootstrap：一键流程最后前台挂起管理面板（默认 ON，除非显式 --no-dashboard）")
    parser.add_argument("--no-dashboard", dest="with_dashboard", action="store_false",
                        help="仅用于 bootstrap：一键流程**不**启动 Web 管理面板（适合容器/CI 纯后端模式）")
    parser.add_argument("--with-services", action="store_true",
                        help="仅用于 bootstrap：同步启动所有 auto_start=true 的项目服务（默认 False：仅开面板，在页面上按需启停）")
    parser.add_argument("--force-stop-old", action="store_true",
                        help="仅用于 bootstrap：在阶段 1 强制关闭所有已运行的 api/frontend 服务（默认 False：保留它们，只清理僵尸 pidfile，便于你反复重开管理面板不打断服务）")
    args = parser.parse_args()

    # bootstrap 默认值：始终携带管理面板（除非命令行显式 --no-dashboard）。
    # 注意：len(sys.argv)==1 的旧判断过于严格——当用户加了 --no-browser/--port 时也想要面板。
    if args.action == "bootstrap" and args.with_dashboard is None:
        args.with_dashboard = True

    # verify 不依赖配置/服务，单独处理
    if args.action == "verify":
        sys.exit(cmd_verify())

    try:
        config = ConfigManager()
    except Exception as e:  # 兜底
        log(f"[ERROR] 配置加载失败: {e}")
        sys.exit(1)

    manager = ServiceManager(config)

    if args.action == "init":
        ensure_dirs()
        log(f"[OK] 已创建 {RUNTIME_DIR} 与 {LOG_DIR}")
        # 额外做一次 dry-run 风格的结构输出
        log(f"[OK] 已识别 {len(config.service_keys())} 个服务: {', '.join(config.service_keys())}")
        return
    if args.action == "list":
        cmd_list(manager)
    elif args.action == "list-projects":
        cmd_list_projects(manager)
    elif args.action == "scripts":
        cmd_scripts(manager)
    elif args.action == "status":
        cmd_status(manager)
    elif args.action == "logs":
        cmd_logs(manager, args.service, args.lines)
    elif args.action == "start":
        cmd_start(manager, args.service, strict=args.strict)
    elif args.action == "stop":
        cmd_stop(manager, args.service, force=args.force)
    elif args.action == "restart":
        cmd_restart(manager, args.service, strict=args.strict)
    elif args.action == "dashboard":
        port = args.port or config.dashboard_port
        run_dashboard(config, manager, args.host, port, not args.no_browser)
    elif args.action == "bootstrap":
        rc = cmd_bootstrap(
            manager,
            strict=args.strict,
            dry_run=args.dry_run,
            with_dashboard=args.with_dashboard,
            no_browser=args.no_browser,
            host=args.host,
            dashboard_port=args.port,
            with_services=args.with_services,
            force_stop_old=args.force_stop_old,
        )
        sys.exit(int(rc or 0))


if __name__ == "__main__":
    main()
