# -*- coding: utf-8 -*-
"""
mox-ai 智能助手引擎
===================
mox 管理中心中台"结合 AI"能力。无外部 LLM 时使用内置意图引擎（零依赖、真实可用），
配置 MOX_LLM_URL / MOX_LLM_KEY 后可切换到大模型。

能力：
- nl2sql    ：自然语言 → SQL 模板 + 参数说明 + 可试运行 SQL（针对企业官网业务库）
- explain_sql：解释一段 SQL 的目标/来源/条件/排序/限制
- suggest_sql：SQL 优化建议（全扫/select*/like前置%/缺limit/可缓存）
- assistant  ：统一入口，返回结构 {reply, sql, params, actions}
"""
from __future__ import annotations

import json
import os
import re
import time
import urllib.request
from typing import Any, Optional

# ---------------- 内置意图库（企业官网业务域） ----------------
SCHEMA_HINT = {
    "products": "products(id,name,category,price,image,summary,specs_json,hot,recommend)",
    "news": "news(id,title,category,date,views,image,summary,content)",
    "cases": "cases(id,title,customer,industry,image,summary,background,solution,results_json)",
    "team": "team(id,name,role,bio,avatar)",
    "messages": "messages(id,name,phone,email,company,content,status,created_at)",
}

# (意图正则, 动作key)
_INTENTS = [
    (re.compile(r"(统计|多少|数量|总数|count)"), "count"),
    (re.compile(r"(搜索|查找|搜一?下|查一下)"), "search"),
    (re.compile(r"(留言|反馈|消息)"), "message"),
    (re.compile(r"(团队|人员|成员|同事)"), "team"),
    (re.compile(r"(案例|客户|项目)"), "case"),
    (re.compile(r"(新闻|资讯|动态|公告)"), "news"),
    (re.compile(r"(产品|商品|服务)"), "product"),
]

_COND_CTX = [
    (re.compile(r"分类[:为是]?\s*([\w\u4e00-\u9fa5-]+)"), "category"),
    (re.compile(r"行业[:为是]?\s*([\w\u4e00-\u9fa5-]+)"), "industry"),
    (re.compile(r"状态[:为是]?\s*([\w\u4e00-\u9fa5-]+)"), "status"),
    (re.compile(r"关键字[:为是]?\s*([\w\u4e00-\u9fa5-]+)"), "keyword"),
]
_LIMIT_CTX = [
    (re.compile(r"(最新|最近|热门)"), "order_recent"),
    (re.compile(r"前\s*(\d+)\s*(条|个)"), "limit_n"),
    (re.compile(r"(\d+)\s*(条|个)"), "limit_n"),
]


def _first_match(text: str, patterns) -> Optional[tuple]:
    for pat, key in patterns:
        m = pat.search(text)
        if m:
            return m, key
    return None


