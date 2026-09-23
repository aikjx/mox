#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""mox-governance-mcp —— 璇玑（infotopograph）仓库治理 MCP 服务器（零第三方依赖）

定位
    把仓库既有的 CI 门禁脚本（唯一事实源）包装成可被 IDE / Coding Agent 调用的工具：

        scripts/gate/verify-ports.py     端口漂移校验（PORT-REGISTRY-001）
        scripts/gate/check-doc-links.py  文档链接校验（DOC-GOV-ARC-V1.0 第 7 节）

    这样 AI 助手在"写代码之前"就能读到权威端口表，在"写完之后"能自己跑一遍门禁，
    把"端口漂移 / 文档断链"这两类仓库治理缺陷从"事后 CI 报错"提前到"编码当场拦截"。

为什么不用第三方 MCP SDK
    本服务器只用 Python 标准库（>= 3.8），通过 stdio 自实现 MCP 的 JSON-RPC 2.0 子集。
    目的：CI 容器、Windows 开发机（本机实测 CPython 3.8.8）、离线环境均可即用，
    不引入依赖漂移，也让"工具契约"完全显式可读。

协议支持
    initialize / ping / tools/list / tools/call / resources/list / resources/read
    传输：stdio，换行分隔 JSON；同时兼容带 Content-Length 头的 LSP 式分帧。

安全边界（刻意收紧，请勿放宽）
    1. 只能执行 SCRIPTS 白名单内的门禁脚本；参数为受控布尔/枚举，绝不使用 shell=True；
    2. 仓库根只允许两种来源：脚本自身上溯两级，或环境变量 MOX_REPO_ROOT 显式指定
       且必须通过存在性校验；不接受任意路径拼接出的可执行文件；
    3. 每次脚本调用有超时（默认 300 秒，硬上限 900 秒）。

用法
    python tools/mox-governance-mcp/server.py             # 以 MCP 服务器运行（stdio）
    python tools/mox-governance-mcp/server.py --describe  # 打印工具/资源清单（供 IDE 登记）
    python tools/mox-governance-mcp/server.py --selftest  # 本地自检（含真实调用，不起外部进程）
