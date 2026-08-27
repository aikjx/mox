# -*- coding: utf-8 -*-
"""mox-server API 层全接口测试"""
import json
import urllib.request

BASE = "http://127.0.0.1:8600"
fails = 0


def call(method, path, body=None):
    url = BASE + path
    data = json.dumps(body, ensure_ascii=False).encode("utf-8") if body is not None else None
    req = urllib.request.Request(url, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        return json.loads(resp.read().decode("utf-8"))


def check(name, cond, extra=""):
    global fails
    print(("PASS" if cond else "FAIL") + " | " + name + (" " + extra if extra else ""))
    if not cond:
        fails += 1


# 1. DSQL 执行
r = call("POST", "/api/dsql/execute", {"sql_code": "product_list", "params": {"category": "软件平台"}})
check("POST /api/dsql/execute product_list(软件平台)", r["success"] and len(r["data"]) == 1, "rows=%d" % len(r["data"]))

# 2. guest 字段级权限
r = call("POST", "/api/dsql/execute", {"sql_code": "product_list", "params": {}, "role": "guest"})
check("guest field permission hides price", all("price" not in x for x in r["data"]))
r = call("POST", "/api/dsql/execute", {"sql_code": "product_list", "params": {}, "role": "admin"})
check("admin sees price", all("price" in x for x in r["data"]))

# 3. explain
r = call("POST", "/api/dsql/explain", {"sql_code": "product_detail", "params": {"id": 1}})
check("POST /api/dsql/explain renders sql", r["success"] and "WHERE id=?" in r["data"]["rendered_sql"])

# 4. batch
r = call("POST", "/api/dsql/execute-batch", {"items": [
    {"sql_code": "home_banner_list", "params": {}},
    {"sql_code": "team_list", "params": {}},
]})
check("POST /api/dsql/execute-batch 2 items", r["success"] and len(r["data"]) == 2)

# 5. SQL 定义管理
r = call("GET", "/api/admin/sqls")
check("GET /api/admin/sqls count>=14", r["success"] and len(r["data"]) >= 14, "n=%d" % len(r["data"]))
r = call("POST", "/api/admin/sqls", {"code": "demo_ping", "name": "测试SQL", "template": "SELECT {{v}} AS val", "status": "published", "cache_ttl": 5})
check("POST /api/admin/sqls create demo_ping", r["success"])
r = call("POST", "/api/dsql/execute", {"sql_code": "demo_ping", "params": {"v": "hello"}})
check("execute demo_ping", r["success"] and r["data"][0]["val"] == "hello")
r = call("DELETE", "/api/admin/sqls/demo_ping")
check("DELETE /api/admin/sqls/demo_ping", r["success"])

# 6. 数据源
r = call("GET", "/api/admin/datasources")
check("GET /api/admin/datasources has default", any(d["name"] == "default" for d in r["data"]))

# 7. 字段权限配置
r = call("POST", "/api/admin/permissions", {"resource": "product_list", "role": "staff", "allowed_fields": "id,name,category"})
check("POST /api/admin/permissions set", r["success"])
r = call("POST", "/api/dsql/execute", {"sql_code": "product_list", "params": {}, "role": "staff"})
check("staff sees only id,name,category", all(set(x.keys()) == {"id", "name", "category"} for x in r["data"]))
r = call("DELETE", "/api/admin/permissions", {"resource": "product_list", "role": "staff"})
check("DELETE /api/admin/permissions", r["success"])

# 8. 图谱
r = call("GET", "/api/kg/graph")
check("GET /api/kg/graph", r["success"] and r["data"]["vertex_count"] == 16)
r = call("POST", "/api/kg/query", {"dsl": "neighbors:product:3", "direction": "both"})
check("POST /api/kg/query neighbors", r["success"] and len(r["data"]["neighbors"]) >= 3)
r = call("POST", "/api/kg/query", {"dsl": "reachable:product:3:2"})
check("POST /api/kg/query reachable", r["success"] and len(r["data"]["reachable"]) >= 5)
r = call("POST", "/api/kg/query", {"dsl": "path:case:1:category:3"})
check("POST /api/kg/query shortest path", r["success"] and r["data"]["path"])
r = call("POST", "/api/kg/traverse", {"vertex_id": "case:2"})
check("POST /api/kg/traverse", r["success"] and len(r["data"]["neighbors"]) >= 1)

# 9. 缓存
r = call("GET", "/api/cache/stats")
check("GET /api/cache/stats", r["success"])
r = call("POST", "/api/cache/clear")
check("POST /api/cache/clear", r["success"])

# 10. 官网写接口
r = call("POST", "/api/website/message", {"name": "测试用户", "phone": "13800000000", "content": "API测试留言"})
check("POST /api/website/message", r["success"])
r = call("POST", "/api/website/resume", {"name": "张三", "email": "z@mox.tech"})
check("POST /api/website/resume", r["success"])
r = call("POST", "/api/website/consultation", {"name": "李四", "product": "MX-Data"})
check("POST /api/website/consultation", r["success"])

# 11. 平台概览 / 审计
r = call("GET", "/api/stats")
check("GET /api/stats", r["success"] and r["data"]["dsql_sqls"] >= 14)
r = call("GET", "/api/audit?limit=10")
check("GET /api/audit has audit trail", r["success"] and len(r["data"]) >= 1)

print()
print("RESULT:", "ALL API PASSED" if fails == 0 else ("%d FAILED" % fails))
