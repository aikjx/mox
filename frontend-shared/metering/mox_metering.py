#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
mox_metering.py — AI 计量归因基础设施（B-3 / G-5）

目标：
  让"专家联盟/任何 AI 调用"都能按租户做 token 计量与成本归因。
  当前仓库暂无实际 LLM 网关调用点，本模块先提供**幂等的落盘基础设施 +
  统一 JSONL 规范**，后续任何 AI 调用只需调用 record() 即可完成计量。

输出规范：
  data/metering/llm_usage.jsonl
  每行一个 JSON 对象：
  {
    "ts": "2026-08-30T12:00:00.000Z",     # UTC 时间戳
    "tenant_id": "gov-tenant",            # 租户（缺省 default-tenant）
    "user_id": "anonymous",               # 用户
    "session_id": "",                     # 会话
    "trace_id": "",                       # 链路追踪（与专家联盟 SSE 对齐）
    "service": "mox-ai-expert-svc",       # 调用来源
    "model": "unknown",                   # 模型名
    "prompt_tokens": 0,
    "completion_tokens": 0,
    "total_tokens": 0,
    "cost_est_usd": 0.0,                  # 成本估算（USD）
    "latency_ms": 0,
    "status": "ok"                        # ok / error / degraded
  }

幂等性：
  - 追加写（open "a"），不锁文件、不覆盖历史。
  - record() 任何异常都不抛出（计量不应阻断主流程）。

用法：
  import mox_metering
  mox_metering.record(tenant_id="gov-tenant", model="deepseek-v3",
                      prompt_tokens=120, completion_tokens=80, trace_id="...")

CLI 自检：
  python shared/metering/mox_metering.py --probe
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
import uuid
from datetime import datetime, timezone

# 仓库根（shared/metering/ → 上三级）
PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DEFAULT_OUT = os.path.join(PROJECT_ROOT, "data", "metering", "llm_usage.jsonl")

# 常见模型 token 单价（USD/1K tokens），供 cost 估算；未知模型记 0
_MODEL_RATE = {
    "deepseek-v3": 0.27,
    "deepseek-r1": 0.55,
    "qwen-max": 0.20,
    "qwen-plus": 0.004,
    "moonshot-v1": 0.012,
    "gpt-4o": 2.50,
    "gpt-4o-mini": 0.15,
    "claude-sonnet-4": 3.00,
    "unknown": 0.0,
}


def _utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def estimate_cost_usd(model: str, prompt_tokens: int, completion_tokens: int) -> float:
    """按 1K tokens 单价估算成本（输入按 full 价，输出按 1.5x 系数粗略折算）。"""
    rate = _MODEL_RATE.get(model or "unknown", 0.0)
    if rate <= 0:
        return 0.0
    return (prompt_tokens / 1000.0 * rate) + (completion_tokens / 1000.0 * rate * 1.5)


def record(
    tenant_id: str = "default-tenant",
    user_id: str = "anonymous",
    session_id: str = "",
    trace_id: str = "",
    service: str = "mox-ai-expert-svc",
    model: str = "unknown",
    prompt_tokens: int = 0,
    completion_tokens: int = 0,
    latency_ms: int = 0,
    status: str = "ok",
    out_path: str | None = None,
    _force_ts: str | None = None,
) -> dict | None:
    """记录一条 AI 用量。任何异常不抛出（计量不阻断业务）。"""
    try:
        total = int(prompt_tokens or 0) + int(completion_tokens or 0)
        entry = {
            "ts": _force_ts or _utc_now(),
            "tenant_id": tenant_id or "default-tenant",
            "user_id": user_id or "anonymous",
            "session_id": session_id or "",
            "trace_id": trace_id or str(uuid.uuid4()),
            "service": service or "mox-ai-expert-svc",
            "model": model or "unknown",
            "prompt_tokens": int(prompt_tokens or 0),
            "completion_tokens": int(completion_tokens or 0),
            "total_tokens": total,
            "cost_est_usd": round(estimate_cost_usd(model, int(prompt_tokens or 0), int(completion_tokens or 0)), 6),
            "latency_ms": int(latency_ms or 0),
            "status": status,
        }
        path = out_path or DEFAULT_OUT
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return entry
    except Exception as e:  # noqa: BLE001 计量失败不应阻断
        try:
            sys.stderr.write(f"[mox_metering] record failed: {e}\n")
        except Exception:
            pass
        return None


def _probe() -> int:
    """自检：写入一条样本并读回验证。"""
    e = record(
        tenant_id="gov-tenant",
        user_id="tester",
        trace_id="probe-" + uuid.uuid4().hex[:12],
        service="mox_metering.selfcheck",
        model="deepseek-v3",
        prompt_tokens=120,
        completion_tokens=80,
        latency_ms=320,
        _force_ts="2026-08-30T00:00:00.000Z",
    )
    if not e:
        print("[FAIL] record() returned None")
        return 1
    print("[OK] 样本已写入:", DEFAULT_OUT)
    print("  entry:", json.dumps(e, ensure_ascii=False))
    # 读回最后一行验证可解析
    try:
        with open(DEFAULT_OUT, "r", encoding="utf-8") as fh:
            lines = fh.read().strip().splitlines()
        last = json.loads(lines[-1])
        if last.get("tenant_id") == "gov-tenant" and last.get("total_tokens") == 200:
            print("[OK] 读回校验通过: total_tokens=200, cost_est_usd=", last["cost_est_usd"])
            return 0
        print("[FAIL] 读回校验不一致:", last)
        return 1
    except Exception as ex:
        print("[FAIL] 读回失败:", ex)
        return 1


def main() -> int:
    parser = argparse.ArgumentParser(description="MOX AI 计量归因基础设施")
    parser.add_argument("--probe", action="store_true", help="写入样本并自检")
    args = parser.parse_args()
    if args.probe:
        return _probe()
    parser.print_help()
    return 0


if __name__ == "__main__":
    sys.exit(main())
