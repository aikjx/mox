#!/usr/bin/env python3
"""芯擎科技官网 — 全链路API验证"""
import json, urllib.request, time

API = "http://localhost:8600/api"

def call(method, path, data=None):
    url = API + path
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header("Content-Type", "application/json")
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            result = json.loads(resp.read().decode())
            dt = (time.time()-t0)*1000
            return result, dt
    except Exception as e:
        return {"success":False,"error":str(e)}, (time.time()-t0)*1000

def test(name, method, path, data=None, check=None):
    result, dt = call(method, path, data)
    ok = result.get("success", False)
    if check and ok:
        ok = check(result)
    status = "✓ PASS" if ok else "✗ FAIL"
    detail = ""
    if ok:
        d = result.get("data", [])
        if isinstance(d, list):
            detail = f" ({len(d)} rows, {dt:.0f}ms)"
        elif isinstance(d, dict):
            detail = f" ({dt:.0f}ms)"
    else:
        detail = f" - {result.get('error','unknown')}"
    print(f"  {status} {name}{detail}")
    return ok

print("=" * 60)
print("  芯擎科技官网 — 全链路API验证")
print("=" * 60)

results = []

# 1. 健康检查
print("\n[基础服务]")
results.append(test("健康检查", "GET", "/health", check=lambda r: r["data"]["status"]=="healthy"))

# 2. DSQL模板列表
print("\n[DSQL引擎]")
results.append(test("SQL模板列表", "GET", "/admin/sqls", check=lambda r: len(r["data"])>=10))

# 3. 芯片公司DSQL执行
print("\n[芯片业务数据]")
results.append(test("产品列表", "POST", "/dsql/execute", {"sql_code":"chip_products_list","params":{}},
    check=lambda r: len(r["data"])==6))
results.append(test("新闻列表", "POST", "/dsql/execute", {"sql_code":"chip_news_list","params":{}},
    check=lambda r: len(r["data"])==6))
results.append(test("案例列表", "POST", "/dsql/execute", {"sql_code":"chip_cases_list","params":{}},
    check=lambda r: len(r["data"])==3))
results.append(test("团队列表", "POST", "/dsql/execute", {"sql_code":"chip_team_list","params":{}},
    check=lambda r: len(r["data"])==3))
results.append(test("首页推荐产品", "POST", "/dsql/execute", {"sql_code":"chip_home_products","params":{}},
    check=lambda r: len(r["data"])>=3))
results.append(test("首页最新新闻", "POST", "/dsql/execute", {"sql_code":"chip_home_news","params":{}},
    check=lambda r: len(r["data"])==3))
results.append(test("首页案例", "POST", "/dsql/execute", {"sql_code":"chip_home_cases","params":{}},
    check=lambda r: len(r["data"])==3))

# 4. 带参数的DSQL
print("\n[动态参数查询]")
results.append(test("产品按分类筛选", "POST", "/dsql/execute",
    {"sql_code":"chip_products_by_category","params":{"category":"AI计算"}},
    check=lambda r: len(r["data"])>=2))
results.append(test("新闻详情", "POST", "/dsql/execute",
    {"sql_code":"chip_news_detail","params":{"id":1}},
    check=lambda r: len(r["data"])==1 and "XE-A2" in r["data"][0]["title"]))

# 5. 知识图谱
print("\n[知识图谱引擎]")
results.append(test("图谱全量", "GET", "/kg/graph",
    check=lambda r: r["data"]["vertex_count"]>=30 and r["data"]["edge_count"]>=40))
results.append(test("图谱邻居查询", "POST", "/kg/traverse",
    {"vertex_id":"chip:product:1","direction":"both"},
    check=lambda r: len(r["data"].get("neighbors",[]))>=1))

# 6. 留言提交
print("\n[业务写入]")
results.append(test("提交留言", "POST", "/website/message",
    {"name":"测试客户","phone":"13800138000","email":"test@xinengine.com",
     "company":"测试公司","content":"API验证测试留言"},
    check=lambda r: r.get("success", False)))

# 7. AI接口
print("\n[AI引擎]")
results.append(test("AI对话", "POST", "/ai/assistant",
    {"message":"芯擎科技有哪些产品？","app_key":"xinengine"},
    check=lambda r: r.get("success", False) or "error" in r))

# 总结
print("\n" + "=" * 60)
passed = sum(results)
total = len(results)
print(f"  验证结果: {passed}/{total} 通过")
if passed == total:
    print("  ✓ 全链路验证通过! 芯擎科技官网已通过MOX系统从0-1开发完成")
else:
    print(f"  ✗ {total-passed} 项未通过，需要修复")
print("=" * 60)

# 打印数据样本
print("\n[数据样本]")
r, _ = call("POST", "/dsql/execute", {"sql_code":"chip_products_list","params":{}})
if r.get("success"):
    print(f"  产品(前2): {[p['name'] for p in r['data'][:2]]}")
r, _ = call("GET", "/kg/graph")
if r.get("success"):
    d = r["data"]
    print(f"  图谱: {d['vertex_count']}顶点 / {d['edge_count']}边")
    types = {}
    for n in d["nodes"]:
        types[n["type"]] = types.get(n["type"],0)+1
    print(f"  顶点类型分布: {types}")
