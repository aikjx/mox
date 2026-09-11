# -*- coding: utf-8 -*-
"""mox-server 引擎冒烟测试"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mox.server import DSQL, KG

fails = 0

def check(name, cond):
    global fails
    print(("PASS" if cond else "FAIL") + " | " + name)
    if not cond:
        fails += 1

# 1. 分类筛选
r = DSQL.execute("product_list", {"category": "数据引擎"}, role="admin", use_cache=False)
check("product_list filtered -> %d rows" % len(r["data"]), len(r["data"]) == 1 and r["data"][0]["category"] == "数据引擎")

# 2. 字段级权限（guest 隐藏 price）
r2 = DSQL.execute("product_list", {}, role="guest", use_cache=False)
check("guest no price field", all("price" not in x for x in r2["data"]))
r3 = DSQL.execute("product_list", {}, role="admin", use_cache=False)
check("admin has price field", all("price" in x for x in r3["data"]))

# 3. 缓存命中
DSQL._cache.clear()
DSQL.execute("home_recommend_products", {}, role="admin", use_cache=True)
r4b = DSQL.execute("home_recommend_products", {}, role="admin", use_cache=True)
check("cache hit on 2nd call", r4b["cache_hit"] is True)

# 4. 全量
r5 = DSQL.execute("product_list", {}, role="admin", use_cache=False)
check("product_list all -> %d rows" % len(r5["data"]), len(r5["data"]) == 5)

# 5. 搜索（keyword 重复绑定）
r6 = DSQL.execute("search_all", {"keyword": "%数据%"}, role="admin", use_cache=False)
check("search_all rows=%d" % len(r6["data"]), len(r6["data"]) >= 1)

# 6. 详情 + 参数
r7 = DSQL.execute("product_detail", {"id": 2}, role="admin", use_cache=False)
check("product_detail id=2", r7["data"][0]["name"] == "墨行企业平台 MX-Cloud")

# 7. SQL 注入：绑定参数当作普通字符串（无注入效果），写语句硬拦截
inj = DSQL.execute("product_list", {"category": 'x" OR 1=1 --'}, role="admin", use_cache=False)
check("injection payload harmless -> %d rows" % len(inj["data"]), len(inj["data"]) == 0)
from mox.db_adapters import sanitize_sql
try:
    sanitize_sql("DELETE FROM products WHERE 1=1")
    check("write SQL hard-blocked", False)
except ValueError:
    check("write SQL hard-blocked", True)
try:
    sanitize_sql("SELECT * FROM products; DROP TABLE products")
    check("multi-statement hard-blocked", False)
except ValueError:
    check("multi-statement hard-blocked", True)

# 8. 知识图谱
g = KG.graph_data()
check("kg graph v=%d e=%d" % (g["vertex_count"], g["edge_count"]), g["vertex_count"] >= 15 and g["edge_count"] >= 10)
nb = KG.neighbors("product:3", "both")
check("kg neighbors product:3 -> %d" % len(nb), len(nb) >= 3)
reach = KG.reachable("product:3", 2, "both")
check("kg reachable 2-hop -> %d" % len(reach), len(reach) >= 4)
sp = KG.shortest_path("case:1", "category:3")
check("kg shortest path", sp is not None and sp[-1] == "category:3")

print()
print("KG stats:", KG.stats())
print("RESULT:", "ALL PASSED" if fails == 0 else ("%d FAILED" % fails))
sys.exit(1 if fails else 0)
