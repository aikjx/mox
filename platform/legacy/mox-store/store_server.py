# -*- coding: utf-8 -*-
"""
MOX 应用商店服务 (mox-store)
独立运行，端口8601，与mox-server共享mox_meta.db
提供: 应用浏览/发布/安装/卸载/更新/评分/子系统管理
"""
import os, json, sqlite3, hashlib, zipfile, shutil, time, uuid
from datetime import datetime, timezone, timedelta
from typing import Optional
from fastapi import FastAPI, Request, UploadFile, File, Form
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse, FileResponse

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
META_DB = os.path.join(ROOT, "mox-server", "mox_meta.db")
STORE_DIR = os.path.join(ROOT, "mox-store", "store_data")
APPS_DIR = os.path.join(STORE_DIR, "apps")
UPLOAD_DIR = os.path.join(STORE_DIR, "uploads")
os.makedirs(APPS_DIR, exist_ok=True)
os.makedirs(UPLOAD_DIR, exist_ok=True)

app = FastAPI(title="mox-store 应用商店服务", version="1.0.0")
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_credentials=False,
                   allow_methods=["*"], allow_headers=["*"])


def now_iso():
    return datetime.now(CST).isoformat()


def get_db():
    conn = sqlite3.connect(META_DB)
    conn.row_factory = sqlite3.Row
    return conn


def init_db():
    conn = get_db()
    cur = conn.cursor()
    cur.executescript("""
    CREATE TABLE IF NOT EXISTS store_apps (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT UNIQUE NOT NULL,
        app_name TEXT NOT NULL,
        app_type TEXT DEFAULT 'subsystem',
        version TEXT DEFAULT '1.0.0',
        author TEXT DEFAULT '',
        author_key TEXT DEFAULT '',
        description TEXT DEFAULT '',
        long_description TEXT DEFAULT '',
        category TEXT DEFAULT '其他',
        tags TEXT DEFAULT '[]',
        icon_url TEXT DEFAULT '',
        screenshots TEXT DEFAULT '[]',
        price TEXT DEFAULT 'free',
        download_count INTEGER DEFAULT 0,
        install_count INTEGER DEFAULT 0,
        rating_avg REAL DEFAULT 0,
        rating_count INTEGER DEFAULT 0,
        status TEXT DEFAULT 'approved',
        mxap_path TEXT DEFAULT '',
        manifest TEXT DEFAULT '{}',
        signature TEXT DEFAULT '',
        created_at TEXT,
        updated_at TEXT
    );
    CREATE TABLE IF NOT EXISTS store_installs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT NOT NULL,
        app_name TEXT NOT NULL,
        version TEXT DEFAULT '1.0.0',
        install_path TEXT DEFAULT '',
        status TEXT DEFAULT 'running',
        config TEXT DEFAULT '{}',
        installed_at TEXT,
        updated_at TEXT,
        UNIQUE(app_key)
    );
    CREATE TABLE IF NOT EXISTS store_ratings (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT NOT NULL,
        user_key TEXT DEFAULT 'anonymous',
        rating INTEGER DEFAULT 5,
        comment TEXT DEFAULT '',
        created_at TEXT
    );
    """)
    conn.commit()
    conn.close()


init_db()


def trace_id():
    return "str-" + uuid.uuid4().hex[:12]


def ok(data=None, message="ok", code=0):
    return {"success": True, "code": code, "message": message, "data": data,
            "trace_id": trace_id(), "timestamp": now_iso()}


def fail(message, code=1):
    return {"success": False, "code": code, "message": message,
            "trace_id": trace_id(), "timestamp": now_iso()}


# ==================== 商店浏览 ====================

