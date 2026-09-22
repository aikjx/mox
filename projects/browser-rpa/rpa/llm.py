from __future__ import annotations

import json
import re
from typing import Any, Dict, Optional

import httpx

from . import config


def available() -> bool:
    return bool(config.LLM_BASE_URL)


def chat(messages, temperature: float = 0.0) -> Optional[str]:
    """调用 OpenAI 兼容 /chat/completions；未配置或失败返回 None（调用方走离线回退）。"""
    if not available():
        return None
    url = config.LLM_BASE_URL + "/chat/completions"
    headers = {"Content-Type": "application/json"}
    if config.LLM_API_KEY:
        headers["Authorization"] = "Bearer " + config.LLM_API_KEY
    body = {"model": config.LLM_MODEL, "messages": messages, "temperature": temperature}
    try:
        resp = httpx.post(url, json=body, headers=headers, timeout=config.LLM_TIMEOUT)
        resp.raise_for_status()
        return resp.json()["choices"][0]["message"]["content"]
    except Exception:
        return None


_JSON_FENCE = re.compile(r"```(?:json)?\s*(.*?)```", re.S)


def extract_json(text: str) -> Optional[Any]:
    """从 LLM 回复中稳健提取 JSON（支持裸 JSON 与代码围栏）。"""
    if not text:
        return None
    m = _JSON_FENCE.search(text)
    candidate = m.group(1) if m else text
    candidate = candidate.strip()
    try:
        return json.loads(candidate)
    except json.JSONDecodeError:
        pass
    start = candidate.find("{")
    end = candidate.rfind("}")
    if start >= 0 and end > start:
        try:
            return json.loads(candidate[start:end + 1])
        except json.JSONDecodeError:
            return None
    return None


def chat_json(system: str, user: str) -> Optional[Dict[str, Any]]:
    reply = chat([{"role": "system", "content": system}, {"role": "user", "content": user}])
    data = extract_json(reply or "")
    return data if isinstance(data, dict) else None
