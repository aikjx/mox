#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""专家联盟 · 政企场景端到端验证（支持真实模型 / mock 模型服务标注）

做什么
    对网关 `/api/alliance/*` 发起任务提交，覆盖 4 类政企场景（S1 公文起草 /
    S2 合同审查 / S4 财务分析 / S7 政策解读），每类场景指定合理的协作模式与融合策略，
    轮询至终态后拉取融合结果与各专家节点原始输出。

    默认模式（非 mock）：全程对接真实 LLM 端点，输出标注为"真实模型"。
    mock 模式（--mock 或自动检测到 :8999）：LLM 响应来自本地 mock OpenAI 兼容服务，
    但任务提交/DAG 执行/融合/持久化/回读均为真实生产代码路径——报告必须显式标注
    "mock 模型服务 · 真实生产代码路径"，不得与真实模型结果混同。

前置条件
    必须设置环境变量（缺任一即退出码 2，不伪装）：
        MOX_LLM_BASE_URL   OpenAI 兼容 API 基址（含 /v1）
        MOX_LLM_API_KEY   API 密钥
        MOX_LLM_MODEL      模型名
    可选：MOX_LLM_ENABLED（缺省启用）、MOX_LLM_TIMEOUT_MS（缺省 60000）

服务启动
    优先检测网关 :3080 是否就绪（/health 或 /api/alliance/stats）。
    未就绪时调用 scripts/startup/start-alliance-local.ps1 -SkipBuild 启动本地三进程
    （网关 33080 / 调度 33100 / 执行 33200），并把 MOX_LLM_* 传入子进程环境。
    启动后轮询最多 60s 等待就绪。本脚本不执行 cargo build。

用法
    python tools/alliance_scenario_e2e.py
    python tools/alliance_scenario_e2e.py --base-url http://127.0.0.1:3080
    python tools/alliance_scenario_e2e.py --scenarios S1,S2
    python tools/alliance_scenario_e2e.py --no-startup     # 不自动启动服务
    python tools/alliance_scenario_e2e.py --mock           # 显式进入 mock 标注模式
    # 自动检测：MOX_LLM_BASE_URL 含 127.0.0.1:8999 或 localhost:8999 时自动 mock

产物
    reports/data/<YYYYMMDD-HHMMSS>-alliance-scenario-e2e[-mock].json
        机器可读：每个场景的 task_id / status / 融合结果 / 节点结果 / 时间戳 /
        模型名 / 提供者 / HTTP 错误详情（失败项如实记录）。
        mock 模式下顶层含 "mode":"mock" 与 "note" 字段。
    reports/markdown/专家联盟-[-mock]政企场景端到端-<YYYYMMDD>.md
        人读：含 provider/model、时间戳；失败项记录 HTTP status + body。
        mock 模式下标题含"mock 模型服务 · 真实生产代码路径"并在开头加警告块。

退出码
    0 = 全部场景成功（completed）
    1 = 有场景失败（failed / cancelled / 超时 / HTTP 错误）
    2 = 环境前置未满足（缺 MOX_LLM_* / 服务未就绪）
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent

TERMINAL_STATUSES = {"completed", "failed", "cancelled"}
HEALTH_CANDIDATES = ["/health", "/api/alliance/stats", "/api/v1/status"]

