# -*- coding: utf-8 -*-
"""mox-server v2.1 新功能测试：无限发布系统（应用管理 / AI 助手 / 业务流程）。"""
import json
import sys
import urllib.request

BASE = "http://127.0.0.1:8600"
PASS = 0
FAIL = 0


def call(path, body=None, method=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method or ("POST" if data else "GET"))
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        try:
            return json.loads(e.read().decode("utf-8"))
        except Exception:  # noqa: BLE001
            return {"success": False, "status": e.code}


def check(name, cond, extra=""):
    global PASS, FAIL
    if cond:
        PASS += 1
        print(f"  PASS {name}")
    else:
        FAIL += 1
        print(f"  FAIL {name} {extra}")


print("[应用管理]")
call("/api/apps/test_corp", method="DELETE")  # 清理可能残留
apps = call("/api/apps")
check("apps list has mox", any(a["app_key"] == "mox" for a in apps["data"]), str(apps)[:200])
mox = [a for a in apps["data"] if a["app_key"] == "mox"][0]
check("mox app running + version>=1", mox["status"] == "running" and mox["publish_version"] >= 1)
check("mox app has sql_count", mox.get("sql_count", 0) >= 16, str(mox.get("sql_count")))

# 创建新应用
r = call("/api/apps", {"app_key": "test_corp", "name": "测试企业官网", "type": "website",
                       "domain": "test.mox.tech", "config": {"template": "standard"}})
check("create app", r.get("success") and r["data"]["status"] == "draft", str(r)[:300])
# 状态机：draft->prepared->published->running
for t in ("prepared", "published", "running"):
    r = call("/api/apps/test_corp/transition", {"target": t, "operator": "admin"})
    check(f"transition ->{t}", r.get("success") and r["data"]["status"] == t, str(r)[:200])
r = call("/api/apps/test_corp/transition", {"target": "draft"})
check("illegal transition rejected (running->draft)", not r.get("success"), str(r)[:200])
# 发布日志
logs = call("/api/apps/test_corp/logs")
check("publish logs recorded", len(logs["data"]) >= 3, str(len(logs["data"])))
# 删除
r = call("/api/apps/test_corp", method="DELETE")
check("delete app", r.get("success"), str(r)[:200])
r = call("/api/apps/mox", method="DELETE")
check("default app mox protected", not r.get("success"), str(r)[:200])

print("[业务流程]")
flow = call("/api/process/flow")
stages = flow["data"]["stages"]
check("flow 9 stages", len(stages) == 9 and stages[0] == "需求" and stages[-1] == "下线归档", str(stages))
check("flow nodes have input/process/output", all(
    all(k in n for k in ("name", "input", "process", "output", "component", "check"))
    for n in flow["data"]["flow"]))

print("[AI 助手]")
r = call("/api/ai/assistant", {"message": "查询所有产品列表", "app_key": "mox"})
check("ai product intent + sql", r.get("success") and r["data"].get("sql"), str(r)[:300])
check("ai sql contains products", "products" in r["data"].get("sql", ""))
r2 = call("/api/ai/assistant", {"message": "最新新闻前3条", "app_key": "mox"})
check("ai news intent", r2.get("success") and "news" in r2["data"].get("sql", ""))
r3 = call("/api/ai/assistant", {"message": "统计产品数量", "app_key": "mox"})
check("ai count intent", r3.get("success") and "COUNT" in r3["data"].get("sql", "").upper(), str(r3)[:300])
r4 = call("/api/ai/assistant", {"message": "解释 SELECT id,name FROM products WHERE category='软件平台'"})
check("ai explain sql", r4.get("success") and "products" in r4["data"].get("sql_explain", ""))
r5 = call("/api/ai/assistant", {"message": "优化 SELECT * FROM products"})
check("ai suggest", r5.get("success") and len(r5["data"].get("suggestions", [])) >= 1)
reqs = call("/api/ai/requests")
check("ai requests logged", len(reqs["data"]) >= 5)

print("[app 维度 SQL + 官网全量 SQL]")
r = call("/api/admin/sqls", method="GET")
all_sqls = r["data"]
check("sql list 16+", len(all_sqls) >= 16, str(len(all_sqls)))
for code, params in (
        ("home_banner_list", {}), ("home_latest_news", {}), ("home_recommend_cases", {}),
        ("home_recommend_products", {}), ("product_list", {}),
        ("product_detail", {"id": 1}), ("product_related", {"id": 1}),
        ("news_list", {}), ("news_detail", {"id": 1}),
        ("case_list", {}), ("case_detail", {"id": 1}),
        ("team_list", {}), ("search_all", {"keyword": "%数据%"}),
        ("stats_dashboard", {}), ("message_list", {})):
    rr = call("/api/dsql/execute", {"sql_code": code, "params": params})
    check(f"website sql ok: {code}", rr.get("success") and isinstance(rr.get("data"), list), str(rr)[:200])

print(f"\nRESULT: PASS={PASS} FAIL={FAIL}")
sys.exit(1 if FAIL else 0)
