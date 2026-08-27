# -*- coding: utf-8 -*-
"""
mox-dsql-core 动态 SQL 引擎（核心）
==================================
mox 低代码平台的执行核心：所有业务 SQL 以"定义"形式存储于元数据库，可动态配置、
动态发布，运行时经模板渲染 + 参数绑定 + 安全校验 + 缓存 + 字段级权限 后执行，
"比写死 SQL 与代码逻辑的执行速度更快"（命中缓存 O(1)，未命中也可走多级缓存）。

核心能力：
- SQL 定义管理：code 唯一标识，模板文本存库，支持版本、启停、TTL
- 模板渲染：{{param}} 参数占位、{% if param %} 条件片段、{{limit|数字}} 整数内联
- 参数绑定：统一 ? 占位符绑定，杜绝 SQL 注入；limit/offset 强制整数
- 安全校验：sanitize_sql 白名单（仅 SELECT/WITH，拦截写语句/多语句/危险关键字）
- 多级缓存：sql_code + 参数哈希 + 角色维度，命中即返回
- 字段级权限：按角色配置可见字段白名单，结果集列过滤 + 敏感字段脱敏
- 统一返回结构：trace_id / duration_ms / cache_hit，全链路可观测
"""
from __future__ import annotations

import re
import threading
import time
from typing import Any, Optional

from .cache import cache_key
from .db_adapters import sanitize_sql

# ---------------- 模板渲染器 ----------------
_TOKEN_RE = re.compile(r"\{\{(.*?)\}\}|\{%\s*if\s+([\w.]+)\s*%\}(.*?)\{%\s*endif\s*%\}", re.S)


class SqlTemplate:
    """轻量 SQL 模板：占位符 + 条件片段。生成 (sql, params)。"""

    def __init__(self, text: str):
        self._text = text

    def validate(self):
        """语法校验：占位符闭合、if/endif 成对。不要求参数存在。"""
        text = self._text
        # if/endif 配对
        opens = len(re.findall(r"\{%\s*if\s+[\w.]+\s*%\}", text))
        closes = len(re.findall(r"\{%\s*endif\s*%\}", text))
        if opens != closes:
            raise ValueError(f"模板条件块不配对: if={opens} endif={closes}")
        # 占位符闭合
        for m in re.finditer(r"\{\{", text):
            # 找到配对的 }}
            seg = text[m.end():]
            idx = seg.find("}}")
            if idx < 0:
                raise ValueError("模板占位符未闭合: " + text[m.start():m.start() + 30])
            expr = seg[:idx].strip()
            if not expr:
                raise ValueError("模板占位符为空")
        # 多余的 }}
        for m in re.finditer(r"\}\}", text):
            pass  # 允许文本中独立出现
        return True

    def render(self, params: dict) -> tuple[str, list]:
        sql_parts: list[str] = []
        bind: list = []

        def resolve_expr(expr: str) -> str:
            expr = expr.strip()
            # 默认值语法：{{key|default}}
            key, _, default = expr.partition("|")
            key = key.strip()
            if key in ("limit", "offset"):
                # 分页参数强制整数内联，防止注入
                try:
                    v = int(params.get(key, default or (20 if key == "limit" else 0)))
                except (TypeError, ValueError):
                    raise ValueError(f"参数 {key} 必须是整数")
                return str(max(0, v))
            if key in params:
                bind.append(params[key])
            elif default != "":
                bind.append(default)
            else:
                raise ValueError(f"缺少必填参数: {key}")
            return "?"

        pos = 0
        for m in _TOKEN_RE.finditer(self._text):
            sql_parts.append(self._text[pos:m.start()])
            if m.group(1) is not None:
                sql_parts.append(resolve_expr(m.group(1)))
            else:
                cond_key = m.group(2).strip()
                inner = m.group(3)
                if _is_truthy(params.get(cond_key)):
                    # 条件命中：递归渲染内部片段
                    inner_sql, inner_bind = SqlTemplate(inner).render(params)
                    sql_parts.append(inner_sql)
                    bind.extend(inner_bind)
            pos = m.end()
        sql_parts.append(self._text[pos:])
        return "".join(sql_parts), bind