# --------------------------------------------------------------------------- #
# 场景定义（内置）
# --------------------------------------------------------------------------- #
# mode / fusion_strategy 对应网关 serde 枚举名（与 alliance_demo.py 一致）。
# experts 字段仅为期望路由目标的文档记录，不直接传给 API——由调度器 matcher
# 根据 title/description 自动选专家。
SCENARIOS: List[Dict[str, Any]] = [
    {
        "id": "S1",
        "name": "政企公文起草与合规审查",
        "title": "政企公文起草-数据安全合规通知",
        "description": (
            "起草一份关于企业数据安全合规管理的内部通知公文，"
            "需法律合规审查条款合法性、学术研究支撑政策依据、创意文案优化行文风格。"
        ),
        "tags": ["公文", "合规", "政企"],
        "mode": "sequential",
        "fusion_strategy": "best_of",
        "expected_experts": ["law", "research", "creative"],
    },
    {
        "id": "S2",
        "name": "合同智能审查",
        "title": "采购合同智能审查-风险条款识别",
        "description": (
            "审查一份企业软件采购合同，需法律合规专家识别违约与免责条款风险，"
            "金融分析专家评估付款条件与财务风险，学术研究专家提供行业惯例对照。"
        ),
        "tags": ["合同", "审查", "法律"],
        "mode": "parallel",
        "fusion_strategy": "weighted",
        "expected_experts": ["law", "finance", "research"],
    },
    {
        "id": "S4",
        "name": "财务分析",
        "title": "季度财务分析-营收结构与成本趋势",
        "description": (
            "对企业季度财报进行多维度财务分析，包括营收结构拆解、成本趋势对比、"
            "数学建模预测下季度现金流，并由学术研究专家提供行业对标分析。"
        ),
        "tags": ["财务", "分析", "财报"],
        "mode": "hierarchical",
        "fusion_strategy": "stacking",
        "expected_experts": ["finance", "math", "research"],
    },
    {
        "id": "S7",
        "name": "政策解读",
        "title": "数据要素政策解读-政务数据开放",
        "description": (
            "解读国家数据要素相关政策对企业的影响，需法律合规专家分析合规义务，"
            "学术研究专家梳理政策脉络，系统架构专家评估技术落地路径。"
        ),
        "tags": ["政策", "解读", "数据要素"],
        "mode": "iterative",
        "fusion_strategy": "iterative",
        "expected_experts": ["law", "research", "arch"],
    },
]


class E2EError(Exception):
    pass


# --------------------------------------------------------------------------- #
# HTTP 客户端（对齐 alliance_demo.py GatewayClient）
# --------------------------------------------------------------------------- #
class GatewayClient:
    """极简 REST 客户端：只依赖 urllib，记录每次请求的耗时与状态码。"""

    def __init__(self, base_url: str, timeout: int, token: str = "") -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout
        self.token = token
        self.latencies: List[Tuple[str, int, int]] = []

    def request(self, method: str, path: str,
                body: Optional[Dict[str, Any]] = None) -> Tuple[int, Any]:
        url = self.base_url + path
        data = None
        headers = {"Accept": "application/json"}
        if self.token:
            headers["Authorization"] = "Bearer " + self.token
        if body is not None:
            data = json.dumps(body, ensure_ascii=False).encode("utf-8")
            headers["Content-Type"] = "application/json"
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        t0 = time.time()
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw = resp.read().decode("utf-8", "replace")
                status = resp.getcode()
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode("utf-8", "replace") if hasattr(exc, "read") else ""
            status = exc.code
        except Exception as exc:
            self.latencies.append((method + " " + path, int((time.time() - t0) * 1000), 0))
            raise E2EError(method + " " + path + " 请求失败: "
                           + type(exc).__name__ + ": " + str(exc))
        ms = int((time.time() - t0) * 1000)
        self.latencies.append((method + " " + path, ms, status))
        try:
            return status, json.loads(raw) if raw else None
        except ValueError:
            return status, raw

    def json_ok(self, method: str, path: str,
                body: Optional[Dict[str, Any]] = None) -> Any:
        status, payload = self.request(method, path, body)
        if not (200 <= status < 300):
            raise E2EError(
                method + " " + path + " 返回 HTTP " + str(status) + ": "
                + (json.dumps(payload, ensure_ascii=False)[:300]
                   if not isinstance(payload, str) else payload[:300]))
        return payload


def data_of(payload: Any) -> Dict[str, Any]:
    """网关统一响应体分层剥离：{elapsed_ms, data:{...}} → 业务数据层。"""
    layer = payload if isinstance(payload, dict) else {}
    seen = 0
    while isinstance(layer, dict) and isinstance(layer.get("data"), dict) and seen < 5:
        layer = layer["data"]
        seen += 1
    return layer if isinstance(layer, dict) else {}


def rows_of(payload: Any,
            keys: Tuple[str, ...] = ("nodes", "items", "tasks")) -> List[Any]:
    layer = data_of(payload)
    if isinstance(layer, list):
        return layer
    inner = layer.get("data") if isinstance(layer.get("data"), dict) else layer
    for holder in (layer, inner):
        if not isinstance(holder, dict):
            continue
        for k in keys:
            if isinstance(holder.get(k), list):
                return holder[k]
    return []


