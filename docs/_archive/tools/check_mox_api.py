#!/usr/bin/env python3
"""检查MOX后端API连通性和数据结构"""
import json, urllib.request, sqlite3, os

BASE = "http://localhost:8600"

def api(method, path, data=None):
    url = BASE + path
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode())
    except Exception as e:
        return {"success": False, "error": str(e)}

print("=" * 60)
print("  MOX 后端 API 检查")
print("=" * 60)

# 1. 健康检查
print("\n【1】健康检查")
r = api("GET", "/api/health")
print(f"  {json.dumps(r, ensure_ascii=False)[:200]}")

# 2. SQL模板列表
print("\n【2】SQL模板列表 (GET /api/admin/sqls)")
r = api("GET", "/api/admin/sqls")
if r.get("success"):
    sqls = r.get("data", [])
    print(f"  总数: {len(sqls)}")
    for s in sqls[:10]:
        print(f"    - {s.get('code')}: {s.get('name','')}")
else:
    print(f"  FAIL: {r}")

# 3. 测试DSQL执行
print("\n【3】DSQL执行测试 (POST /api/dsql/execute)")
# 先找一个存在的sql_code
test_code = None
r = api("GET", "/api/admin/sqls")
if r.get("success") and r.get("data"):
    test_code = r["data"][0].get("code")
    print(f"  测试模板: {test_code}")
    r2 = api("POST", "/api/dsql/execute", {"sql_code": test_code, "params": {}})
    print(f"  结果: success={r2.get('success')}, data_count={len(r2.get('data',[])) if isinstance(r2.get('data'),list) else 'N/A'}")
    if r2.get("data") and isinstance(r2["data"], list) and r2["data"]:
        print(f"  首行: {json.dumps(r2['data'][0], ensure_ascii=False)[:150]}")

# 4. 知识图谱
print("\n【4】知识图谱 (GET /api/kg/graph)")
r = api("GET", "/api/kg/graph")
if r.get("success"):
    d = r.get("data", {})
    print(f"  vertex_count={d.get('vertex_count')}, edge_count={d.get('edge_count')}")
else:
    print(f"  FAIL: {r}")

# 5. 数据库表结构
print("\n【5】数据库表结构")
for db_name in ["mox_meta.db", "mox_business.db"]:
    db_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "platform", "mox-server", db_name)
    if os.path.exists(db_path):
        c = sqlite3.connect(db_path)
        tables = [r[0] for r in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").fetchall()]
        print(f"\n  {db_name}:")
        for t in tables:
            cnt = c.execute(f"SELECT COUNT(*) FROM [{t}]").fetchone()[0]
            cols = [r[1] for r in c.execute(f"PRAGMA table_info([{t}])").fetchall()]
            print(f"    {t} ({cnt}行): {', '.join(cols[:8])}{'...' if len(cols)>8 else ''}")
        c.close()

print("\n" + "=" * 60)