def _is_truthy(v: Any) -> bool:
    if v is None:
        return False
    if isinstance(v, bool):
        return v
    if isinstance(v, (int, float)):
        return v != 0
    if isinstance(v, str):
        return v.strip() != "" and v.lower() not in ("false", "0", "null", "none")
    if isinstance(v, (list, dict, tuple, set)):
        return len(v) > 0
    return True


# ---------------- 字段级权限 ----------------
_MASK_FIELDS = {
    "phone": lambda v: _mask_str(v, 3, 4),
    "mobile": lambda v: _mask_str(v, 3, 4),
    "id_card": lambda v: _mask_str(v, 6, 4),
    "email": lambda v: v.split("@")[0][:2] + "***@" + v.split("@")[-1] if "@" in v else v,
    "password": lambda v: "******",
    "password_hash": lambda v: "******",
}


def _mask_str(v: Any, keep_head: int, keep_tail: int) -> Any:
    s = str(v)
    if len(s) <= keep_head + keep_tail:
        return "*" * len(s)
    return s[:keep_head] + "*" * (len(s) - keep_head - keep_tail) + s[-keep_tail:]


def apply_field_permission(rows: list[dict], allowed_fields: Optional[list[str]],
                           mask: bool = True) -> list[dict]:
    """结果集字段级过滤 + 敏感脱敏。allowed_fields=None 表示全部可见。"""
    out: list[dict] = []
    for row in rows:
        if allowed_fields is None:
            item = dict(row)
        else:
            item = {k: row.get(k) for k in allowed_fields if k in row}
        if mask:
            for k, v in item.items():
                lk = k.lower()
                for pat, fn in _MASK_FIELDS.items():
                    if pat in lk and v not in (None, ""):
                        try:
                            item[k] = fn(v)
                        except Exception:
                            pass
                        break
        out.append(item)
    return out