# --------------------------------------------------------------------------- #
# 环境变量校验
# --------------------------------------------------------------------------- #
REQUIRED_ENV = {
    "MOX_LLM_BASE_URL": "OpenAI 兼容 API 基址（含 /v1），如 https://api.openai.com/v1",
    "MOX_LLM_API_KEY": "API 密钥",
    "MOX_LLM_MODEL": "模型名，如 gpt-4o-mini",
}


def check_env() -> Dict[str, str]:
    """读取并校验 MOX_LLM_* 环境变量；缺任一则打印清晰错误并退出码 2。"""
    missing: List[str] = []
    env: Dict[str, str] = {}
    for key, desc in REQUIRED_ENV.items():
        val = os.environ.get(key, "").strip()
        if not val:
            missing.append("  " + key + " — " + desc)
        else:
            env[key] = val
    if missing:
        sys.stderr.write("[ERROR] 缺少必要环境变量（不伪装，退出码 2）:\n")
        for m in missing:
            sys.stderr.write(m + "\n")
        sys.stderr.write(
            "\n请先在 PowerShell 中设置：\n"
            "  $env:MOX_LLM_BASE_URL  = 'https://api.openai.com/v1'\n"
            "  $env:MOX_LLM_API_KEY   = 'sk-...'\n"
            "  $env:MOX_LLM_MODEL     = 'gpt-4o-mini'\n"
        )
        sys.exit(2)
    # 可选环境变量透传
    for opt in ("MOX_LLM_ENABLED", "MOX_LLM_TIMEOUT_MS"):
        v = os.environ.get(opt, "").strip()
        if v:
            env[opt] = v
    return env


def detect_mock_mode(explicit_flag: bool) -> bool:
    """判定是否进入 mock 标注模式。

    显式 --mock flag 优先；否则自动检测 MOX_LLM_BASE_URL 是否指向本地
    mock OpenAI 兼容服务（127.0.0.1:8999 或 localhost:8999）。
    """
    if explicit_flag:
        return True
    base = os.environ.get("MOX_LLM_BASE_URL", "")
    return ("127.0.0.1:8999" in base) or ("localhost:8999" in base)


# --------------------------------------------------------------------------- #
# 服务健康检测与启动
# --------------------------------------------------------------------------- #
def check_health(client: GatewayClient) -> Dict[str, Any]:
    for path in HEALTH_CANDIDATES:
        try:
            status, _ = client.request("GET", path)
            if 200 <= status < 300:
                return {"healthy": True, "probe": path, "http": status}
        except E2EError:
            continue
    return {"healthy": False, "probe": "", "http": 0}