def nl2sql(text: str) -> dict:
    """自然语言 → 动作意图 + 生成 SQL 模板与可试运行 SQL。"""
    text = (text or "").strip()
    intent = None
    for pat, key in _INTENTS:
        if pat.search(text):
            intent = key
            break
    intent = intent or "product"

    conds = {}
    for pat, key in _COND_CTX:
        m = pat.search(text)
        if m:
            conds[key] = m.group(1).strip()
    limit_n = None
    order = ""
    for pat, key in _LIMIT_CTX:
        m = pat.search(text)
        if m:
            if key == "order_recent":
                order = " ORDER BY date DESC" if intent in ("news",) else " ORDER BY id DESC"
            elif key == "limit_n":
                limit_n = int(m.group(1))

    # 组装模板
    params: dict = {}
    sql_tpl = ""
    table = {"product": "products", "news": "news", "case": "cases",
             "team": "team", "message": "messages"}.get(intent, "products")
    where = []
    if intent == "search":
        sql_tpl = (f"SELECT id,name AS title,summary AS snippet FROM products "
                   f"WHERE name LIKE {{keyword}} OR summary LIKE {{keyword}} "
                   f"UNION ALL SELECT id,title,summary FROM news WHERE title LIKE {{keyword}} "
                   f"UNION ALL SELECT id,title,summary FROM cases WHERE title LIKE {{keyword}}")
        params["keyword"] = "%关键词%"
    else:
        if intent == "count":
            sql_tpl = f"SELECT COUNT(*) AS cnt FROM {table}"
        elif intent == "team":
            sql_tpl = "SELECT id,name,role,bio,avatar FROM team ORDER BY id ASC"
        elif intent == "message":
            sql_tpl = ("SELECT id,name,phone,company,content,status,created_at FROM messages "
                       "WHERE 1=1 {% if status %} AND status={{status}} {% endif %} ORDER BY created_at DESC")
            if "status" in conds:
                params["status"] = conds["status"]
        elif intent == "case":
            sql_tpl = ("SELECT id,title,customer,industry,image,summary FROM cases WHERE 1=1 "
                       "{% if industry %} AND industry={{industry}} {% endif %} ORDER BY id ASC")
            if "industry" in conds:
                params["industry"] = conds["industry"]
        elif intent == "news":
            sql_tpl = ("SELECT id,title,category,date,views,image,summary FROM news WHERE 1=1 "
                       "{% if category %} AND category={{category}} {% endif %} ORDER BY date DESC")
            if "category" in conds:
                params["category"] = conds["category"]
        else:  # product
            sql_tpl = ("SELECT id,name,category,price,image,summary,hot FROM products WHERE 1=1 "
                       "{% if category %} AND category={{category}} {% endif %} "
                       "{% if keyword %} AND (name LIKE {{keyword}} OR summary LIKE {{keyword}}) {% endif %} "
                       "ORDER BY id ASC")
            if "category" in conds:
                params["category"] = conds["category"]
            if "keyword" in conds:
                params["keyword"] = "%" + conds["keyword"] + "%"
    # LIMIT
    if intent == "count":
        pass
    elif limit_n:
        sql_tpl += f" LIMIT {{limit}}"
        params["limit"] = limit_n

    # 试运行参数（用默认值渲染成可执行 SQL 与绑定值）
    run_params = {k: (1 if k in ("id",) else v) for k, v in params.items()}
    return {
        "intent": intent,
        "table": table,
        "conditions": conds,
        "order": order.strip() or "默认",
        "limit": limit_n,
        "sql_template": sql_tpl,
        "params": params,
        "params_desc": _describe_params(params),
    }


def _describe_params(params: dict) -> list[dict]:
    return [{"key": k, "example": v, "desc": {
        "category": "产品/新闻分类", "industry": "案例行业", "status": "留言状态(待处理/已联系)",
        "keyword": "模糊关键字", "limit": "返回条数(整数)"}.get(k, "业务参数")} for k, v in params.items()]


def explain_sql(sql: str) -> str:
    """结构化解释一段 SQL。"""
    sql = re.sub(r"\s+", " ", (sql or "")).strip()
    if not sql:
        return "（空 SQL）"
    out = []
    m = re.search(r"SELECT\s+(.+?)\s+FROM\s+([^\s]+)", sql, re.I)
    if m:
        fields, table = m.group(1), m.group(2)
        if fields.strip() == "*":
            out.append(f"目标表：**{table}**，查询全部字段")
        else:
            out.append(f"目标表：**{table}**，查询字段：`{fields}`")
    m = re.search(r"WHERE\s+(.+?)(\sORDER\sBY|\sLIMIT\s|$)", sql, re.I | re.S)
    if m and m.group(1).strip():
        out.append(f"过滤条件：`{m.group(1).strip()}`")
    m = re.search(r"ORDER\s+BY\s+([^\s]+)\s*(ASC|DESC)?", sql, re.I)
    if m:
        out.append(f"排序：按 `{m.group(1)}` {'升序' if (m.group(2) or 'ASC').upper() != 'DESC' else '降序'}")
    m = re.search(r"LIMIT\s+(\d+)", sql, re.I)
    if m:
        out.append(f"限制返回：{m.group(1)} 条")
    if not out:
        return "（无法识别的 SQL 结构）"
    return "；".join(out)


