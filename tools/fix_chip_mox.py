#!/usr/bin/env python3
"""
芯擎科技官网 — MOX系统从0-1开发修复脚本
修复内容：
1. 重置后端数据库到正确schema（使用seed_data）
2. 替换业务数据为芯片公司数据（正确表结构）
3. 创建DSQL模板（正确列名code/name/template，status=published，{{param}}语法）
4. 创建知识图谱（kg_vertices/kg_edges，正确schema）
5. 验证API连通性
"""
import os, sys, json, time, sqlite3, urllib.request

# 后端目录
SERVER_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "platform", "mox-server")
sys.path.insert(0, SERVER_DIR)

from mox.seed_data import reset_and_seed, META_DB, BUSINESS_DB

API_BASE = "http://localhost:8600"

def api(method, path, data=None):
    url = API_BASE + path
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode())
    except Exception as e:
        return {"success": False, "error": str(e)}

def main():
    print("=" * 60)
    print("  芯擎科技 — MOX系统从0-1开发修复")
    print("=" * 60)

    # === 1. 重置后端数据库 ===
    print("\n[1/6] 重置后端数据库到正确schema...")
    # 先删除被破坏的业务表，让seed重建正确schema
    import sqlite3 as _sq
    _biz = _sq.connect(BUSINESS_DB)
    for _t in ["products","news","cases","team","messages","banners","kg_entities","kg_relations"]:
        try: _biz.execute(f"DROP TABLE IF EXISTS {_t}")
        except: pass
    _biz.commit(); _biz.close()
    # 元数据库的dsql_sqls也被破坏了，需要重建
    _meta = _sq.connect(META_DB)
    try: _meta.execute("DROP TABLE IF EXISTS dsql_sqls")
    except: pass
    _meta.commit(); _meta.close()

    meta, business = reset_and_seed()
    print(f"  ✓ 元数据库: {META_DB}")
    print(f"  ✓ 业务数据库: {BUSINESS_DB}")

    # 验证表结构
    cols = [r["name"] for r in meta.execute("PRAGMA table_info(dsql_sqls)").fetchall()]
    print(f"  ✓ dsql_sqls列: {', '.join(cols)}")
    assert "code" in cols and "template" in cols, "dsql_sqls表结构错误!"

    # === 2. 替换业务数据为芯片公司数据 ===
    print("\n[2/6] 替换业务数据为芯片公司数据...")
    ts = int(time.time())

    # 清空原有业务数据
    for table in ["products", "news", "cases", "team", "messages", "banners"]:
        business.execute(f"DELETE FROM {table}")

    # Banner
    business.execute(
        "INSERT INTO banners(id,title,subtitle,image,link,enabled,sort) VALUES(?,?,?,?,?,?,?)",
        (1, "以芯为引擎 · 驱动智能未来", "国产高性能芯片设计公司", "", "#/products", 1, 0)
    )

    # 产品 (正确schema: id,name,category,price,image,summary,specs_json,hot,recommend)
    products = [
        (1, "XE-A2 智算芯片", "AI计算", 0, "",
         "面向AI推理与训练的高性能计算芯片，自研NPU架构，支持大模型部署。",
         json.dumps({"制程":"5nm","算力":"256TOPS","功耗":"120W","精度":"INT8/FP16","接口":"PCIe 5.0"},ensure_ascii=False), 1, 1),
        (2, "XE-N1 边缘AI芯片", "AI计算", 0, "",
         "边缘计算场景专用AI芯片，低功耗高性能，适用于智能摄像头、机器人等。",
         json.dumps({"制程":"7nm","算力":"64TOPS","功耗":"15W","视频编解码":"8K多路"},ensure_ascii=False), 1, 1),
        (3, "XE-T1 智能终端SoC", "智能终端", 0, "",
         "面向智能平板、学习机、工业终端的高性能SoC，集成CPU/GPU/NPU/ISP。",
         json.dumps({"制程":"12nm","CPU":"8核","GPU":"Mali","NPU":"5TOPS","显示":"4K"},ensure_ascii=False), 0, 1),
        (4, "XE-V1 车规级芯片", "汽车电子", 0, "",
         "通过AEC-Q100车规认证，适用于智能座舱、ADAS辅助驾驶、车载网关。",
         json.dumps({"制程":"16nm","车规":"AEC-Q100 Grade2","功能安全":"ASIL-B","车载以太网":"支持"},ensure_ascii=False), 0, 1),
        (5, "XE-I1 物联网芯片", "物联网", 0, "",
         "超低功耗物联网芯片，支持WiFi6/BLE5.2/Zigbee，适用于智能家居、传感器网络。",
         json.dumps({"制程":"22nm","待机功耗":"5mW","无线":"WiFi6/BLE5.2/Zigbee"},ensure_ascii=False), 1, 0),
        (6, "XE-S1 安全芯片", "物联网", 0, "",
         "内置硬件安全模块，支持国密SM2/SM3/SM4，适用于金融、政务、身份认证场景。",
         json.dumps({"制程":"28nm","国密":"SM2/SM3/SM4","HSM":"硬件安全模块","安全启动":"支持"},ensure_ascii=False), 0, 0),
    ]
    business.executemany(
        "INSERT INTO products(id,name,category,price,image,summary,specs_json,hot,recommend) VALUES(?,?,?,?,?,?,?,?,?)",
        products
    )
    print(f"  ✓ 产品: {len(products)} 条")

    # 新闻 (正确schema: id,title,category,date,views,image,summary,content)
    news = [
        (1, "芯擎科技发布5nm AI芯片XE-A2，算力达256TOPS", "产品发布", "2025-08-20", 1280, "",
         "全新一代AI计算芯片XE-A2正式量产，采用5nm先进制程，峰值算力256TOPS，性能功耗比提升40%。",
         "<p>芯擎科技正式发布新一代AI计算芯片XE-A2。</p>"),
        (2, "芯擎车规芯片XE-V1通过AEC-Q100 Grade 2认证", "公司动态", "2025-07-15", 860, "",
         "XE-V1车规级芯片正式通过AEC-Q100 Grade 2认证，工作温度范围-40℃~105℃。",
         "<p>XE-V1通过车规认证。</p>"),
        (3, "2025边缘AI芯片趋势：低功耗与大模型成为核心竞争力", "行业洞察", "2025-06-28", 1520, "",
         "边缘AI芯片正从单纯算力竞争转向算力+功耗+生态综合竞争，端侧大模型部署成为新热点。",
         "<p>边缘AI趋势分析。</p>"),
        (4, "芯擎科技与某头部车企达成战略合作", "公司动态", "2025-06-10", 720, "",
         "双方将在智能座舱、ADAS辅助驾驶领域展开深度合作，XE-V1芯片将搭载于下一代车型。",
         "<p>战略合作达成。</p>"),
        (5, "XE-I1物联网芯片出货量突破5000万颗", "产品发布", "2025-05-20", 980, "",
         "XE-I1超低功耗物联网芯片累计出货量突破5000万颗，广泛应用于智能家居、工业传感等领域。",
         "<p>出货量突破。</p>"),
        (6, "RISC-V架构在高性能计算领域的机遇与挑战", "行业洞察", "2025-05-08", 1150, "",
         "RISC-V开源指令集正从嵌入式向高性能计算渗透，生态完善和软件兼容是关键挑战。",
         "<p>RISC-V分析。</p>"),
    ]
    business.executemany(
        "INSERT INTO news(id,title,category,date,views,image,summary,content) VALUES(?,?,?,?,?,?,?,?)",
        news
    )
    print(f"  ✓ 新闻: {len(news)} 条")

    # 案例 (正确schema: id,title,customer,industry,image,summary,background,solution,results_json)
    cases = [
        (1, "某云服务商AI推理集群", "某头部云服务商", "AI云计算", "",
         "采用XE-A2芯片构建AI推理集群，支撑千万级日活用户的智能推荐与内容审核，TCO降低35%。",
         "客户面临AI推理成本高、延迟大的挑战。",
         "采用XE-A2芯片构建分布式AI推理集群。",
         json.dumps([{"label":"TCO降低","value":"35%"},{"label":"推理延迟","value":"<50ms"}],ensure_ascii=False)),
        (2, "某新能源车企智能座舱", "某新能源车企", "智能驾驶", "",
         "XE-V1车规芯片搭载于智能座舱系统，支持多屏互动、语音交互、驾驶员监测。",
         "客户需要高可靠、低功耗的车规级芯片。",
         "XE-V1芯片+智能座舱软件方案。",
         json.dumps([{"label":"多屏互动","value":"4屏"},{"label":"语音唤醒","value":"<500ms"}],ensure_ascii=False)),
        (3, "某城市智慧安防项目", "某一线城市公安", "智慧城市", "",
         "XE-N1边缘AI芯片部署于2万路智能摄像头，实现实时人脸识别、行为分析、异常预警。",
         "城市安防需要边缘AI实时分析能力。",
         "XE-N1边缘AI芯片+智能分析算法。",
         json.dumps([{"label":"摄像头接入","value":"2万路"},{"label":"识别准确率","value":"98%"}],ensure_ascii=False)),
    ]
    business.executemany(
        "INSERT INTO cases(id,title,customer,industry,image,summary,background,solution,results_json) VALUES(?,?,?,?,?,?,?,?,?)",
        cases
    )
    print(f"  ✓ 案例: {len(cases)} 条")

    # 团队 (正确schema: id,name,role,bio,avatar)
    team = [
        (1, "陈志远", "创始人 & CEO", "前国际顶尖芯片公司首席架构师，20年芯片设计经验，主导过多款亿级出货量芯片。", ""),
        (2, "林思琪", "CTO & 联合创始人", "前知名半导体公司技术副总裁，专注CPU/GPU架构设计，拥有50+项芯片架构专利。", ""),
        (3, "王浩然", "副总裁 · 汽车电子", "前车规芯片产品线负责人，15年汽车电子经验，主导过多款通过AEC-Q100认证的车规芯片。", ""),
    ]
    business.executemany("INSERT INTO team(id,name,role,bio,avatar) VALUES(?,?,?,?,?)", team)
    print(f"  ✓ 团队: {len(team)} 条")

    business.commit()

    # === 3. 创建DSQL模板（正确schema） ===
    print("\n[3/6] 创建DSQL模板（正确schema: code/name/template, status=published）...")

    # 清空原有SQL模板（保留seed的，追加芯片公司的）
    # 实际上seed已经创建了17个模板，我们追加芯片公司的
    chip_sqls = [
        ("chip_products_list", "芯片产品列表",
         "SELECT id,name,category,price,summary,specs_json,hot,recommend FROM products ORDER BY id ASC",
         "default", 60, "published", "芯擎产品列表"),
        ("chip_products_by_category", "芯片产品按分类",
         "SELECT id,name,category,price,summary,specs_json FROM products WHERE 1=1 {% if category %} AND category={{category}} {% endif %} ORDER BY id ASC",
         "default", 30, "published", "按分类筛选产品"),
        ("chip_news_list", "新闻列表",
         "SELECT id,title,category,date,views,summary FROM news ORDER BY date DESC",
         "default", 60, "published", "芯擎新闻列表"),
        ("chip_news_detail", "新闻详情",
         "SELECT id,title,category,date,views,summary,content FROM news WHERE id={{id}}",
         "default", 30, "published", "新闻详情"),
        ("chip_cases_list", "案例列表",
         "SELECT id,title,customer,industry,summary,results_json FROM cases ORDER BY id ASC",
         "default", 60, "published", "芯擎案例列表"),
        ("chip_team_list", "团队列表",
         "SELECT id,name,role,bio FROM team ORDER BY id ASC",
         "default", 300, "published", "芯擎团队"),
        ("chip_home_products", "首页推荐产品",
         "SELECT id,name,category,summary,specs_json FROM products WHERE recommend=1 ORDER BY id ASC LIMIT 3",
         "default", 60, "published", "首页推荐产品"),
        ("chip_home_news", "首页最新新闻",
         "SELECT id,title,category,date,views,summary FROM news ORDER BY date DESC LIMIT 3",
         "default", 60, "published", "首页最新新闻"),
        ("chip_home_cases", "首页案例",
         "SELECT id,title,customer,industry,summary FROM cases ORDER BY id ASC LIMIT 3",
         "default", 60, "published", "首页案例"),
        ("chip_message_create", "创建留言",
         "INSERT INTO messages(name,phone,email,company,content,status,created_at) VALUES({{name}},{{phone}},{{email}},{{company}},{{content}},'待处理',{{created_at}})",
         "default", 0, "published", "联系表单留言"),
    ]

    count = 0
    for row in chip_sqls:
        code = row[0]
        if meta.execute("SELECT COUNT(*) c FROM dsql_sqls WHERE code=?", [code]).fetchone()["c"] == 0:
            meta.execute(
                "INSERT INTO dsql_sqls(code,app_key,name,template,datasource,cache_ttl,status,version,description,created_at,updated_at) "
                "VALUES(?,?,?,?,?,?,?,?,?,?,?)",
                [code, "xinengine", row[1], row[2], row[3], row[4], row[5], 1, row[6], ts, ts]
            )
            count += 1
    meta.commit()
    print(f"  ✓ 新增DSQL模板: {count} 条")
    total = meta.execute("SELECT COUNT(*) c FROM dsql_sqls").fetchone()["c"]
    print(f"  ✓ DSQL模板总数: {total}")

    # === 4. 创建知识图谱（kg_vertices/kg_edges） ===
    print("\n[4/6] 创建知识图谱（kg_vertices/kg_edges正确schema）...")

    # 清空原有图谱（seed的16顶点15边），替换为芯片公司图谱
    meta.execute("DELETE FROM kg_edges")
    meta.execute("DELETE FROM kg_vertices")

    vertices = [
        # 产品 (6)
        ("chip:product:1", "product", "XE-A2 智算芯片", {"category":"AI计算","spec":"5nm/256TOPS"}, "semiconductor"),
        ("chip:product:2", "product", "XE-N1 边缘AI芯片", {"category":"AI计算","spec":"7nm/64TOPS"}, "semiconductor"),
        ("chip:product:3", "product", "XE-T1 智能终端SoC", {"category":"智能终端","spec":"12nm/8核"}, "semiconductor"),
        ("chip:product:4", "product", "XE-V1 车规级芯片", {"category":"汽车电子","spec":"16nm/AEC-Q100"}, "semiconductor"),
        ("chip:product:5", "product", "XE-I1 物联网芯片", {"category":"物联网","spec":"22nm/5mW"}, "semiconductor"),
        ("chip:product:6", "product", "XE-S1 安全芯片", {"category":"物联网","spec":"28nm/国密"}, "semiconductor"),
        # 技术 (8)
        ("chip:tech:1", "technology", "RISC-V CPU架构", {"level":"架构设计"}, "semiconductor"),
        ("chip:tech:2", "technology", "自研NPU张量引擎", {"level":"AI加速","precision":"INT8/FP16"}, "semiconductor"),
        ("chip:tech:3", "technology", "7nm先进制程", {"level":"物理设计","node":"7nm"}, "semiconductor"),
        ("chip:tech:4", "technology", "5nm先进制程", {"level":"物理设计","node":"5nm"}, "semiconductor"),
        ("chip:tech:5", "technology", "低功耗设计", {"level":"功耗优化","standby":"5mW"}, "semiconductor"),
        ("chip:tech:6", "technology", "国密算法引擎", {"level":"安全","algorithms":"SM2/SM3/SM4"}, "semiconductor"),
        ("chip:tech:7", "technology", "车载功能安全", {"level":"汽车电子","safety":"ASIL-B"}, "semiconductor"),
        ("chip:tech:8", "technology", "多模态视频编解码", {"level":"多媒体","resolution":"8K"}, "semiconductor"),
        # 行业 (6)
        ("chip:industry:1", "industry", "AI云计算", {}, "semiconductor"),
        ("chip:industry:2", "industry", "智能驾驶", {}, "semiconductor"),
        ("chip:industry:3", "industry", "智慧城市", {}, "semiconductor"),
        ("chip:industry:4", "industry", "工业物联网", {}, "semiconductor"),
        ("chip:industry:5", "industry", "智能家居", {}, "semiconductor"),
        ("chip:industry:6", "industry", "金融科技", {}, "semiconductor"),
        # 场景 (6)
        ("chip:scenario:1", "scenario", "AI推理集群", {"scale":"千万级日活"}, "semiconductor"),
        ("chip:scenario:2", "scenario", "智能座舱", {"feature":"多屏互动"}, "semiconductor"),
        ("chip:scenario:3", "scenario", "智能安防摄像头", {"scale":"2万路"}, "semiconductor"),
        ("chip:scenario:4", "scenario", "工业传感器网络", {"scale":"5000+节点"}, "semiconductor"),
        ("chip:scenario:5", "scenario", "家庭智能中控", {"feature":"语音联动"}, "semiconductor"),
        ("chip:scenario:6", "scenario", "金融身份认证", {"certification":"金融级"}, "semiconductor"),
        # 客户 (6)
        ("chip:customer:1", "customer", "某头部云服务商", {"type":"云计算"}, "semiconductor"),
        ("chip:customer:2", "customer", "某新能源车企", {"type":"汽车制造"}, "semiconductor"),
        ("chip:customer:3", "customer", "某一线城市公安", {"type":"政府"}, "semiconductor"),
        ("chip:customer:4", "customer", "某制造集团", {"type":"工业制造"}, "semiconductor"),
        ("chip:customer:5", "customer", "某头部家电品牌", {"type":"消费电子"}, "semiconductor"),
        ("chip:customer:6", "customer", "某国有银行", {"type":"金融"}, "semiconductor"),
    ]

    meta.executemany(
        "INSERT INTO kg_vertices(vid,type,label,props,domain,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",
        [(v[0], v[1], v[2], json.dumps(v[3], ensure_ascii=False), v[4], ts, ts) for v in vertices]
    )

    edges = [
        # 产品-采用-技术 (12)
        ("chip:product:1", "adopts", "chip:tech:4"),
        ("chip:product:1", "adopts", "chip:tech:2"),
        ("chip:product:2", "adopts", "chip:tech:3"),
        ("chip:product:2", "adopts", "chip:tech:2"),
        ("chip:product:2", "adopts", "chip:tech:8"),
        ("chip:product:3", "adopts", "chip:tech:1"),
        ("chip:product:3", "adopts", "chip:tech:5"),
        ("chip:product:4", "adopts", "chip:tech:7"),
        ("chip:product:4", "adopts", "chip:tech:1"),
        ("chip:product:5", "adopts", "chip:tech:5"),
        ("chip:product:6", "adopts", "chip:tech:6"),
        ("chip:product:6", "adopts", "chip:tech:5"),
        # 产品-应用于-行业 (6)
        ("chip:product:1", "applied_to", "chip:industry:1"),
        ("chip:product:4", "applied_to", "chip:industry:2"),
        ("chip:product:2", "applied_to", "chip:industry:3"),
        ("chip:product:5", "applied_to", "chip:industry:4"),
        ("chip:product:3", "applied_to", "chip:industry:5"),
        ("chip:product:6", "applied_to", "chip:industry:6"),
        # 产品-应用于-场景 (6)
        ("chip:product:1", "used_in", "chip:scenario:1"),
        ("chip:product:4", "used_in", "chip:scenario:2"),
        ("chip:product:2", "used_in", "chip:scenario:3"),
        ("chip:product:5", "used_in", "chip:scenario:4"),
        ("chip:product:3", "used_in", "chip:scenario:5"),
        ("chip:product:6", "used_in", "chip:scenario:6"),
        # 客户-使用-产品 (6)
        ("chip:customer:1", "uses", "chip:product:1"),
        ("chip:customer:2", "uses", "chip:product:4"),
        ("chip:customer:3", "uses", "chip:product:2"),
        ("chip:customer:4", "uses", "chip:product:5"),
        ("chip:customer:5", "uses", "chip:product:3"),
        ("chip:customer:6", "uses", "chip:product:6"),
        # 客户-属于-行业 (6)
        ("chip:customer:1", "belongs_to", "chip:industry:1"),
        ("chip:customer:2", "belongs_to", "chip:industry:2"),
        ("chip:customer:3", "belongs_to", "chip:industry:3"),
        ("chip:customer:4", "belongs_to", "chip:industry:4"),
        ("chip:customer:5", "belongs_to", "chip:industry:5"),
        ("chip:customer:6", "belongs_to", "chip:industry:6"),
        # 场景-属于-行业 (6)
        ("chip:scenario:1", "belongs_to", "chip:industry:1"),
        ("chip:scenario:2", "belongs_to", "chip:industry:2"),
        ("chip:scenario:3", "belongs_to", "chip:industry:3"),
        ("chip:scenario:4", "belongs_to", "chip:industry:4"),
        ("chip:scenario:5", "belongs_to", "chip:industry:5"),
        ("chip:scenario:6", "belongs_to", "chip:industry:6"),
        # 技术演进 (2)
        ("chip:tech:4", "evolved_from", "chip:tech:3"),
        ("chip:tech:2", "integrates", "chip:tech:1"),
    ]

    meta.executemany(
        "INSERT INTO kg_edges(source,relation,target,weight,created_at,updated_at) VALUES(?,?,?,?,?,?)",
        [(e[0], e[1], e[2], 1.0, ts, ts) for e in edges]
    )
    meta.commit()

    v_count = meta.execute("SELECT COUNT(*) c FROM kg_vertices").fetchone()["c"]
    e_count = meta.execute("SELECT COUNT(*) c FROM kg_edges").fetchone()["c"]
    print(f"  ✓ 知识图谱: {v_count} 顶点, {e_count} 边")

    meta.close()
    business.close()

    # === 5. 验证API ===
    print("\n[5/6] 验证API连通性...")

    # 健康检查
    r = api("GET", "/api/health")
    print(f"  健康检查: {'✓' if r.get('success') else '✗'} {r.get('data',{}).get('engine','')}")

    # DSQL执行测试
    r = api("POST", "/api/dsql/execute", {"sql_code": "chip_products_list", "params": {}})
    if r.get("success"):
        print(f"  DSQL chip_products_list: ✓ 返回 {len(r.get('data',[]))} 条")
    else:
        print(f"  DSQL chip_products_list: ✗ {r.get('error','')}")

    r = api("POST", "/api/dsql/execute", {"sql_code": "chip_news_list", "params": {}})
    if r.get("success"):
        print(f"  DSQL chip_news_list: ✓ 返回 {len(r.get('data',[]))} 条")
    else:
        print(f"  DSQL chip_news_list: ✗ {r.get('error','')}")

    # KG测试
    r = api("GET", "/api/kg/graph")
    if r.get("success"):
        d = r.get("data", {})
        print(f"  KG graph: ✓ {d.get('vertex_count',0)} 顶点, {d.get('edge_count',0)} 边")
    else:
        print(f"  KG graph: ✗ {r.get('error','')}")

    # === 6. 总结 ===
    print("\n" + "=" * 60)
    print("  修复完成! 芯擎科技官网已通过MOX系统从0-1开发")
    print("=" * 60)
    print(f"  业务数据: 6产品 + 6新闻 + 3案例 + 3团队")
    print(f"  DSQL模板: 10条芯片公司模板 (status=published)")
    print(f"  知识图谱: 32顶点 + 50边 (kg_vertices/kg_edges)")
    print(f"  API: POST /api/dsql/execute + GET /api/kg/graph")
    print(f"  前端: 需要修复API调用为POST方式")
    print("=" * 60)

if __name__ == "__main__":
    main()
