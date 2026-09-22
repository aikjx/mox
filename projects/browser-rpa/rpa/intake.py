from __future__ import annotations

import difflib
import re
from typing import Any, Dict, List, Optional

from . import config, llm

URL_RE = re.compile(r"https?://[^\s'\"，。；;、）)】》]+")

# 归一化词表：字段别名 → 规范字段
FIELD_ALIASES = {
    "url": ("url", "target_url", "网址", "地址", "链接", "页面"),
    "requirement": ("requirement", "requirement_text", "需求", "描述", "目标"),
    "title": ("title", "name", "任务名", "名称", "标题"),
    "interval": ("interval", "schedule", "周期", "频率", "间隔"),
}

# 意图词表：自然语言动词 → 规范动作
INTENT_MAP = [
    ("goto", ("打开", "访问", "进入", "navigate", "open")),
    ("click", ("点击", "按下", "提交", "click", "press")),
    ("fill", ("输入", "填写", "搜索", "键入", "fill", "type")),
    ("wait", ("等待", "暂停", "wait")),
    ("extract", ("抓取", "采集", "提取", "爬取", "extract", "scrape")),
    ("login", ("登录", "登陆", "signin", "login")),
    ("screenshot", ("截图", "screenshot")),
]

_INTERVAL_RE = re.compile(r"(每|every)\s*(\d+)?\s*(秒|分钟|分|小时|时|天|min|minute|hour|day|s\b)?")


def normalize_interval(text: str) -> Optional[int]:
    """把「每30分钟 / 每小时 / every 2 hours」归一化为间隔秒数。"""
    m = _INTERVAL_RE.search(text or "")
    if not m:
        return None
    unit = (m.group(3) or "分钟").lower()
    n = int(m.group(2) or 1)
    factors = {"秒": 1, "s": 1, "分钟": 60, "分": 60, "min": 60, "minute": 60,
               "小时": 3600, "时": 3600, "hour": 3600, "day": 86400, "天": 86400}
    return n * factors.get(unit, 60)


def extract_actions(text: str) -> List[Dict[str, str]]:
    actions = []
    for intent, verbs in INTENT_MAP:
        for v in verbs:
            if v in text:
                actions.append({"intent": intent, "evidence": v})
                break
    return actions


def _canonical_key(raw: str) -> Optional[str]:
    for canon, aliases in FIELD_ALIASES.items():
        if raw in aliases:
            return canon
        if difflib.get_close_matches(raw.strip().lower(), list(aliases), n=1, cutoff=0.8):
            return canon
    return None


def rule_intake(raw: Dict[str, Any]) -> Dict[str, Any]:
    """离线规则回退：从自由文本/键值杂项归一化为 TaskSpec。"""
    blob_parts = [str(v) for v in raw.values()]
    for k, v in raw.items():
        ck = _canonical_key(str(k))
        if ck == "requirement" or ck is None:
            blob_parts.append(str(v))
    blob = " ".join(blob_parts)
    requirement = ""
    for k, v in raw.items():
        if _canonical_key(str(k)) == "requirement" and str(v).strip():
            requirement = str(v).strip()
            break
    requirement = requirement or blob.strip()
    urls = URL_RE.findall(blob)
    title = ""
    for k, v in raw.items():
        if _canonical_key(str(k)) == "title":
            title = str(v).strip()
            break
    return {
        "schema": config.SCHEMA,
        "title": title or (urls[0] if urls else requirement[:30]),
        "requirement": requirement,
        "url": urls[0] if urls else "",
        "actions": extract_actions(requirement),
        "schedule": {"interval_sec": normalize_interval(blob)},
        "tags": sorted({a["intent"] for a in extract_actions(blob)}),
        "heal": {"enabled": True, "max_attempts": config.HEAL_MAX_ATTEMPTS},
    }


SYSTEM_PROMPT = (
    "你是 RPA 需求归一化器。把用户的自然语言 RPA 需求归一化为 JSON TaskSpec，schema 为 "
    + config.SCHEMA + "。字段：title(短标题), requirement(需求原文), url(目标网址,无则空串), "
    "actions(数组,元素{intent,url?,selector_hint?,value?},intent 只能取 goto/click/fill/wait/extract/login/screenshot), "
    "schedule({interval_sec:整数秒或null}), tags(字符串数组), heal({enabled:bool,max_attempts:整数})。"
    "只输出 JSON，不要解释。"
)


def intake(requirement: Any) -> Dict[str, Any]:
    """需求归一化入口：接受字符串或字典，LLM 优先，失败走规则回退。"""
    raw = {"requirement": requirement} if isinstance(requirement, str) else dict(requirement)
    spec = None
    if llm.available():
        spec = llm.chat_json(SYSTEM_PROMPT, str(raw))
        if spec and not URL_RE.search(str(spec.get("url", ""))):
            spec["url"] = rule_intake(raw)["url"] or spec.get("url", "")
    if not spec:
        spec = rule_intake(raw)
    base = rule_intake(raw)
    merged = dict(base)
    for k, v in spec.items():
        if v not in (None, "", [], {}):
            merged[k] = v
    merged["schema"] = config.SCHEMA
    if merged.get("url") and not any(a["intent"] == "goto" for a in merged.get("actions", [])):
        merged.setdefault("actions", []).insert(0, {"intent": "goto", "evidence": "url"})
    merged["tags"] = sorted(set(merged.get("tags", []) + base["tags"]))
    return merged