def suggest_sql(sql: str) -> list[dict]:
    """SQL 优化建议（返回 建议+理由 列表）。"""
    sql = (sql or "").strip()
    if not sql:
        return []
    tips = []
    if re.search(r"SELECT\s+\*", sql, re.I):
        tips.append({"level": "warn", "title": "避免 SELECT *", "reason": "建议按需列出字段，减少网络与内存开销"})
    if not re.search(r"\bWHERE\b", sql, re.I):
        tips.append({"level": "danger", "title": "缺少 WHERE，将全表扫描", "reason": "请添加过滤条件或考虑行级安全"})
    if re.search(r"LIKE\s+['\"]%", sql, re.I):
        tips.append({"level": "warn", "title": "LIKE 前置 % 无法走索引", "reason": "前置通配符会导致全表扫描，可考虑全文索引"})
    if not re.search(r"\bLIMIT\b", sql, re.I):
        tips.append({"level": "info", "title": "建议限制返回条数", "reason": "大数据量下使用 LIMIT 提升响应与内存效率"})
    if re.search(r"\bJOIN\b", sql, re.I):
        tips.append({"level": "info", "title": "存在 JOIN", "reason": "可为关联列建立索引；复杂关联可考虑知识图谱多跳查询"})
    if not tips:
        tips.append({"level": "ok", "title": "SQL 结构良好", "reason": "已避免明显性能反模式"})
    return tips


def _call_llm(prompt: str) -> Optional[str]:
    url = os.environ.get("MOX_LLM_URL")
    key = os.environ.get("MOX_LLM_KEY")
    if not url:
        return None
    payload = {
        "model": os.environ.get("MOX_LLM_MODEL", "gpt-4o-mini"),
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.2,
    }
    req = urllib.request.Request(url, data=json.dumps(payload).encode("utf-8"),
                                 headers={"Content-Type": "application/json",
                                          "Authorization": "Bearer " + key})
    with urllib.request.urlopen(req, timeout=20) as resp:
        j = json.loads(resp.read().decode("utf-8"))
        return j["choices"][0]["message"]["content"]


def assistant(message: str, app_key: str = "mox") -> dict:
    """统一助手入口。返回 {reply, sql, sql_explain, suggestions, actions}。"""
    t0 = time.time()
    # 优先外部 LLM（若配置）
    llm_reply = _call_llm("你是企业官网低代码平台的 AI 运维助手，请简洁回答：\n" + message)
    if llm_reply:
        return {
            "engine": "llm", "reply": llm_reply, "sql": None, "sql_explain": None,
            "suggestions": [], "actions": [], "duration_ms": round((time.time() - t0) * 1000, 2),
        }
    # 内置引擎
    msg = message.lower()
    if "解释" in msg or "explain" in msg:
        m = re.search(r"解释[：:]?\s*(.+)$", message)
        sql = m.group(1) if m else message.replace("解释", "").strip()
        return {"engine": "rule", "reply": "SQL 解释：\n" + explain_sql(sql),
                "sql": sql, "sql_explain": explain_sql(sql), "suggestions": suggest_sql(sql),
                "actions": [], "duration_ms": round((time.time() - t0) * 1000, 2)}
    if "优化" in msg or "建议" in msg or "suggest" in msg:
        m = re.search(r"(?:优化|建议)[：:]?\s*(.+)$", message)
        sql = m.group(1) if m else None
        if not sql:
            return {"engine": "rule", "reply": "请提供需要优化的 SQL，例如：优化 SELECT * FROM products",
                    "sql": None, "sql_explain": None, "suggestions": [], "actions": [],
                    "duration_ms": round((time.time() - t0) * 1000, 2)}
        return {"engine": "rule", "reply": "SQL 优化建议：", "sql": sql,
                "sql_explain": explain_sql(sql), "suggestions": suggest_sql(sql), "actions": [],
                "duration_ms": round((time.time() - t0) * 1000, 2)}
    # 默认：自然语言生成 SQL
    r = nl2sql(message)
    action = None
    if not r["sql_template"].startswith("SELECT COUNT"):
        action = {"label": "生成并试运行", "sql_code": "ai:" + r["intent"], "params": r["params"]}
    reply = (f"已将需求解析为 **{r['table']}** 域查询（意图：{r['intent']}）\n"
             f"生成 SQL 模板如下，可在右侧 SQL 管理中按此模板落库发布，或点击「生成并试运行」立即验证。")
    return {"engine": "rule", "reply": reply, "sql": r["sql_template"],
            "sql_explain": explain_sql(r["sql_template"].replace("{{", "").replace("}}", "")),
            "suggestions": suggest_sql(r["sql_template"].replace("{{", "").replace("}}", "")),
            "params": r["params"], "actions": [action] if action else [],
            "duration_ms": round((time.time() - t0) * 1000, 2)}