@app.get("/api/store/apps")
def list_apps(category: str = "", keyword: str = "", sort: str = "hot",
              page: int = 1, size: int = 20, app_type: str = ""):
    conn = get_db()
    cur = conn.cursor()
    where = ["status='approved'"]
    params = []
    if category and category != "全部":
        where.append("category=?"); params.append(category)
    if app_type:
        where.append("app_type=?"); params.append(app_type)
    if keyword:
        where.append("(app_name LIKE ? OR description LIKE ? OR tags LIKE ?)")
        kw = f"%{keyword}%"; params.extend([kw, kw, kw])
    sql = f"SELECT * FROM store_apps WHERE {' AND '.join(where)}"
    order = {"hot": "install_count DESC, rating_avg DESC",
             "new": "created_at DESC",
             "rating": "rating_avg DESC, rating_count DESC",
             "name": "app_name ASC"}.get(sort, "install_count DESC")
    sql += f" ORDER BY {order} LIMIT ? OFFSET ?"
    params.extend([size, (page - 1) * size])
    rows = [dict(r) for r in cur.execute(sql, params).fetchall()]
    total = cur.execute(f"SELECT COUNT(*) FROM store_apps WHERE {' AND '.join(where)}", params[:-2]).fetchone()[0]
    conn.close()
    for r in rows:
        r["tags"] = json.loads(r.get("tags", "[]"))
        r["screenshots"] = json.loads(r.get("screenshots", "[]"))
    return ok({"items": rows, "total": total, "page": page, "size": size})


@app.get("/api/store/apps/{app_key}")
def app_detail(app_key: str):
    conn = get_db()
    row = conn.execute("SELECT * FROM store_apps WHERE app_key=?", (app_key,)).fetchone()
    conn.close()
    if not row:
        return JSONResponse(status_code=404, content=fail("应用不存在"))
    r = dict(row)
    r["tags"] = json.loads(r.get("tags", "[]"))
    r["screenshots"] = json.loads(r.get("screenshots", "[]"))
    r["manifest"] = json.loads(r.get("manifest", "{}"))
    return ok(r)


@app.get("/api/store/categories")
def categories():
    cats = ["办公协同", "数据分析", "电商零售", "教育培训", "人力资源",
            "财务管理", "内容管理", "客服支持", "开发工具", "行业方案", "其他"]
    return ok([{"name": c, "icon": "package"} for c in cats])


@app.get("/api/store/featured")
def featured():
    conn = get_db()
    rows = [dict(r) for r in conn.execute(
        "SELECT * FROM store_apps WHERE status='approved' ORDER BY rating_avg DESC, install_count DESC LIMIT 6").fetchall()]
    conn.close()
    for r in rows:
        r["tags"] = json.loads(r.get("tags", "[]"))
    return ok(rows)


@app.get("/api/store/hot")
def hot(limit: int = 10):
    conn = get_db()
    rows = [dict(r) for r in conn.execute(
        "SELECT app_key, app_name, icon_url, category, install_count, rating_avg FROM store_apps WHERE status='approved' ORDER BY install_count DESC LIMIT ?", (limit,)).fetchall()]
    conn.close()
    return ok(rows)


# ==================== 发布 ====================