def start_local_services(llm_env: Dict[str, str]) -> str:
    """调用 start-alliance-local.ps1 启动本地三进程，返回网关 base_url。"""
    script = REPO_ROOT / "scripts" / "startup" / "start-alliance-local.ps1"
    if not script.is_file():
        sys.stderr.write("[ERROR] 未找到启动脚本: " + str(script) + "\n")
        sys.exit(2)

    # 子进程环境 = 当前环境 + MOX_LLM_*（确保 executor 子进程继承）
    child_env = dict(os.environ)
    child_env.update(llm_env)

    sys.stdout.write("[启动] 调用 start-alliance-local.ps1 -SkipBuild …\n")
    try:
        proc = subprocess.Popen(
            ["powershell", "-ExecutionPolicy", "Bypass", "-File", str(script),
             "-SkipBuild"],
            cwd=str(REPO_ROOT),
            env=child_env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        out, _ = proc.communicate(timeout=30)
        text = out.decode("utf-8", errors="replace")
        sys.stdout.write(text[-2000:])
        if proc.returncode != 0:
            sys.stderr.write("[ERROR] 启动脚本退出码 " + str(proc.returncode) + "\n")
            sys.exit(2)
    except subprocess.TimeoutExpired:
        sys.stderr.write("[ERROR] 启动脚本超时（30s）\n")
        sys.exit(2)

    # 本地脚本默认端口 33080
    return "http://127.0.0.1:33080"


def wait_for_gateway(base_url: str, timeout_s: int, token: str = "") -> GatewayClient:
    """轮询网关就绪，超时返回 None。"""
    client = GatewayClient(base_url, timeout=10, token=token)
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        health = check_health(client)
        if health["healthy"]:
            sys.stdout.write("[就绪] 网关 " + base_url + " 探针=" + health["probe"]
                             + " HTTP " + str(health["http"]) + "\n")
            return client
        time.sleep(2)
    sys.stderr.write("[ERROR] 网关 " + base_url + " " + str(timeout_s)
                     + "s 内未就绪\n")
    sys.exit(2)


# --------------------------------------------------------------------------- #
# 单场景执行
# --------------------------------------------------------------------------- #
def run_scenario(client: GatewayClient, sc: Dict[str, Any],
                 poll_timeout: int) -> Dict[str, Any]:
    """提交一个场景任务，轮询至终态，拉取融合结果与节点结果。"""
    result: Dict[str, Any] = {
        "scenario_id": sc["id"],
        "scenario_name": sc["name"],
        "mode": sc["mode"],
        "fusion_strategy": sc["fusion_strategy"],
        "expected_experts": sc["expected_experts"],
        "submitted_at": datetime.now().isoformat(),
        "task_id": None,
        "status": None,
        "error": None,
        "http_status": None,
        "fusion_result": None,
        "nodes": [],
        "duration_ms": None,
        "model": os.environ.get("MOX_LLM_MODEL", ""),
        "provider": os.environ.get("MOX_LLM_BASE_URL", ""),
    }

    # 1) 提交任务
    create_body: Dict[str, Any] = {
        "title": sc["title"],
        "description": sc["description"],
        "task_type": "analysis",
        "priority": "high",
        "mode": sc["mode"],
        "fusion_strategy": sc["fusion_strategy"],
        "tags": sc["tags"],
    }
    t0 = time.time()
    try:
        created = data_of(client.json_ok("POST", "/api/alliance/tasks", create_body))
    except E2EError as e:
        result["error"] = str(e)
        result["status"] = "submit_failed"
        return result

    task_id = created.get("task_id")
    if not task_id:
        result["error"] = "创建任务未返回 task_id: " + json.dumps(created, ensure_ascii=False)[:300]
        result["status"] = "submit_failed"
        return result
    result["task_id"] = str(task_id)
    base = "/api/alliance/tasks/" + str(task_id)
    sys.stdout.write("  [" + sc["id"] + "] task_id=" + str(task_id) + "\n")

    # 2) 轮询至终态
    final_status = None
    deadline = time.time() + poll_timeout
    while time.time() < deadline:
        try:
            detail = data_of(client.json_ok("GET", base))
            final_status = detail.get("status")
            result["status"] = final_status
            if final_status in TERMINAL_STATUSES:
                break
        except E2EError:
            pass
        time.sleep(3)

    elapsed_ms = int((time.time() - t0) * 1000)
    result["duration_ms"] = elapsed_ms

    if final_status not in TERMINAL_STATUSES:
        result["error"] = "轮询 " + str(poll_timeout) + "s 未达终态（status=" + str(final_status) + "）"
        result["status"] = result["status"] or "timeout"
        return result

    # 3) 拉取融合结果（优先 /result，回退 /fusion-result）
    fr_payload = None
    for fr_path in ("/result", "/fusion-result"):
        try:
            status, payload = client.request("GET", base + fr_path)
            if 200 <= status < 300:
                fr_payload = data_of(payload)
                result["fusion_endpoint"] = fr_path
                break
        except E2EError:
            continue
    if fr_payload is not None:
        # 提取融合结果核心字段
        fr_inner = (fr_payload.get("fusion_result")
                    or fr_payload.get("result") or fr_payload)
        result["fusion_result"] = {
            "fusion_status": fr_payload.get("fusion_status"),
            "fusion_strategy": fr_payload.get("fusion_strategy"),
            "confidence": (fr_inner.get("confidence")
                           if isinstance(fr_inner, dict) else None),
            "key_findings": ((fr_inner.get("key_findings") or [])[:5]
                             if isinstance(fr_inner, dict) else []),
            "recommendations": ((fr_inner.get("recommendations") or [])[:3]
                                if isinstance(fr_inner, dict) else []),
            "participating_nodes": fr_payload.get("participating_nodes"),
        }

    # 4) 拉取节点结果
    try:
        nodes_payload = client.json_ok("GET", base + "/nodes")
        nodes_list = rows_of(nodes_payload)
        result["nodes"] = [
            {
                "node_id": str(n.get("node_id", "")),
                "expert_id": str(n.get("expert_id", "")),
                "name": str(n.get("name", "")),
                "status": str(n.get("status", "")),
                "duration_ms": n.get("duration_ms"),
                "error_message": n.get("error_message"),
            }
            for n in nodes_list if isinstance(n, dict)
        ]
    except E2EError as e:
        result["nodes_error"] = str(e)

    result["completed_at"] = datetime.now().isoformat()
    return result


# --------------------------------------------------------------------------- #
# 报告生成
# --------------------------------------------------------------------------- #
def git_head() -> str:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=str(REPO_ROOT), stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, timeout=10)
        if out.returncode == 0:
            return (out.stdout or b"").decode("utf-8", "replace").strip()
    except Exception:
        pass
    return ""