"""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# --------------------------------------------------------------------------- #
# 常量与安全边界
# --------------------------------------------------------------------------- #
SERVER_NAME = "mox-governance-mcp"
SERVER_VERSION = "1.0.0"
PROTOCOL_VERSION = "2024-11-05"

# tools/mox-governance-mcp/server.py -> 上溯两级 = 仓库根
DEFAULT_REPO_ROOT = Path(__file__).resolve().parents[2]

# 允许执行的门禁脚本白名单（文件名 -> 人类可读职责）
SCRIPTS: Dict[str, str] = {
    "verify-ports.py": "端口漂移校验（PORT-REGISTRY-001）",
    "check-doc-links.py": "文档链接校验（DOC-GOV-ARC-V1.0 §7）",
}

# 端口校验需全仓遍历 platform/docs 等目录，本机实测约 130 秒；
# 默认放宽到 600 秒以覆盖慢机器/CI 冷缓存场景，硬上限 900 秒防失控。
TIMEOUT_DEFAULT = 600
TIMEOUT_HARD_MAX = 900
DETAIL_LIMIT_MAX = 500


class ToolError(Exception):
    """工具执行期的可控失败（会作为 MCP 工具错误返回，而非协议错误）。"""


def repo_root() -> Path:
    """解析仓库根：环境变量优先，但必须真实存在，否则回落到脚本自身定位。"""
    env = os.environ.get("MOX_REPO_ROOT")
    if env:
        p = Path(env).resolve()
        if not p.is_dir():
            raise ToolError("MOX_REPO_ROOT 指向的目录不存在: " + str(p))
        return p
    return DEFAULT_REPO_ROOT


# --------------------------------------------------------------------------- #
# 工具（tools）定义
# --------------------------------------------------------------------------- #
TOOLS: List[Dict[str, Any]] = [
    {
        "name": "mox_port_lookup",
        "title": "端口注册表查询",
        "description": (
            "查询璇玑仓库权威端口注册表（源自 scripts/gate/verify-ports.py 的 CANONICAL， "
            "与 docs/api/PORT-REGISTRY.md 同步）。可按端口号精确查，也可按服务名关键字模糊查。 "
            "用途：在新增服务、编写 docker-compose / nginx / 健康检查 / CORS 配置之前，"
            "先取得权威端口，避免臆造端口造成 CI 门禁失败。"
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "port": {"type": "integer", "description": "要查询的端口号（如 3080）"},
                "query": {"type": "string", "description": "服务名/说明关键字（如 alliance、executor）"},
                "category": {
                    "type": "string",
                    "description": "按分类过滤",
                    "enum": ["RUNTIME", "ALLIANCE", "ANCILLARY", "LEGACY", "DEPRECATED", "TEST", "THIRD"],
                },
            },
            "additionalProperties": False,
        },
    },
    {
        "name": "mox_port_verify",
        "title": "端口漂移校验",
        "description": (
            "执行 scripts/gate/verify-ports.py 全仓端口漂移校验。"
            "ERROR=已退役端口被活跃文件引用 或 platform_config.json 与注册表不一致（会阻断 CI）；"
            "WARN=发现未登记端口（潜在新服务，需按 PORT-REGISTRY-001 第 5 章登记）；"
            "INFO=正常引用。建议在改动端口、新增服务、release 之前调用。"
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "severity": {
                    "type": "array",
                    "description": "只返回这些级别的条目，留空表示全部",
                    "items": {"type": "string", "enum": ["ERROR", "WARN", "INFO"]},
                },
                "limit": {"type": "integer", "description": "最多返回多少条明细，默认 50"},
                "timeout_seconds": {"type": "integer", "description": "超时秒数，默认 300"},
            },
            "additionalProperties": False,
        },
    },
    {
        "name": "mox_doc_links_check",
        "title": "文档链接校验",
        "description": (
            "执行 scripts/gate/check-doc-links.py 文档链接校验（docs/ 结构门禁）。"
            "检出 Markdown 链接、HTML href/src、反引号路径引用中的断链。"
            "用途：目录迁移、文件重命名之后，确认没有遗留死链。"
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "include_archive": {"type": "boolean", "description": "是否包含 docs/_archive/，默认 false"},
                "include_repo": {"type": "boolean", "description": "是否一并校验仓库其它 md/html 中对 docs/ 的引用，默认 false"},
                "strict": {"type": "boolean", "description": "反引号路径引用不存在时也算断链，默认 false"},
                "limit": {"type": "integer", "description": "最多返回多少条断链明细，默认 50"},
                "timeout_seconds": {"type": "integer", "description": "超时秒数，默认 300"},
            },
            "additionalProperties": False,
        },
    },
    {
        "name": "mox_ci_gate",
        "title": "仓库治理体检（自定义多步流程）",
        "description": (
            "自定义流程：一次串联『端口漂移校验』与『文档链接校验』两项门禁，输出统一结论。"
            "这是本 MCP 的编排示范——Agent 无需知道底层有两个脚本、各自参数与退出码，"
            "一次调用即可拿到可直接贴进 PR 描述的体检报告。返回 ok=false 时应先修复再提交。"
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "include_repo_docs": {"type": "boolean", "description": "文档链接校验是否覆盖仓库其它 md/html，默认 false"},
                "limit": {"type": "integer", "description": "每个门禁最多返回多少条明细，默认 20"},
                "timeout_seconds": {"type": "integer", "description": "单个脚本的超时秒数，默认 300"},
            },
            "additionalProperties": False,
        },
    },
]

RESOURCES: List[Dict[str, Any]] = [
    {
        "uri": "mox://governance/port-registry",
        "name": "璇玑权威端口注册表",
        "description": "RUNTIME/ALLIANCE/ANCILLARY/LEGACY/DEPRECATED/TEST/THIRD 全量端口与说明",
        "mimeType": "text/markdown",
    },
    {
        "uri": "mox://governance/gates",
        "name": "璇玑 CI 门禁脚本清单",
        "description": "本机可调用的门禁脚本、职责与规范出处",
        "mimeType": "text/markdown",
    },
]


# --------------------------------------------------------------------------- #
# 底层执行：白名单脚本
# --------------------------------------------------------------------------- #
def _resolve_script(name: str) -> Path:
    if name not in SCRIPTS:
        raise ToolError("脚本不在白名单内: " + str(name))
    p = repo_root() / "scripts" / name
    if not p.is_file():
        raise ToolError("门禁脚本缺失: " + str(p))
    return p


def _normalize_timeout(value: Any) -> int:
    if value is None:
        return TIMEOUT_DEFAULT
    try:
        n = int(value)
    except (TypeError, ValueError):
        raise ToolError("timeout_seconds 必须是整数")
    if n <= 0:
        raise ToolError("timeout_seconds 必须为正整数")
    return min(n, TIMEOUT_HARD_MAX)


def _run_script(name: str, args: List[str], timeout_seconds: Any) -> Tuple[int, str, str, int]:
    """执行白名单门禁脚本，返回 (returncode, stdout, stderr, 耗时毫秒)。"""
    script = _resolve_script(name)
    timeout = _normalize_timeout(timeout_seconds)
    env = dict(os.environ)
    env["PYTHONIOENCODING"] = "utf-8"
    cmd = [sys.executable, str(script)] + [str(a) for a in args]
    started = time.time()
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(repo_root()),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            shell=False,  # 恒为 False：绝不走 shell
        )
    except subprocess.TimeoutExpired:
        raise ToolError("门禁脚本执行超时（" + str(timeout) + " 秒）: " + name)
    elapsed_ms = int((time.time() - started) * 1000)
    out = (proc.stdout or b"").decode("utf-8", "replace")
    err = (proc.stderr or b"").decode("utf-8", "replace")
    return proc.returncode, out, err, elapsed_ms


def _loads(text: str) -> Any:
    """从脚本输出中稳健提取 JSON：先整体 parse，失败则截取最外层花括号。"""
    text = text.strip()
    try:
        return json.loads(text)
    except ValueError:
        pass
    start, end = text.find("{"), text.rfind("}")
    if start >= 0 and end > start:
        return json.loads(text[start:end + 1])
    raise ToolError("无法从脚本输出中解析 JSON")


# --------------------------------------------------------------------------- #
# 工具实现
# --------------------------------------------------------------------------- #
def _load_port_module() -> Any:
    """以文件路径方式加载 verify-ports.py，取得其中的 CANONICAL 注册表（只读）。"""
    spec = importlib.util.spec_from_file_location("_mox_verify_ports", _resolve_script("verify-ports.py"))
    if spec is None or spec.loader is None:  # pragma: no cover
        raise ToolError("无法加载 verify-ports.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _clamp_limit(value: Any, default: int = 50) -> int:
    if value is None:
        return default
    try:
        n = int(value)
    except (TypeError, ValueError):
        raise ToolError("limit 必须是整数")
    return max(1, min(n, DETAIL_LIMIT_MAX))


def tool_port_lookup(args: Dict[str, Any]) -> Dict[str, Any]:
    mod = _load_port_module()
    canonical: Dict[int, Tuple[str, str]] = getattr(mod, "CANONICAL")

    port = args.get("port")
    query = (args.get("query") or "").strip()
    category = args.get("category")

    if port is not None:
        try:
            key = int(port)
        except (TypeError, ValueError):
            raise ToolError("port 必须是整数")
        entry = canonical.get(key)
        if entry is None:
            return {
                "found": False,
                "port": key,
                "hint": "该端口未在权威注册表中登记；若确属新服务，请按 PORT-REGISTRY-001 第 5 章登记后再使用",
            }
        cat, desc = entry
        return {
            "found": True,
            "port": key,
            "category": cat,
            "service": desc,
            "deprecated": cat == "DEPRECATED",
            "reusable": cat == "DEPRECATED",  # True 表示禁止复用
        }

    if not query and not category:
        raise ToolError("需要提供 port，或提供 query/category 之一")

    needle = query.lower()
    matches = []
    for p, (cat, desc) in sorted(canonical.items()):
        if category and cat != category:
            continue
        if needle and needle not in desc.lower() and needle not in cat.lower():
            continue
        matches.append({"port": p, "category": cat, "service": desc})
    return {"found": bool(matches), "count": len(matches), "matches": matches}


def tool_port_verify(args: Dict[str, Any]) -> Dict[str, Any]:
    severity = args.get("severity") or []
    limit = _clamp_limit(args.get("limit"))
    rc, out, err, ms = _run_script("verify-ports.py", ["--json"], args.get("timeout_seconds"))
    report = _loads(out)
    issues = report.get("issues") or []
    if severity:
        wanted = {str(s).upper() for s in severity}
        issues = [i for i in issues if str(i.get("severity", "")).upper() in wanted]

    result = {
        "repo": report.get("repo", str(repo_root())),
        "passed": bool(report.get("passed")),
        "exit_code": rc,
        "error_count": report.get("error_count", 0),
        "warn_count": report.get("warn_count", 0),
        "scanned_ports": report.get("scanned_ports", 0),
        "duration_ms": ms,
        "returned_issues": min(len(issues), limit),
        "issues": issues[:limit],
        "advice": (
            "端口校验通过，可放心提交。"
            if report.get("passed")
            else "存在 ERROR：请修正后重跑 python scripts/gate/verify-ports.py。"
        ),
    }
    if err.strip():
        result["stderr_tail"] = err.strip()[-800:]
    return result


def tool_doc_links_check(args: Dict[str, Any]) -> Dict[str, Any]:
    limit = _clamp_limit(args.get("limit"))
    cmd: List[str] = []
    if args.get("include_archive"):
        cmd.append("--all")
    if args.get("include_repo"):
        cmd.append("--repo")
    if args.get("strict"):
        cmd.append("--strict")

    tmpdir = tempfile.mkdtemp(prefix="mox_gate_")
    json_path = os.path.join(tmpdir, "doc-links.json")
    cmd += ["--json", json_path, "--max-detail", str(limit)]
    rc, out, err, ms = _run_script("check-doc-links.py", cmd, args.get("timeout_seconds"))

    report: Dict[str, Any] = {}
    if os.path.isfile(json_path):
        try:
            with open(json_path, "r", encoding="utf-8") as fh:
                report = json.load(fh)
        except ValueError:
            report = {}
        finally:
            try:
                os.remove(json_path)
                os.rmdir(tmpdir)
            except OSError:
                pass

    broken = report.get("broken") or []
    warnings = report.get("warnings") or []
    per_file: Dict[str, int] = {}
    for item in broken:
        per_file[item.get("file", "?")] = per_file.get(item.get("file", "?"), 0) + 1
    top_files = sorted(per_file.items(), key=lambda kv: -kv[1])[:10]

    result = {
        "passed": len(broken) == 0,
        "exit_code": rc,
        "scanned_files": report.get("scanned_files", 0),
        "checked_refs": report.get("checked_refs", 0),
        "broken_count": len(broken),
        "warning_count": len(warnings),
        "duration_ms": ms,
        "top_broken_files": [{"file": f, "count": c} for f, c in top_files],
        "broken": broken[:limit],
        "stdout_summary": _first_line(out),
        "advice": (
            "文档链接校验通过。"
            if not broken
            else "存在断链：请修复上述引用，或确认是否属于目录迁移后漏改的路径。"
        ),
    }
    if err.strip():
        result["stderr_tail"] = err.strip()[-800:]
    return result


def tool_ci_gate(args: Dict[str, Any]) -> Dict[str, Any]:
    limit = _clamp_limit(args.get("limit"), default=20)
    timeout = args.get("timeout_seconds")

    gates: Dict[str, Any] = {}
    failures: List[str] = []

    try:
        ports = tool_port_verify({"severity": ["ERROR", "WARN"], "limit": limit, "timeout_seconds": timeout})
    except ToolError as exc:
        ports = {"passed": False, "error": str(exc)}
        failures.append("端口校验未能执行: " + str(exc))
    else:
        if not ports.get("passed"):
            failures.append("端口漂移校验未通过（ERROR=" + str(ports.get("error_count")) + "）")
    gates["port_registry"] = ports

    try:
        docs = tool_doc_links_check({"include_repo": bool(args.get("include_repo_docs")),
                                     "limit": limit, "timeout_seconds": timeout})
    except ToolError as exc:
        docs = {"passed": False, "error": str(exc)}
        failures.append("文档链接校验未能执行: " + str(exc))
    else:
        if not docs.get("passed"):
            failures.append("文档链接校验未通过（断链=" + str(docs.get("broken_count")) + "）")
    gates["doc_links"] = docs

    ok = not failures
    return {
        "ok": ok,
        "repo": str(repo_root()),
        "gates": gates,
        "failures": failures,
        "summary": (
            "治理体检通过：端口无漂移、文档无断链，可以提交/发布。"
            if ok
            else "治理体检未通过，请按 failures 逐项修复后重跑 python scripts/gate/verify-ports.py 与 python scripts/gate/check-doc-links.py。"
        ),
    }


def _first_line(text: str) -> str:
    for line in (text or "").splitlines():
        if line.strip():
            return line.strip()
    return ""


TOOL_IMPLS = {
    "mox_port_lookup": tool_port_lookup,
    "mox_port_verify": tool_port_verify,
    "mox_doc_links_check": tool_doc_links_check,
    "mox_ci_gate": tool_ci_gate,
}


# --------------------------------------------------------------------------- #
# 资源（resources）实现
# --------------------------------------------------------------------------- #
def resource_port_registry() -> str:
    mod = _load_port_module()
    canonical: Dict[int, Tuple[str, str]] = getattr(mod, "CANONICAL")
    grouped: Dict[str, List[Tuple[int, str]]] = {}
    for port, (cat, desc) in canonical.items():
        grouped.setdefault(cat, []).append((port, desc))
    lines = ["# 璇玑权威端口注册表", ""]
    lines.append("> 来源：scripts/gate/verify-ports.py 的 CANONICAL；改动须同步 docs/api/PORT-REGISTRY.md 并通过 PORT-REGISTRY-001 变更流程。")
    lines.append("")
    for cat in ["RUNTIME", "ALLIANCE", "ANCILLARY", "LEGACY", "DEPRECATED", "TEST", "THIRD"]:
        items = sorted(grouped.get(cat, []))
        if not items:
            continue
        lines.append("## " + cat + "（" + str(len(items)) + " 个）")
        lines.append("")
        for port, desc in items:
            mark = "  [禁止复用]" if cat == "DEPRECATED" else ""
            lines.append("- " + str(port) + " —— " + desc + mark)
        lines.append("")
    return "\n".join(lines)


def resource_gates() -> str:
    lines = ["# 璇玑 CI 门禁脚本清单", ""]
    for name, duty in SCRIPTS.items():
        lines.append("- scripts/" + name + " —— " + duty)
    lines.append("")
    lines.append("调用方式统一为：python scripts/<脚本名> [--json]，退出码 0 表示通过。")
    return "\n".join(lines)


RESOURCE_IMPLS = {
    "mox://governance/port-registry": resource_port_registry,
    "mox://governance/gates": resource_gates,
}


# --------------------------------------------------------------------------- #
# MCP 协议层（JSON-RPC 2.0 over stdio）
# --------------------------------------------------------------------------- #
def _result(mid: Any, payload: Dict[str, Any]) -> Dict[str, Any]:
    return {"jsonrpc": "2.0", "id": mid, "result": payload}


def _error(mid: Any, code: int, message: str) -> Dict[str, Any]:
    return {"jsonrpc": "2.0", "id": mid, "error": {"code": code, "message": message}}


def call_tool(name: str, args: Optional[Dict[str, Any]]) -> Dict[str, Any]:
    impl = TOOL_IMPLS.get(name)
    if impl is None:
        raise ToolError("未知工具: " + str(name))
    payload = impl(dict(args or {}))
    pretty = json.dumps(payload, ensure_ascii=False, indent=2)
    return {"content": [{"type": "text", "text": pretty}], "isError": False, "structuredContent": payload}


def handle(msg: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """处理一条 JSON-RPC 消息；通知（无 id 且无 result 期望）返回 None。"""
    if not isinstance(msg, dict):
        return _error(None, -32600, "Invalid Request")
    mid = msg.get("id")
    method = msg.get("method") or ""

    if method == "initialize":
        return _result(mid, {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {"listChanged": False}, "resources": {"subscribe": False}},
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
            "instructions": (
                "璇玑仓库治理 MCP。改端口前先调 mox_port_lookup；"
                "改动涉及端口/服务部署时调 mox_port_verify；"
                "移动或重命名 docs 文件后调 mox_doc_links_check；"
                "提交/发布前调 mox_ci_gate 做一次整体体检。"
            ),
        })

    if method in ("notifications/initialized", "notifications/cancelled", "initialized"):
        return None
    if method == "ping":
        return _result(mid, {})

    if method == "tools/list":
        return _result(mid, {"tools": TOOLS})
    if method == "tools/call":
        params = msg.get("params") or {}
        name = params.get("name")
        arguments = params.get("arguments") or {}
        try:
            return _result(mid, call_tool(name, arguments))
        except ToolError as exc:
            return _result(mid, {
                "content": [{"type": "text", "text": "[工具执行失败] " + str(exc)}],
                "isError": True,
            })
        except Exception as exc:  # 兜底：任何意外都不能打挂会话
            return _result(mid, {
                "content": [{"type": "text", "text": "[内部错误] " + type(exc).__name__ + ": " + str(exc)}],
                "isError": True,
            })

    if method == "resources/list":
        return _result(mid, {"resources": RESOURCES})
    if method == "resources/read":
        params = msg.get("params") or {}
        uri = params.get("uri") or ""
        impl = RESOURCE_IMPLS.get(uri)
        if impl is None:
            return _error(mid, -32602, "未知资源: " + str(uri))
        try:
            text = impl()
        except ToolError as exc:
            return _error(mid, -32603, str(exc))
        meta = next((r for r in RESOURCES if r["uri"] == uri), {})
        return _result(mid, {"contents": [{
            "uri": uri,
            "mimeType": meta.get("mimeType", "text/markdown"),
            "text": text,
        }]})

    if mid is None:
        return None
    return _error(mid, -32601, "Method not found: " + str(method))


# --------------------------------------------------------------------------- #
# stdio 传输：换行分隔 JSON + 兼容 Content-Length 分帧
# --------------------------------------------------------------------------- #
def _read_exact(stream, size: int) -> bytes:
    buf = b""
    while len(buf) < size:
        chunk = stream.read(size - len(buf))
        if not chunk:
            break
        buf += chunk
    return buf


def iter_messages(stream):
    """从二进制流迭代解析 JSON-RPC 消息。"""
    while True:
        line = stream.readline()
        if not line:
            return
        head = line.strip()
        if not head:
            continue
        if head.lower().startswith(b"content-length:"):
            size = int(head.split(b":", 1)[1].strip())
            # 读完剩余头部直到空行
            while True:
                nxt = stream.readline()
                if not nxt or nxt in (b"\r\n", b"\n"):
                    break
            body = _read_exact(stream, size)
            yield body
            continue
        yield head


def serve(stdin=None, stdout=None) -> int:
    stdin = stdin or sys.stdin.buffer
    stdout = stdout or sys.stdout.buffer
    for raw in iter_messages(stdin):
        try:
            msg = json.loads(raw.decode("utf-8", "replace"))
        except ValueError:
            stdout.write((json.dumps(_error(None, -32700, "Parse error"), ensure_ascii=False) + "\n").encode("utf-8"))
            stdout.flush()
            continue
        resp = handle(msg)
        if resp is None:
            continue
        stdout.write((json.dumps(resp, ensure_ascii=False) + "\n").encode("utf-8"))
        stdout.flush()
    return 0


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #
def _describe() -> int:
    payload = {
        "server": SERVER_NAME,
        "version": SERVER_VERSION,
        "protocolVersion": PROTOCOL_VERSION,
        "transport": "stdio",
        "repo_root": str(repo_root()),
        "capabilities": ["tools", "resources"],
        "tools": [{"name": t["name"], "title": t.get("title"), "description": t["description"]} for t in TOOLS],
        "resources": [{"uri": r["uri"], "name": r["name"]} for r in RESOURCES],
        "python": sys.version.split()[0],
    }
    sys.stdout.write(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
    return 0


def _selftest() -> int:
    """本地自检：覆盖协议分发、白名单、工具schema 与真实脚本调用。"""
    checks: List[Tuple[str, bool, str]] = []

    def record(name: str, ok: bool, detail: str = "") -> None:
        checks.append((name, ok, detail))

    def safe(name: str, fn) -> Any:
        try:
            value = fn()
            record(name, True)
            return value
        except Exception as exc:  # noqa: BLE001
            record(name, False, type(exc).__name__ + ": " + str(exc))
            return None

    init = safe("initialize 返回 serverInfo", lambda: handle({"jsonrpc": "2.0", "id": 1, "method": "initialize"}))
    record("initialize serverInfo 正确",
           bool(init) and init.get("result", {}).get("serverInfo", {}).get("name") == SERVER_NAME)

    tl = safe("tools/list 返回 4 个工具", lambda: handle({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}))
    record("tools/list 数量为 4", bool(tl) and len(tl.get("result", {}).get("tools", [])) == 4)

    schema_ok = True
    for tool in TOOLS:
        sch = tool.get("inputSchema", {})
        if sch.get("type") != "object" or "properties" not in sch:
            schema_ok = False
    record("全部工具 inputSchema 合规", schema_ok)

    lookup = safe("查询已登记端口", lambda: call_tool("mox_port_lookup", {"port": 3080}))
    record("端口 3080 归类为 RUNTIME",
           bool(lookup) and lookup.get("structuredContent", {}).get("category") == "RUNTIME")

    # 示范端口刻意取 99999（超出 TCP 端口上界）：既能验证“未登记”分支，
    # 又不会被 scripts/gate/verify-ports.py 扫成未登记端口 WARN（避免本文件污染自家门禁）
    miss = safe("查询未登记端口", lambda: call_tool("mox_port_lookup", {"port": 99999}))
    record("未登记端口返回 found=false", bool(miss) and miss.get("structuredContent", {}).get("found") is False)

    search = safe("关键字查询 alliance", lambda: call_tool("mox_port_lookup", {"query": "alliance"}))
    record("关键字查询有命中", bool(search) and int(search.get("structuredContent", {}).get("count", 0)) > 0)

    resource = safe("读取端口注册表资源", lambda: handle({
        "jsonrpc": "2.0", "id": 3, "method": "resources/read",
        "params": {"uri": "mox://governance/port-registry"}}))
    record("资源内容非markdown空文", bool(resource)
           and "RUNTIME" in resource.get("result", {}).get("contents", [{}])[0].get("text", ""))

    bad = safe("未知方法返回 -32601", lambda: handle({"jsonrpc": "2.0", "id": 4, "method": "does/not/exist"}))
    record("未知方法错误码正确", bool(bad) and bad.get("error", {}).get("code") == -32601)

    # 未知工具的隔离在协议层 handle() 完成：call_tool 抛 ToolError，handle 捕获后回 isError
    unknown_raised = False
    try:
        call_tool("not_a_tool", {})
    except ToolError:
        unknown_raised = True
    record("call_tool 对未知工具抛 ToolError", unknown_raised)

    unknown_tool = safe("未知工具经协议层隔离", lambda: handle({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": {"name": "not_a_tool", "arguments": {}}}))
    record("未知工具返回 isError 而非中断会话",
           bool(unknown_tool)
           and unknown_tool.get("result", {}).get("isError") is True
           and "未知工具" in json.dumps(unknown_tool, ensure_ascii=False))

    ports = safe("真实调用 mox_port_verify", lambda: call_tool("mox_port_verify", {"severity": ["ERROR", "WARN"], "limit": 5}))
    if ports:
        body = ports.get("structuredContent", {})
        record("端口校验返回结构化字段",
               all(k in body for k in ("passed", "error_count", "warn_count", "scanned_ports")))

    noti = safe("通知类消息不产生响应", lambda: handle({"jsonrpc": "2.0", "method": "notifications/initialized"}))
    record("通知返回 None", noti is None)

    # stdio 分帧端到端（真实子进程）
    payload = "\n".join([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
        json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                    "params": {"name": "mox_port_lookup", "arguments": {"port": 3100}}}),
    ]) + "\n"
    try:
        proc = subprocess.run([sys.executable, str(Path(__file__).resolve())],
                              input=payload.encode("utf-8"),
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
        lines = [l for l in (proc.stdout or b"").decode("utf-8", "replace").splitlines() if l.strip()]
        ok = len(lines) == 2 and json.loads(lines[1]).get("result", {}) \
            .get("structuredContent", {}).get("service", "").find("scheduler") >= 0
        record("stdio 端到端（子进程）", ok, "stdout 行数=" + str(len(lines)))
    except Exception as exc:  # noqa: BLE001
        record("stdio 端到端（子进程）", False, type(exc).__name__ + ": " + str(exc))

    failed = [c for c in checks if not c[1]]
    sys.stdout.write("=" * 70 + "\n")
    sys.stdout.write("mox-governance-mcp 自检  python=" + sys.version.split()[0] + "  repo=" + str(repo_root()) + "\n")
    sys.stdout.write("=" * 70 + "\n")
    for name, ok, detail in checks:
        sys.stdout.write("  [" + ("PASS" if ok else "FAIL") + "] " + name + (("  <- " + detail) if detail and not ok else "") + "\n")
    sys.stdout.write("-" * 70 + "\n")
    sys.stdout.write("  合计 " + str(len(checks)) + " 项，失败 " + str(len(failed)) + " 项\n")
    sys.stdout.write("=" * 70 + "\n")
    return 1 if failed else 0


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    except Exception:
        pass
    args = sys.argv[1:]
    if "--describe" in args:
        return _describe()
    if "--selftest" in args:
        return _selftest()
    if len(args) > 0 and not args[0].startswith("-"):
        sys.stderr.write("未知参数: " + args[0] + "\n")
        return 2
    return serve()


if __name__ == "__main__":
    sys.exit(main())