@app.post("/api/store/publish")
async def publish_app(
    manifest: str = Form(...),
    mxap: UploadFile = File(...),
    icon: Optional[UploadFile] = File(None)
):
    try:
        mf = json.loads(manifest)
    except Exception:
        return JSONResponse(status_code=400, content=fail("manifest格式错误"))
    app_key = mf.get("app_key", "")
    if not app_key:
        return JSONResponse(status_code=400, content=fail("缺少app_key"))

    # 保存MXAP包
    mxap_path = os.path.join(APPS_DIR, f"{app_key}-{mf.get('version','1.0.0')}.mxap")
    content = await mxap.read()
    with open(mxap_path, "wb") as f:
        f.write(content)

    # 保存图标
    icon_url = ""
    if icon:
        icon_path = os.path.join(APPS_DIR, f"{app_key}.png")
        with open(icon_path, "wb") as f:
            f.write(await icon.read())
        icon_url = f"/store/static/{app_key}.png"

    ts = now_iso()
    conn = get_db()
    cur = conn.cursor()
    existing = cur.execute("SELECT id FROM store_apps WHERE app_key=?", (app_key,)).fetchone()
    if existing:
        cur.execute("""UPDATE store_apps SET app_name=?, app_type=?, version=?, author=?,
            description=?, long_description=?, category=?, tags=?, icon_url=?, price=?,
            mxap_path=?, manifest=?, updated_at=? WHERE app_key=?""",
            (mf.get("app_name", app_key), mf.get("app_type", "subsystem"),
             mf.get("version", "1.0.0"), mf.get("author", ""),
             mf.get("description", ""), mf.get("long_description", ""),
             mf.get("category", "其他"), json.dumps(mf.get("tags", [])),
             icon_url, mf.get("price", "free"), mxap_path, json.dumps(mf), ts, app_key))
    else:
        cur.execute("""INSERT INTO store_apps
            (app_key, app_name, app_type, version, author, author_key, description,
             long_description, category, tags, icon_url, price, status, mxap_path,
             manifest, created_at, updated_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (app_key, mf.get("app_name", app_key), mf.get("app_type", "subsystem"),
             mf.get("version", "1.0.0"), mf.get("author", ""), mf.get("author_key", ""),
             mf.get("description", ""), mf.get("long_description", ""),
             mf.get("category", "其他"), json.dumps(mf.get("tags", [])),
             icon_url, mf.get("price", "free"), "approved", mxap_path,
             json.dumps(mf), ts, ts))
    conn.commit()
    conn.close()
    return ok({"app_key": app_key, "version": mf.get("version", "1.0.0"),
               "status": "published", "mxap_size": len(content)})


# ==================== 安装管理 ====================

@app.get("/api/store/installed")
def installed_apps():
    conn = get_db()
    rows = [dict(r) for r in conn.execute("SELECT * FROM store_installs ORDER BY installed_at DESC").fetchall()]
    conn.close()
    for r in rows:
        r["config"] = json.loads(r.get("config", "{}"))
    return ok(rows)


@app.post("/api/store/install/{app_key}")
def install_app(app_key: str):
    conn = get_db()
    app = conn.execute("SELECT * FROM store_apps WHERE app_key=?", (app_key,)).fetchone()
    if not app:
        conn.close()
        return JSONResponse(status_code=404, content=fail("应用不存在"))
    app = dict(app)

    # 检查是否已安装
    existing = conn.execute("SELECT id FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    if existing:
        conn.close()
        return fail("应用已安装，请使用更新")

    # 解压MXAP包到安装目录
    install_path = os.path.join(STORE_DIR, "installed", app_key)
    os.makedirs(install_path, exist_ok=True)
    mxap_path = app.get("mxap_path", "")
    if mxap_path and os.path.exists(mxap_path):
        try:
            with zipfile.ZipFile(mxap_path, 'r') as zf:
                zf.extractall(install_path)
        except Exception as e:
            conn.close()
            return fail(f"解压失败: {e}")

    ts = now_iso()
    conn.execute("""INSERT INTO store_installs (app_key, app_name, version, install_path, status, config, installed_at, updated_at)
        VALUES (?,?,?,?,?,?,?,?)""",
        (app_key, app["app_name"], app["version"], install_path, "running", "{}", ts, ts))
    conn.execute("UPDATE store_apps SET install_count=install_count+1 WHERE app_key=?", (app_key,))
    conn.commit()
    conn.close()
    return ok({"app_key": app_key, "app_name": app["app_name"], "version": app["version"],
               "install_path": install_path, "status": "installed"})


@app.delete("/api/store/install/{app_key}")
def uninstall_app(app_key: str):
    conn = get_db()
    inst = conn.execute("SELECT * FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    if not inst:
        conn.close()
        return JSONResponse(status_code=404, content=fail("应用未安装"))
    inst = dict(inst)
    # 删除安装目录
    if inst.get("install_path") and os.path.exists(inst["install_path"]):
        shutil.rmtree(inst["install_path"], ignore_errors=True)
    conn.execute("DELETE FROM store_installs WHERE app_key=?", (app_key,))
    conn.commit()
    conn.close()
    return ok({"app_key": app_key, "status": "uninstalled"})


@app.post("/api/store/install/{app_key}/update")
def update_app(app_key: str):
    conn = get_db()
    app = conn.execute("SELECT * FROM store_apps WHERE app_key=?", (app_key,)).fetchone()
    inst = conn.execute("SELECT * FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    if not app or not inst:
        conn.close()
        return JSONResponse(status_code=404, content=fail("应用不存在或未安装"))
    app, inst = dict(app), dict(inst)
    # 重新解压
    if app.get("mxap_path") and os.path.exists(app["mxap_path"]):
        shutil.rmtree(inst["install_path"], ignore_errors=True)
        os.makedirs(inst["install_path"], exist_ok=True)
        with zipfile.ZipFile(app["mxap_path"], 'r') as zf:
            zf.extractall(inst["install_path"])
    ts = now_iso()
    conn.execute("UPDATE store_installs SET version=?, updated_at=? WHERE app_key=?",
                 (app["version"], ts, app_key))
    conn.commit()
    conn.close()
    return ok({"app_key": app_key, "version": app["version"], "status": "updated"})


# ==================== 评分 ====================

@app.get("/api/store/apps/{app_key}/ratings")
def list_ratings(app_key: str, page: int = 1, size: int = 20):
    conn = get_db()
    rows = [dict(r) for r in conn.execute(
        "SELECT * FROM store_ratings WHERE app_key=? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        (app_key, size, (page - 1) * size)).fetchall()]
    stats = conn.execute(
        "SELECT COUNT(*) as cnt, COALESCE(AVG(rating),0) as avg FROM store_ratings WHERE app_key=?",
        (app_key,)).fetchone()
    conn.close()
    return ok({"items": rows, "total": stats["cnt"], "avg_rating": round(stats["avg"], 2)})


@app.post("/api/store/apps/{app_key}/ratings")
async def rate_app(app_key: str, request: Request):
    body = await request.json()
    rating = min(5, max(1, int(body.get("rating", 5))))
    comment = body.get("comment", "")
    user_key = body.get("user_key", "anonymous")
    ts = now_iso()
    conn = get_db()
    conn.execute("INSERT INTO store_ratings (app_key, user_key, rating, comment, created_at) VALUES (?,?,?,?,?)",
                 (app_key, user_key, rating, comment, ts))
    stats = conn.execute("SELECT COUNT(*) as cnt, AVG(rating) as avg FROM store_ratings WHERE app_key=?", (app_key,)).fetchone()
    conn.execute("UPDATE store_apps SET rating_avg=?, rating_count=? WHERE app_key=?",
                 (round(stats["avg"], 2), stats["cnt"], app_key))
    conn.commit()
    conn.close()
    return ok({"app_key": app_key, "rating": rating, "avg_rating": round(stats["avg"], 2)})


# ==================== 子系统运行时 ====================

@app.get("/api/store/runtime/{app_key}")
def runtime_info(app_key: str):
    conn = get_db()
    inst = conn.execute("SELECT * FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    conn.close()
    if not inst:
        return JSONResponse(status_code=404, content=fail("应用未安装"))
    inst = dict(inst)
    inst["config"] = json.loads(inst.get("config", "{}"))
    # 检查前端入口
    entry = os.path.join(inst["install_path"], "frontend", "index.html")
    inst["entry_exists"] = os.path.exists(entry)
    inst["entry_url"] = f"/store/apps/{app_key}/index.html"
    return ok(inst)


@app.get("/store/apps/{app_key}/{path:path}")
def serve_app_file(app_key: str, path: str):
    conn = get_db()
    inst = conn.execute("SELECT install_path FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    conn.close()
    if not inst:
        return JSONResponse(status_code=404, content=fail("应用未安装"))
    file_path = os.path.join(inst["install_path"], "frontend", path)
    if not os.path.exists(file_path) or not os.path.isfile(file_path):
        return JSONResponse(status_code=404, content=fail("文件不存在"))
    return FileResponse(file_path)


@app.get("/store/static/{filename}")
def serve_static(filename: str):
    file_path = os.path.join(APPS_DIR, filename)
    if not os.path.exists(file_path):
        return JSONResponse(status_code=404, content=fail("文件不存在"))
    return FileResponse(file_path)


# ==================== 健康检查 ====================

@app.get("/api/store/health")
def health():
    return ok({"status": "ok", "service": "mox-store", "version": "1.0.0", "time": now_iso()})


@app.get("/api/store/stats")
def stats():
    conn = get_db()
    total_apps = conn.execute("SELECT COUNT(*) FROM store_apps WHERE status='approved'").fetchone()[0]
    total_installs = conn.execute("SELECT COUNT(*) FROM store_installs").fetchone()[0]
    total_ratings = conn.execute("SELECT COUNT(*) FROM store_ratings").fetchone()[0]
    categories = [dict(r) for r in conn.execute(
        "SELECT category, COUNT(*) as cnt FROM store_apps WHERE status='approved' GROUP BY category ORDER BY cnt DESC").fetchall()]
    conn.close()
    return ok({"total_apps": total_apps, "total_installs": total_installs,
               "total_ratings": total_ratings, "categories": categories})


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8601)