def build_markdown(meta: Dict[str, Any], results: List[Dict[str, Any]],
                   llm_env: Dict[str, str], is_mock: bool = False) -> str:
    lines: List[str] = []
    if is_mock:
        lines.append("# 专家联盟 · 政企场景端到端报告（mock 模型服务 · 真实生产代码路径）")
        lines.append("")
        lines.append("> ⚠️ **本报告使用本地 mock OpenAI 兼容服务（:8999），LLM 响应为 mock 数据；"
                     "任务提交/DAG 执行/融合/持久化/回读均为真实生产代码路径。**")
        lines.append("")
    else:
        lines.append("# 专家联盟 · 真实模型政企场景端到端报告")
        lines.append("")
    lines.append("> 由 `tools/alliance_scenario_e2e.py` 自动生成；"
                 "**工程事实以机器可读 JSON 证据为准**，本文件为同一批数据的可读视图。")
    lines.append("")
    lines.append("## 1. 元信息")
    lines.append("")
    lines.append("| 项 | 值 |")
    lines.append("|---|---|")
    lines.append("| 生成时间 | " + str(meta["generated_at"]) + " |")
    lines.append("| 网关地址 | " + str(meta["base_url"]) + " |")
    lines.append("| Git HEAD | " + (meta["git_head"] or "未取到") + " |")
    lines.append("| 模型 Provider | " + llm_env.get("MOX_LLM_BASE_URL", "-") + " |")
    lines.append("| 模型名 | " + llm_env.get("MOX_LLM_MODEL", "-") + " |")
    lines.append("| 场景数 | " + str(len(results)) + " |")
    ok_count = sum(1 for r in results if r.get("status") == "completed")
    lines.append("| 成功 / 总数 | " + str(ok_count) + " / " + str(len(results)) + " |")
    lines.append("")

    lines.append("## 2. 场景执行汇总")
    lines.append("")
    lines.append("| 场景 | 名称 | 模式 | 融合策略 | task_id | 终态 | 耗时(ms) |")
    lines.append("|---|---|---|---|---|---|---|")
    for r in results:
        lines.append(
            "| " + str(r.get("scenario_id", "-"))
            + " | " + str(r.get("scenario_name", "-"))
            + " | " + str(r.get("mode", "-"))
            + " | " + str(r.get("fusion_strategy", "-"))
            + " | " + str(r.get("task_id", "-"))[:12]
            + " | " + str(r.get("status", "-"))
            + " | " + str(r.get("duration_ms", "-"))
            + " |")
    lines.append("")

    lines.append("## 3. 各场景详情")
    lines.append("")
    for r in results:
        lines.append("### 3." + str(results.index(r) + 1) + " "
                     + str(r.get("scenario_id")) + " " + str(r.get("scenario_name")))
        lines.append("")
        lines.append("- task_id: `" + str(r.get("task_id", "-")) + "`")
        lines.append("- 模式: `" + str(r.get("mode")) + "` 融合策略: `"
                     + str(r.get("fusion_strategy")) + "`")
        lines.append("- 期望专家路由: " + "、".join(r.get("expected_experts", [])))
        lines.append("- 终态: **" + str(r.get("status")) + "**"
                     + "（耗时 " + str(r.get("duration_ms")) + "ms）")
        if r.get("error"):
            lines.append("- **错误**: " + str(r["error"])[:500])
        # 融合结果
        fr = r.get("fusion_result") or {}
        if fr:
            lines.append("- 融合状态: " + str(fr.get("fusion_status"))
                         + " 策略: " + str(fr.get("fusion_strategy"))
                         + " 置信度: " + str(fr.get("confidence")))
            findings = fr.get("key_findings") or []
            if findings:
                lines.append("- Key findings:")
                for f in findings[:5]:
                    lines.append("  - " + str(f)[:200])
        # 节点
        nodes = r.get("nodes") or []
        if nodes:
            lines.append("- 节点结果（" + str(len(nodes)) + " 个）:")
            lines.append("")
            lines.append("  | 节点 | 专家 | 状态 | 耗时 |")
            lines.append("  |---|---|---|---|")
            for n in nodes:
                lines.append("  | " + str(n.get("name", ""))
                             + " | " + str(n.get("expert_id", ""))
                             + " | " + str(n.get("status", ""))
                             + " | " + str(n.get("duration_ms", "-"))
                             + " |")
            # 摘录节点输出（如可获取）
            for n in nodes:
                if n.get("error_message"):
                    lines.append("  - 节点 " + str(n.get("name")) + " 错误: "
                                 + str(n["error_message"])[:200])
        lines.append("")

    lines.append("## 4. 结论")
    lines.append("")
    failed = [r for r in results if r.get("status") != "completed"]
    if not failed:
        if is_mock:
            lines.append("- 全部 " + str(len(results)) + " 个场景达到 completed 终态，"
                         "mock LLM 响应 + 真实生产代码路径已记录到 JSON 证据。")
        else:
            lines.append("- 全部 " + str(len(results)) + " 个场景达到 completed 终态，"
                         "真实模型输出已记录到 JSON 证据。")
    else:
        lines.append("- " + str(len(failed)) + " 个场景未成功:")
        for r in failed:
            lines.append("  - " + str(r.get("scenario_id")) + " "
                         + str(r.get("scenario_name")) + " — status="
                         + str(r.get("status")) + " error="
                         + str(r.get("error", ""))[:200])
    lines.append("")
    return "\n".join(lines)