# ---------------- DSQL 引擎 ----------------
class DsqlEngine:
    """动态 SQL 执行引擎。依赖：元数据库(meta)、缓存(cache)、适配器注册表(adapters)。"""

    def __init__(self, meta: Any, cache: Any, adapters: dict[str, Any]):
        self._meta = meta            # 元数据库（提供 query/execute）
        self._cache = cache          # CacheAdapter
        self._adapters = adapters    # {datasource_id: DBAdapter}
        self._def_cache: dict[str, tuple[float, dict]] = {}
        self._def_cache_ttl = 30.0
        self._lock = threading.RLock()

    # ---- SQL 定义管理 ----
    def _load_def(self, code: str) -> dict:
        with self._lock:
            now = time.time()
            hit = self._def_cache.get(code)
            if hit and now - hit[0] < self._def_cache_ttl:
                return hit[1]
        rows = self._meta.query(
            "SELECT * FROM dsql_sqls WHERE code=?", [code])
        if not rows:
            raise KeyError(f"SQL 定义不存在: {code}")
        row = rows[0]
        if row.get("status") != "published":
            raise ValueError(f"SQL 定义未发布: {code}")
        defn = {
            "code": row["code"],
            "name": row.get("name", ""),
            "template": row.get("template", ""),
            "datasource": row.get("datasource", "default"),
            "cache_ttl": int(row.get("cache_ttl") or 0),
            "version": row.get("version", 1),
            "description": row.get("description", ""),
        }
        with self._lock:
            self._def_cache[code] = (time.time(), defn)
        return defn

    def invalidate_def(self, code: Optional[str] = None):
        with self._lock:
            if code:
                self._def_cache.pop(code, None)
            else:
                self._def_cache.clear()

    def list_defs(self) -> list[dict]:
        return self._meta.query(
            "SELECT code,name,datasource,cache_ttl,status,version,description,updated_at "
            "FROM dsql_sqls ORDER BY updated_at DESC")

    def upsert_def(self, code: str, name: str, template: str, datasource: str,
                   cache_ttl: int = 0, status: str = "draft", version: int = None,
                   description: str = "") -> dict:
        # 发布前先做语法校验 + 安全校验，保证入库即可用
        SqlTemplate(template).validate()  # 语法校验（不要求参数存在）
        sanitize_sql(template)            # 只读白名单校验
        rows = self._meta.query("SELECT version FROM dsql_sqls WHERE code=?", [code])
        if rows:
            new_version = (rows[0]["version"] or 0) + 1 if version is None else version
            self._meta.execute(
                "UPDATE dsql_sqls SET name=?, template=?, datasource=?, cache_ttl=?, "
                "status=?, version=?, description=?, updated_at=? WHERE code=?",
                [name, template, datasource, int(cache_ttl), status, new_version,
                 description, int(time.time()), code])
            saved_version = new_version
        else:
            saved_version = version or 1
            self._meta.execute(
                "INSERT INTO dsql_sqls(code,name,template,datasource,cache_ttl,status,"
                "version,description,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)",
                [code, name, template, datasource, int(cache_ttl), status, saved_version,
                 description, int(time.time()), int(time.time())])
        self.invalidate_def(code)
        return {"code": code, "version": saved_version, "status": status}

    def set_status(self, code: str, status: str) -> dict:
        self._meta.execute("UPDATE dsql_sqls SET status=?, updated_at=? WHERE code=?",
                           [status, int(time.time()), code])
        self.invalidate_def(code)
        return {"code": code, "status": status}

    def delete_def(self, code: str) -> int:
        self.invalidate_def(code)
        return self._meta.execute("DELETE FROM dsql_sqls WHERE code=?", [code])["rows_affected"]

    # ---- 权限 ----
    def get_field_permission(self, resource: str, role: str) -> Optional[list[str]]:
        """按 sql_code + role 返回可见字段白名单；无配置 -> None（全部可见）。"""
        rows = self._meta.query(
            "SELECT allowed_fields FROM field_permissions WHERE resource=? AND role=?",
            [resource, role])
        if not rows:
            return None
        raw = rows[0].get("allowed_fields") or ""
        return [f.strip() for f in raw.split(",") if f.strip()]

    # ---- 执行 ----
    def execute(self, code: str, params: Optional[dict] = None, role: str = "anonymous",
                use_cache: bool = True) -> dict:
        params = params or {}
        t0 = time.time()
        trace_id = "tr-" + hex(int(time.time() * 1_000_000))[2:] + "-" + code[:12]
        # 1. 加载定义
        defn = self._load_def(code)
        # 2. 渲染模板 -> (sql, bind)
        rendered_sql, bind = SqlTemplate(defn["template"]).render(params)
        # 3. 安全校验
        safe_sql = sanitize_sql(rendered_sql)
        # 4. 缓存键（含参数 + 角色）
        ck = cache_key("dsql:" + code, {"p": params, "role": role})
        if use_cache and defn["cache_ttl"] > 0:
            cached = self._cache.get(ck)
            if cached is not None:
                return {
                    "success": True, "code": code, "message": "ok(cache)",
                    "data": cached, "trace_id": trace_id,
                    "duration_ms": round((time.time() - t0) * 1000, 2),
                    "cache_hit": True, "from": "cache",
                }
        # 5. 取数据源适配器
        adapter = self._adapters.get(defn["datasource"])
        if adapter is None:
            raise ValueError(f"数据源未配置: {defn['datasource']}")
        # 6. 执行
        rows = adapter.query(safe_sql, bind)
        # 7. 字段级权限过滤 + 脱敏
        allowed = self.get_field_permission(code, role)
        rows = apply_field_permission(rows, allowed)
        # 8. 写缓存
        if use_cache and defn["cache_ttl"] > 0:
            self._cache.set(ck, rows, defn["cache_ttl"])
        return {
            "success": True, "code": code, "message": "ok",
            "data": rows, "trace_id": trace_id,
            "duration_ms": round((time.time() - t0) * 1000, 2),
            "cache_hit": False, "from": "db",
            "sql": safe_sql, "row_count": len(rows),
        }

    def explain(self, code: str, params: Optional[dict] = None) -> dict:
        """渲染 + 校验 + 字段权限预览，不真正执行。"""
        params = params or {}
        defn = self._load_def(code)
        rendered_sql, bind = SqlTemplate(defn["template"]).render(params)
        safe_sql = sanitize_sql(rendered_sql)
        allowed = self.get_field_permission(code, "anonymous")
        return {
            "code": code, "version": defn["version"], "datasource": defn["datasource"],
            "rendered_sql": safe_sql, "bind_params": bind,
            "cache_ttl": defn["cache_ttl"], "allowed_fields": allowed,
        }
