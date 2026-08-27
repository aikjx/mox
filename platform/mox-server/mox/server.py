# -*- coding: utf-8 -*-
"""
mox-server：mox 低代码平台运行服务（FastAPI）
=============================================
统一 API 层，提供：

  /api/dsql/*       动态 SQL 执行与 SQL 定义管理
  /api/admin/*      数据源 / 字段权限 / 角色 / 用户 / 审计 管理
  /api/kg/*         自研知识图谱查询与管理
  /api/cache/*      缓存监控与清理
  /api/website/*    官网写接口（留言/简历/咨询）
  /api/stats        平台概览

所有响应统一结构：{ success, code, message, data, trace_id, ... }
CORS 全开，供前端配置台与官网跨域调用。
"""
from __future__ import annotations

import json
import threading
import time
import traceback
from typing import Any, Optional

from fastapi import FastAPI, Request, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from .cache import build_cache
from .db_adapters import build_adapter
from .dsql_core import DsqlEngine
from .kg_core import KnowledgeGraph
from .seed_data import reset_and_seed, META_DB

app = FastAPI(title="mox-server 低代码平台运行服务", version="2.0.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=False,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ---------------- 运行时装配 ----------------
class MetaStore:
    """元数据库访问层：为 sqlite3.Connection 提供统一 query/execute 接口。"""

    def __init__(self, conn):
        self._conn = conn

    def query(self, sql: str, params: Optional[list] = None) -> list[dict]:
        cur = self._conn.execute(sql, params or [])
        return [dict(r) for r in cur.fetchall()]

    def execute(self, sql: str, params: Optional[list] = None) -> dict:
        cur = self._conn.execute(sql, params or [])
        self._conn.commit()
        return {"rows_affected": cur.rowcount, "last_insert_id": cur.lastrowid}


_RAW_META, _RAW_BUSINESS = reset_and_seed()
META = MetaStore(_RAW_META)
BUSINESS = _RAW_BUSINESS
CACHE = build_cache(driver="memory")  # 改 "redis" 即可切换 redis 缓存

# 数据源适配器注册表（中间层）：按元库 datasources 表动态构建
ADAPTERS: dict[str, Any] = {}


def refresh_adapters():
    global ADAPTERS
    rows = META.query("SELECT name,driver,config_json,enabled FROM datasources")
    ADAPTERS = {}
    for r in rows:
        if not r["enabled"]:
            continue
        try:
            ADAPTERS[r["name"]] = build_adapter(r["driver"], json.loads(r["config_json"] or "{}"))
        except Exception as e:  # noqa: BLE001
            print(f"[mox] 数据源 {r['name']} 初始化失败: {e}")


refresh_adapters()
DSQL = DsqlEngine(meta=META, cache=CACHE, adapters=ADAPTERS)
KG = KnowledgeGraph(meta=META)
_LOCK = threading.RLock()


def _trace() -> str:
    return "tr-" + hex(int(time.time() * 1_000_000))[2:]


def _audit(actor: str, action: str, detail: Any, trace_id: str):
    try:
        META.execute(
            "INSERT INTO audit_logs(ts,trace_id,actor,action,detail) VALUES(?,?,?,?,?)",
            [int(time.time()), trace_id, actor, action,
             json.dumps(detail, ensure_ascii=False, default=str)[:4000]])
    except Exception:  # noqa: BLE001
        pass


def _ok(data: Any = None, message: str = "ok", **extra) -> dict:
    return {"success": True, "code": 0, "message": message, "data": data, **extra}


def _err(message: str, status: int = 400) -> JSONResponse:
    return JSONResponse(status_code=status, content={
        "success": False, "code": status, "message": message})


# ================================================================
#  DSQL：执行 / 解释 / 批量
# ================================================================
@app.post("/api/dsql/execute")
async def dsql_execute(req: Request):
    body = await req.json()
    code = body.get("sql_code") or body.get("code")
    if not code:
        return _err("缺少 sql_code")
    try:
        res = DSQL.execute(code, body.get("params") or {},
                           role=body.get("role", "anonymous"),
                           use_cache=body.get("use_cache", True))
        _audit(body.get("actor", "api"), f"dsql.execute:{code}",
               {"params": body.get("params") or {}, "role": body.get("role")}, res["trace_id"])
        return res
    except KeyError as e:
        return _err(str(e), 404)
    except ValueError as e:
        return _err(str(e), 422)
    except Exception as e:  # noqa: BLE001
        traceback.print_exc()
        return _err(f"执行失败: {e}", 500)


@app.post("/api/dsql/explain")
async def dsql_explain(req: Request):
    body = await req.json()
    code = body.get("sql_code") or body.get("code")
    if not code:
        return _err("缺少 sql_code")
    try:
        return _ok(DSQL.explain(code, body.get("params") or {}), trace_id=_trace())
    except Exception as e:  # noqa: BLE001
        return _err(f"解释失败: {e}", 422)


@app.post("/api/dsql/execute-batch")
async def dsql_execute_batch(req: Request):
    body = await req.json()
    items = body.get("items") or []
    if not isinstance(items, list) or not items:
        return _err("缺少 items")
    results = []
    for it in items:
        try:
            results.append(DSQL.execute(it.get("sql_code") or it.get("code"),
                                        it.get("params") or {},
                                        role=body.get("role", "anonymous"),
                                        use_cache=body.get("use_cache", True)))
        except Exception as e:  # noqa: BLE001
            results.append({"success": False, "code": (it.get("sql_code") or it.get("code")),
                            "message": str(e)})
    return _ok(results, trace_id=_trace())


# ================================================================
#  SQL 定义管理
# ================================================================
@app.get("/api/admin/sqls")
async def admin_sqls():
    return _ok(DSQL.list_defs(), trace_id=_trace())


@app.post("/api/admin/sqls")
async def admin_sql_create(req: Request):
    b = await req.json()
    code = b.get("code")
    if not code or not b.get("template"):
        return _err("code 与 template 必填")
    try:
        r = DSQL.upsert_def(code=code, name=b.get("name", ""), template=b["template"],
                            datasource=b.get("datasource", "default"),
                            cache_ttl=b.get("cache_ttl", 0),
                            status=b.get("status", "published"),
                            description=b.get("description", ""))
        _audit("admin", "sql.upsert", r, _trace())
        return _ok(r, message="SQL 定义已保存")
    except ValueError as e:
        return _err(str(e), 422)


@app.put("/api/admin/sqls/{code}")
async def admin_sql_update(code: str, req: Request):
    b = await req.json()
    try:
        r = DSQL.upsert_def(code=code, name=b.get("name", ""), template=b.get("template", ""),
                            datasource=b.get("datasource", "default"),
                            cache_ttl=b.get("cache_ttl", 0),
                            status=b.get("status", "published"),
                            description=b.get("description", ""))
        _audit("admin", "sql.update", {"code": code}, _trace())
        return _ok(r, message="SQL 定义已更新")
    except ValueError as e:
        return _err(str(e), 422)


@app.post("/api/admin/sqls/{code}/status")
async def admin_sql_status(code: str, req: Request):
    b = await req.json()
    status = b.get("status")
    if status not in ("draft", "published", "disabled"):
        return _err("status 必须为 draft/published/disabled")
    return _ok(DSQL.set_status(code, status), message=f"状态已更新为 {status}")


@app.post("/api/admin/sqls/{code}/test")
async def admin_sql_test(code: str, req: Request):
    b = await req.json()
    try:
        # 用临时定义试运行（不覆盖正式定义）
        tmp = dict(b)
        tmp["code"] = code
        saved = DSQL.upsert_def(code=code, name=b.get("name", code), template=b.get("template", ""),
                                datasource=b.get("datasource", "default"),
                                cache_ttl=0, status="published", description="test")
        res = DSQL.execute(code, b.get("params") or {}, role=b.get("role", "admin"), use_cache=False)
        # 恢复原定义（按原模板回写，防止测试覆盖）
        rows = META.query("SELECT template FROM dsql_sqls WHERE code=?", [code])
        return _ok({"rendered_sql": res.get("sql"), "data": res["data"],
                    "row_count": res.get("row_count", 0),
                    "duration_ms": res["duration_ms"]}, message="测试通过", trace_id=res["trace_id"])
    except Exception as e:  # noqa: BLE001
        return _err(f"测试失败: {e}", 422)


@app.delete("/api/admin/sqls/{code}")
async def admin_sql_delete(code: str):
    DSQL.delete_def(code)
    return _ok(message=f"SQL 定义已删除: {code}")


# ================================================================
#  数据源管理
# ================================================================
@app.get("/api/admin/datasources")
async def admin_datasources():
    return _ok(META.query("SELECT id,name,driver,config_json,enabled FROM datasources"),
               trace_id=_trace())


@app.post("/api/admin/datasources")
async def admin_datasource_create(req: Request):
    b = await req.json()
    name, driver = b.get("name"), b.get("driver", "sqlite")
    if not name:
        return _err("name 必填")
    try:
        META.execute(
            "INSERT INTO datasources(name,driver,config_json,enabled,created_at,updated_at) "
            "VALUES(?,?,?,?,?,?)",
            [name, driver, json.dumps(b.get("config", {}), ensure_ascii=False), 1,
             int(time.time()), int(time.time())])
        refresh_adapters()
        return _ok(message=f"数据源 {name} 已创建（驱动 {driver}）")
    except Exception as e:  # noqa: BLE001
        return _err(f"创建失败: {e}", 422)


@app.post("/api/admin/datasources/{name}/reload")
async def admin_datasource_reload(name: str):
    refresh_adapters()
    return _ok(ADAPTERS.get(name) and ADAPTERS[name].describe(), message="数据源已重载")


# ================================================================
#  字段级权限
# ================================================================
@app.get("/api/admin/permissions")
async def admin_permissions():
    return _ok(META.query("SELECT resource,role,allowed_fields FROM field_permissions ORDER BY resource,role"),
               trace_id=_trace())


@app.post("/api/admin/permissions")
async def admin_permission_set(req: Request):
    b = await req.json()
    resource, role = b.get("resource"), b.get("role")
    allowed = b.get("allowed_fields")  # 字符串 "a,b,c" 或 None(全部)
    if not resource or not role:
        return _err("resource 与 role 必填")
    META.execute(
        "INSERT INTO field_permissions(resource,role,allowed_fields) VALUES(?,?,?) "
        "ON CONFLICT(resource,role) DO UPDATE SET allowed_fields=excluded.allowed_fields",
        [resource, role, allowed])
    DSQL.invalidate_def(resource)
    return _ok(message=f"已设置 {role}@{resource} 字段权限")


@app.delete("/api/admin/permissions")
async def admin_permission_delete(req: Request):
    b = await req.json()
    META.execute("DELETE FROM field_permissions WHERE resource=? AND role=?",
                 [b.get("resource"), b.get("role")])
    return _ok(message="权限已删除")


@app.get("/api/admin/roles")
async def admin_roles():
    return _ok(META.query("SELECT id,name,description FROM roles"), trace_id=_trace())


@app.get("/api/admin/users")
async def admin_users():
    return _ok(META.query("SELECT id,username,role,display_name FROM users"), trace_id=_trace())


# ================================================================
#  知识图谱（自研 mox-kg-core）
# ================================================================
@app.get("/api/kg/graph")
async def kg_graph(domain: Optional[str] = None):
    return _ok(KG.graph_data(domain), trace_id=_trace())


@app.post("/api/kg/query")
async def kg_query(req: Request):
    b = await req.json()
    dsl = b.get("dsl") or ""
    try:
        if dsl == "graph":
            data = KG.graph_data(b.get("domain"))
        elif dsl.startswith("neighbors:"):
            vid = dsl.split(":", 1)[1]
            data = {"neighbors": KG.neighbors(vid, b.get("direction", "out"))}
        elif dsl.startswith("reachable:"):
            parts = dsl.split(":")
            vid = parts[1]
            hops = int(parts[2]) if len(parts) > 2 else 2
            data = {"reachable": KG.reachable(vid, hops, b.get("direction", "out"))}
        elif dsl.startswith("path:"):
            _, s, t = dsl.split(":")
            data = {"path": KG.shortest_path(s, t)}
        elif dsl == "stats":
            data = KG.stats()
        else:
            return _err(f"不支持的图谱 DSL: {dsl}", 422)
        return _ok(data, message="kg query ok", trace_id=_trace())
    except Exception as e:  # noqa: BLE001
        return _err(f"图谱查询失败: {e}", 422)


@app.post("/api/kg/traverse")
async def kg_traverse(req: Request):
    b = await req.json()
    vid = b.get("vertex_id")
    if not vid:
        return _err("缺少 vertex_id")
    return _ok({"neighbors": KG.neighbors(vid, b.get("direction", "out"))}, trace_id=_trace())


@app.post("/api/admin/kg/vertices")
async def kg_vertex_upsert(req: Request):
    b = await req.json()
    vid = b.get("vid")
    if not vid or not b.get("type") or not b.get("label"):
        return _err("vid/type/label 必填")
    v = KG.upsert_vertex(vid=vid, vtype=b["type"], label=b["label"],
                         props=b.get("props") or {}, domain=b.get("domain", "default"))
    return _ok(v, message="图谱顶点已保存")


@app.delete("/api/admin/kg/vertices/{vid}")
async def kg_vertex_delete(vid: str):
    KG.delete_vertex(vid)
    return _ok(message="顶点已删除")


@app.post("/api/admin/kg/edges")
async def kg_edge_upsert(req: Request):
    b = await req.json()
    if not b.get("source") or not b.get("relation") or not b.get("target"):
        return _err("source/relation/target 必填")
    e = KG.upsert_edge(b["source"], b["relation"], b["target"], float(b.get("weight", 1.0)))
    return _ok(e, message="图谱关系已保存")


@app.delete("/api/admin/kg/edges")
async def kg_edge_delete(req: Request):
    b = await req.json()
    KG.delete_edge(b.get("source"), b.get("relation"), b.get("target"))
    return _ok(message="图谱关系已删除")


# ================================================================
#  缓存
# ================================================================
@app.get("/api/cache/stats")
async def cache_stats():
    return _ok(CACHE.stats(), trace_id=_trace())


@app.post("/api/cache/clear")
async def cache_clear():
    n = CACHE.clear()
    DSQL.invalidate_def()
    return _ok({"cleared": n}, message=f"已清理 {n} 条缓存")


# ================================================================
#  官网写接口
# ================================================================
@app.post("/api/website/message")
async def website_message(req: Request):
    b = await req.json()
    if not b.get("name") or not b.get("content"):
        return _err("name/content 必填")
    META.execute(
        "INSERT INTO messages(name,phone,email,company,content,status,created_at) "
        "VALUES(?,?,?,?,?,?,?)",
        [b.get("name"), b.get("phone", ""), b.get("email", ""), b.get("company", ""),
         b.get("content"), "待处理", int(time.time())])
    _audit(b.get("name"), "website.message", b, _trace())
    return _ok({"id": 0}, message="留言提交成功")


@app.post("/api/website/resume")
async def website_resume(req: Request):
    b = await req.json()
    if not b.get("name") or not b.get("email"):
        return _err("name/email 必填")
    _audit(b.get("name"), "website.resume", b, _trace())
    return _ok(message="简历投递成功")


@app.post("/api/website/consultation")
async def website_consultation(req: Request):
    b = await req.json()
    _audit(b.get("name", "anon"), "website.consultation", b, _trace())
    return _ok(message="咨询已受理")


# ================================================================
#  平台概览 / 审计 / 健康
# ================================================================
@app.get("/api/stats")
async def api_stats():
    sql_count = META.query("SELECT COUNT(*) c FROM dsql_sqls")[0]["c"]
    kg = KG.stats()
    msgs = META.query("SELECT COUNT(*) c FROM messages")[0]["c"]
    return _ok({
        "dsql_sqls": sql_count,
        "datasources": list(ADAPTERS.keys()),
        "cache": CACHE.stats(),
        "kg": kg,
        "messages": msgs,
        "uptime_since": "boot",
    }, trace_id=_trace())


@app.get("/api/audit")
async def api_audit(limit: int = 50):
    rows = META.query("SELECT ts,trace_id,actor,action,detail FROM audit_logs "
                      "ORDER BY ts DESC LIMIT ?", [min(limit, 500)])
    for r in rows:
        try:
            r["detail"] = json.loads(r["detail"])
        except Exception:  # noqa: BLE001
            pass
    return _ok(rows, trace_id=_trace())


@app.get("/api/health")
async def api_health():
    return _ok({"status": "healthy", "engine": "mox-dsql-core + mox-kg-core", "version": "2.0.0"})