# --------------------------------------------------------------------------- #
# 主流程
# --------------------------------------------------------------------------- #
def run(args: argparse.Namespace) -> int:
    # 1) 校验环境变量
    llm_env = check_env()

    # 1b) 判定 mock 标注模式（显式 flag 或自动检测 :8999）
    is_mock = detect_mock_mode(args.mock)
    if is_mock:
        sys.stdout.write("[模式] MOCK 标注 —— mock 模型服务 · 真实生产代码路径\n")
    else:
        sys.stdout.write("[模式] 真实模型\n")

    # 2) 检测网关；未就绪则启动本地服务
    base_url = args.base_url.rstrip("/")
    client = GatewayClient(base_url, args.request_timeout, token=args.token)
    health = check_health(client)

    if not health["healthy"]:
        if args.no_startup:
            sys.stderr.write("[ERROR] 网关 " + base_url + " 未就绪且 --no-startup 指定不自动启动\n")
            return 2
        sys.stdout.write("[启动] 网关 " + base_url + " 未就绪，尝试启动本地服务…\n")
        local_url = start_local_services(llm_env)
        client = wait_for_gateway(local_url, args.startup_timeout, token=args.token)
        base_url = local_url
    else:
        sys.stdout.write("[就绪] 网关 " + base_url + " 探针=" + health["probe"]
                         + " HTTP " + str(health["http"]) + "\n")

    # 3) 选择场景
    selected_ids = set(s.strip() for s in args.scenarios.split(",") if s.strip())
    scenarios = [s for s in SCENARIOS if not selected_ids or s["id"] in selected_ids]
    if not scenarios:
        sys.stderr.write("[ERROR] 无匹配场景: " + args.scenarios + "\n")
        return 2

    # 4) 逐场景执行
    results: List[Dict[str, Any]] = []
    for sc in scenarios:
        sys.stdout.write("[场景] " + sc["id"] + " " + sc["name"]
                         + " mode=" + sc["mode"]
                         + " fusion=" + sc["fusion_strategy"] + "\n")
        r = run_scenario(client, sc, args.poll_timeout)
        results.append(r)
        sys.stdout.write("  → status=" + str(r.get("status"))
                         + " duration=" + str(r.get("duration_ms")) + "ms\n")

    # 5) 写产物
    stamp = time.strftime("%Y%m%d-%H%M%S")
    today = time.strftime("%Y%m%d")
    if is_mock:
        json_path = REPO_ROOT / "reports" / "data" / (stamp + "-alliance-scenario-e2e-mock.json")
        md_path = REPO_ROOT / "reports" / "markdown" / (
            "专家联盟-mock政企场景端到端-" + today + ".md")
    else:
        json_path = REPO_ROOT / "reports" / "data" / (stamp + "-alliance-scenario-e2e.json")
        md_path = REPO_ROOT / "reports" / "markdown" / (
            "专家联盟-真实模型政企场景端到端-" + today + ".md")
    json_path.parent.mkdir(parents=True, exist_ok=True)
    md_path.parent.mkdir(parents=True, exist_ok=True)

    meta = {
        "generated_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "base_url": base_url,
        "git_head": git_head(),
        "python": sys.version.split()[0],
        "requests": len(client.latencies),
        "json_path": str(json_path.relative_to(REPO_ROOT)),
    }
    evidence: Dict[str, Any] = {
        "meta": meta,
        "llm_config": {
            "base_url": llm_env.get("MOX_LLM_BASE_URL", ""),
            "model": llm_env.get("MOX_LLM_MODEL", ""),
            "timeout_ms": llm_env.get("MOX_LLM_TIMEOUT_MS", "60000"),
        },
        "scenarios": results,
    }
    if is_mock:
        evidence["mode"] = "mock"
        evidence["note"] = "mock 模型服务 · 真实生产代码路径"
    json_path.write_text(json.dumps(evidence, ensure_ascii=False, indent=2),
                          encoding="utf-8")
    md_path.write_text(build_markdown(meta, results, llm_env, is_mock=is_mock),
                       encoding="utf-8")

    # 6) 汇总输出
    sys.stdout.write("-" * 70 + "\n")
    ok = sum(1 for r in results if r.get("status") == "completed")
    sys.stdout.write("场景=" + str(len(results)) + " 成功=" + str(ok)
                     + "  HTTP=" + str(len(client.latencies)) + "\n")
    sys.stdout.write("JSON 证据: " + str(json_path) + "\n")
    sys.stdout.write("Markdown : " + str(md_path) + "\n")
    sys.stdout.write("-" * 70 + "\n")

    return 0 if ok == len(results) else 1


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # type: ignore[attr-defined]
    except Exception:
        pass
    ap = argparse.ArgumentParser(
        description="专家联盟真实模型政企场景端到端验证")
    ap.add_argument("--base-url", dest="base_url",
                    default="http://127.0.0.1:3080",
                    help="网关地址（默认 http://127.0.0.1:3080）")
    ap.add_argument("--scenarios", default="S1,S2,S4,S7",
                    help="逗号分隔的场景 ID（S1,S2,S4,S7），默认全部")
    ap.add_argument("--poll-timeout", dest="poll_timeout", type=int, default=120,
                    help="每个场景轮询至终态的超时秒数（默认 120）")
    ap.add_argument("--request-timeout", dest="request_timeout", type=int, default=30,
                    help="单次 HTTP 请求超时秒数（默认 30）")
    ap.add_argument("--startup-timeout", dest="startup_timeout", type=int, default=60,
                    help="等待服务就绪的最大秒数（默认 60）")
    ap.add_argument("--no-startup", dest="no_startup", action="store_true",
                    help="网关未就绪时不自动启动服务（直接退出码 2）")
    ap.add_argument("--mock", action="store_true", default=False,
                    help="显式进入 mock 标注模式（报告标注 mock 模型服务 · 真实生产代码路径）；"
                         "不指定时自动检测 MOX_LLM_BASE_URL 是否含 127.0.0.1:8999")
    ap.add_argument("--token", default="dev-secret-token",
                    help="网关鉴权令牌（dev 后门令牌）")
    args = ap.parse_args()
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
