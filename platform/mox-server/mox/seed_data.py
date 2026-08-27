# -*- coding: utf-8 -*-
"""
mox 种子数据与元数据库初始化
============================
创建元数据库 schema，并注入：
- 数据源（default = 内存 SQLite 演示业务库；业务表：products/news/cases/team/messages）
- 业务数据（企业官网全部实体数据）
- DSQL SQL 定义（与前端 API 层 sql_code 一一对应，真正"数据库管理 SQL"）
- 自研知识图谱（产品/案例/新闻 实体关系，跨行业融合）
- 角色与字段级权限（演示：role=guest 隐藏产品价格等敏感字段）
"""
from __future__ import annotations

import json
import os
import sqlite3
from typing import Any

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
META_DB = os.path.join(BASE_DIR, "mox_meta.db")
BUSINESS_DB = os.path.join(BASE_DIR, "mox_business.db")


# ---------------- 元数据库 schema ----------------
META_SCHEMA = [
    """CREATE TABLE IF NOT EXISTS datasources(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT UNIQUE NOT NULL,
        driver TEXT NOT NULL DEFAULT 'sqlite',
        config_json TEXT NOT NULL DEFAULT '{}',
        enabled INTEGER DEFAULT 1,
        created_at INTEGER, updated_at INTEGER
    )""",
    """CREATE TABLE IF NOT EXISTS dsql_sqls(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        code TEXT UNIQUE NOT NULL,
        name TEXT, template TEXT NOT NULL,
        datasource TEXT DEFAULT 'default',
        cache_ttl INTEGER DEFAULT 0,
        status TEXT DEFAULT 'draft',
        version INTEGER DEFAULT 1,
        description TEXT,
        created_at INTEGER, updated_at INTEGER
    )""",
    """CREATE TABLE IF NOT EXISTS kg_vertices(
        vid TEXT PRIMARY KEY,
        type TEXT, label TEXT, props TEXT DEFAULT '{}',
        domain TEXT DEFAULT 'default',
        created_at INTEGER, updated_at INTEGER
    )""",
    """CREATE TABLE IF NOT EXISTS kg_edges(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source TEXT NOT NULL, relation TEXT NOT NULL, target TEXT NOT NULL,
        weight REAL DEFAULT 1.0, created_at INTEGER, updated_at INTEGER,
        UNIQUE(source, relation, target)
    )""",
    """CREATE TABLE IF NOT EXISTS roles(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT UNIQUE NOT NULL, description TEXT
    )""",
    """CREATE TABLE IF NOT EXISTS users(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT UNIQUE NOT NULL, role TEXT NOT NULL, display_name TEXT
    )""",
    """CREATE TABLE IF NOT EXISTS field_permissions(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        resource TEXT NOT NULL, role TEXT NOT NULL, allowed_fields TEXT,
        UNIQUE(resource, role)
    )""",
    """CREATE TABLE IF NOT EXISTS audit_logs(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts INTEGER, trace_id TEXT, actor TEXT, action TEXT, detail TEXT
    )""",
    """CREATE TABLE IF NOT EXISTS messages(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT, phone TEXT, email TEXT, company TEXT, content TEXT,
        status TEXT DEFAULT '待处理', created_at INTEGER
    )""",
]


# ---------------- 业务库 schema + 官网数据 ----------------
BUSINESS_SCHEMA = [
    """CREATE TABLE IF NOT EXISTS products(
        id INTEGER PRIMARY KEY, name TEXT, category TEXT, price REAL,
        image TEXT, summary TEXT, specs_json TEXT, hot INTEGER, recommend INTEGER
    )""",
    """CREATE TABLE IF NOT EXISTS news(
        id INTEGER PRIMARY KEY, title TEXT, category TEXT, date TEXT,
        views INTEGER, image TEXT, summary TEXT, content TEXT
    )""",
    """CREATE TABLE IF NOT EXISTS cases(
        id INTEGER PRIMARY KEY, title TEXT, customer TEXT, industry TEXT,
        image TEXT, summary TEXT, background TEXT, solution TEXT,
        results_json TEXT
    )""",
    """CREATE TABLE IF NOT EXISTS team(
        id INTEGER PRIMARY KEY, name TEXT, role TEXT, bio TEXT, avatar TEXT
    )""",
]

PRODUCTS = [
    (1, "墨行智能终端 MX-T1", "智能硬件", 18999, "https://aka.doubaocdn.com/s/9JrzVnyYCT",
     "面向企业的一体化智能终端，融合高性能计算与极致显示。",
     json.dumps({"处理器": "国产八核处理器", "内存": "32GB DDR5", "存储": "1TB NVMe SSD",
                 "显示": "27英寸 4K 广色域", "系统": "UOS / 麒麟 双系统", "安全": "TCM安全芯片"},
                ensure_ascii=False), 1, 1),
    (2, "墨行企业平台 MX-Cloud", "软件平台", 0, "https://aka.doubaocdn.com/s/jIJ0W9Et3G",
     "企业级低代码与微服务一体化平台，让业务上线以天计。",
     json.dumps({"架构": "微服务 + 服务网格", "部署": "私有化 / 混合云", "集成": "300+ 连接器",
                 "低代码": "可视化编排", "高可用": "99.99%", "国产化": "全栈信创适配"},
                ensure_ascii=False), 1, 1),
    (3, "墨行数据引擎 MX-Data", "数据引擎", 0, "https://aka.doubaocdn.com/s/5AjjFy6J5q",
     "PB级数据存储与实时分析引擎，支持图谱化智能查询。",
     json.dumps({"存储": "PB级分布式存储", "查询": "SQL + 知识图谱双引擎", "实时": "毫秒级流式计算",
                 "性能": "百万级 QPS", "生态": "兼容主流数据源", "可视化": "全链路血缘追踪"},
                ensure_ascii=False), 0, 1),
    (4, "墨行AI加速芯片 MX-AI", "AI计算", 0, "https://aka.doubaocdn.com/s/UpTpzONWJU",
     "自研AI算力芯片，为深度学习推理提供澎湃算力。",
     json.dumps({"算力": "128 TOPS", "能效": "8W 典型功耗", "接口": "PCIe 4.0",
                 "框架": "支持主流推理框架", "场景": "边缘推理 / 数据中心", "国产": "全自主指令集"},
                ensure_ascii=False), 0, 0),
    (5, "墨行边缘网关 MX-Edge", "安全网关", 0, "https://aka.doubaocdn.com/s/8gRhtIYYza",
     "工业级边缘计算网关，集连接、计算、安全于一体。",
     json.dumps({"接口": "双千兆 + 4G/5G", "协议": "50+ 工业协议", "计算": "边缘AI推理",
                 "安全": "国密加密通道", "防护": "IP67 工业防护", "管理": "云边协同管理"},
                ensure_ascii=False), 1, 0),
]

NEWS = [
    (1, "墨行科技发布新一代AI企业平台，低代码+知识图谱双引擎上线", "产品发布", "2026-08-20",
     1280, "https://aka.doubaocdn.com/s/jIJ0W9Et3G",
     "全新一代墨行企业平台MX-Cloud 3.0正式发布，融合动态SQL引擎与自研知识图谱。",
     "<p>墨行科技正式发布新一代AI企业平台MX-Cloud 3.0，深度融合动态SQL引擎与自研知识图谱，实现企业数据操作全维度可配置。</p><p>平台内置DSQL引擎将所有业务SQL纳入数据库管理，支持模板渲染、参数校验、多级缓存与版本回滚；自研知识图谱让实体关系可视化，支持多跳关联查询。</p>"),
    (2, "墨行边缘网关通过信创产品认证，助力工业国产化", "公司动态", "2026-08-12",
     860, "https://aka.doubaocdn.com/s/EGGKe1GqyV",
     "墨行边缘计算网关MX-Edge正式通过信创产品认证，自主可控达到国家标准。",
     "<p>墨行科技自主研发的边缘计算网关MX-Edge近日通过信创产品认证。</p><p>MX-Edge内置国密加密通道与国产安全芯片，支持50余种工业协议接入，已广泛应用于智能制造、智慧能源等关键领域。</p>"),
    (3, "2026企业数字化转型趋势：数据引擎成为核心竞争力", "行业洞察", "2026-08-05",
     1520, "https://aka.doubaocdn.com/s/ssfJJNnkG0",
     "从数据存储到数据价值，企业数据引擎正从辅助工具演进为核心生产力。",
     "<p>2026年，企业数字化转型进入深水区，数据引擎成为驱动业务决策的核心竞争力。</p><p>墨行数据引擎MX-Data以PB级分布式存储与实时流式计算为基础，结合SQL与知识图谱双引擎，帮助企业构建数据资产。</p>"),
    (4, "墨行智慧城市大数据平台上线，服务城市治理现代化", "公司动态", "2026-07-28",
     980, "https://aka.doubaocdn.com/s/7IFDJDNteE",
     "墨行科技承建的某智慧城市大数据平台正式上线，以数据驱动城市治理现代化。",
     "<p>由墨行科技承建的大数据平台近日在某城市正式上线，实现城市运行一屏统览。</p><p>平台基于墨行数据引擎MX-Data构建，支撑亿级数据实时分析。</p>"),
    (5, "墨行助力某大型银行完成核心系统国产化改造", "公司动态", "2026-07-15",
     1130, "https://aka.doubaocdn.com/s/r5TOA4NSIv",
     "墨行科技助力某大型银行完成核心业务系统国产化改造，实现全栈自主可控。",
     "<p>墨行科技助力某大型银行完成核心业务系统国产化改造项目。</p><p>项目覆盖数据库、中间件、应用层全栈，改造后系统运行稳定，性能与可用性均达预期。</p>"),
    (6, "墨行数据引擎性能再创新高：百万级QPS实测", "产品发布", "2026-07-02",
     2010, "https://aka.doubaocdn.com/s/5AjjFy6J5q",
     "第三方机构实测中，墨行数据引擎MX-Data达到百万级QPS，刷新同类产品纪录。",
     "<p>墨行数据引擎MX-Data在独立第三方基准测试中，达到单机百万级QPS。</p><p>这得益于自研存储引擎、多级缓存与查询优化器。配合知识图谱引擎，复杂多跳关联查询保持毫秒级响应。</p>"),
]

CASES = [
    (1, "某大型银行数字化转型项目", "某大型国有银行", "金融",
     "https://aka.doubaocdn.com/s/r5TOA4NSIv",
     "以墨行数据引擎为核心，帮助银行完成核心系统国产化改造与数据中台建设。",
     "客户面临核心系统受制于人、数据孤岛严重的双重挑战。",
     "采用墨行数据引擎MX-Data搭建数据中台，结合知识图谱实现客户关系智能分析，并完成核心系统全栈国产化迁移。",
     json.dumps([{"label": "查询性能提升", "value": "5倍"}, {"label": "数据接入", "value": "2000+"},
                 {"label": "系统可用性", "value": "99.99%"}], ensure_ascii=False)),
    (2, "某制造企业智能工厂建设", "某新能源制造集团", "制造",
     "https://aka.doubaocdn.com/s/EGGKe1GqyV",
     "基于墨行边缘计算网关与工业数据平台，打造人机料法环全要素数字化智能工厂。",
     "产线设备种类多、协议杂，数据采集困难，生产管理依赖人工经验。",
     "部署墨行边缘网关MX-Edge实现产线设备统一接入，构建工业数据平台实现生产全流程可视、可控、可预测。",
     json.dumps([{"label": "设备联网率", "value": "98%"}, {"label": "生产效率提升", "value": "32%"},
                 {"label": "停机时间下降", "value": "45%"}], ensure_ascii=False)),
    (3, "某电商平台数据中台建设", "某头部电商平台", "电商",
     "https://aka.doubaocdn.com/s/PAOM8rdPFQ",
     "以墨行数据引擎构建电商数据中台，支撑千万级用户画像与实时推荐。",
     "业务高速增长，原有架构无法支撑实时分析与智能推荐。",
     "采用墨行数据引擎MX-Data构建实时数仓，结合AI算力芯片实现毫秒级用户画像与推荐服务。",
     json.dumps([{"label": "实时计算延迟", "value": "<50ms"}, {"label": "推荐转化提升", "value": "18%"},
                 {"label": "数据规模", "value": "PB级"}], ensure_ascii=False)),
]

TEAM = [
    (1, "莫国子", "创始人 & CEO", "深耕企业级软件与数据领域十余年，主导多项核心系统架构设计。",
     "https://aka.doubaocdn.com/s/JRt0jlNQnX"),
    (2, "林语棠", "产品总监", "曾主导多款企业级产品的从0到1，专注用户体验与业务价值。",
     "https://aka.doubaocdn.com/s/OUCiBUbqSJ"),
    (3, "周彦辰", "技术架构师", "资深全栈架构师，精通分布式系统、数据引擎与AI工程化。",
     "https://aka.doubaocdn.com/s/BhIdk7xkmZ"),
]

# ---------------- DSQL SQL 定义种子（对应官网 sql_code） ----------------
DSQL_SQLS = [
    ("home_banner_list", "首页Banner", "SELECT id,title,subtitle,image,link FROM banners WHERE enabled=1 ORDER BY sort ASC", "default", 60, "published", "首页轮播"),
    ("product_categories", "产品分类", "SELECT DISTINCT category AS name FROM products ORDER BY category", "default", 300, "published", "产品分类去重"),
    ("home_recommend_products", "首页推荐产品", "SELECT id,name,category,price,image,summary,hot,recommend FROM products WHERE recommend=1 ORDER BY id ASC LIMIT 3", "default", 60, "published", "首页推荐"),
    ("product_list", "产品列表(分类/关键字)", "SELECT id,name,category,price,image,summary,hot FROM products WHERE 1=1 {% if category %} AND category={{category}} {% endif %} {% if keyword %} AND (name LIKE {{keyword}} OR summary LIKE {{keyword}}) {% endif %} ORDER BY id ASC", "default", 30, "published", "产品列表动态筛选"),
    ("product_detail", "产品详情", "SELECT * FROM products WHERE id={{id}}", "default", 30, "published", "按ID查产品"),
    ("product_related", "相关产品", "SELECT id,name,category,price,image,summary FROM products WHERE id != {{id}} ORDER BY id ASC LIMIT 3", "default", 60, "published", "相关推荐"),
    ("news_list", "新闻列表(分类)", "SELECT id,title,category,date,views,image,summary FROM news WHERE 1=1 {% if category %} AND category={{category}} {% endif %} ORDER BY date DESC", "default", 30, "published", "新闻列表"),
    ("news_detail", "新闻详情", "SELECT id,title,category,date,views,image,summary,content FROM news WHERE id={{id}}", "default", 30, "published", "新闻详情"),
    ("case_list", "案例列表(行业)", "SELECT id,title,customer,industry,image,summary FROM cases WHERE 1=1 {% if industry %} AND industry={{industry}} {% endif %} ORDER BY id ASC", "default", 60, "published", "案例列表"),
    ("case_detail", "案例详情", "SELECT id,title,customer,industry,image,summary,background,solution,results_json AS results FROM cases WHERE id={{id}}", "default", 60, "published", "案例详情"),
    ("team_list", "核心团队", "SELECT id,name,role,bio,avatar FROM team ORDER BY id ASC", "default", 300, "published", "团队列表"),
    ("search_all", "全站搜索", "SELECT 'product' AS kind,id,name AS title,summary AS snippet FROM products WHERE name LIKE {{keyword}} OR summary LIKE {{keyword}} UNION ALL SELECT 'news',id,title,summary FROM news WHERE title LIKE {{keyword}} OR summary LIKE {{keyword}} UNION ALL SELECT 'case',id,title,summary FROM cases WHERE title LIKE {{keyword}} OR summary LIKE {{keyword}}", "default", 10, "published", "跨表联合搜索"),
    ("stats_dashboard", "管理看板统计", "SELECT (SELECT COUNT(*) FROM products) AS product_count, (SELECT COUNT(*) FROM news) AS news_count, (SELECT COUNT(*) FROM cases) AS case_count, (SELECT COUNT(*) FROM messages) AS message_count", "default", 15, "published", "看板KPI"),
    ("message_list", "留言列表", "SELECT id,name,phone,email,company,content,status,created_at FROM messages ORDER BY created_at DESC", "default", 5, "published", "留言管理"),
]

# ---------------- 图谱种子 ----------------
KG_VERTICES = [
    # 产品
    ("product:1", "product", "智能终端 MX-T1", {"category": "智能硬件"}, "hardware"),
    ("product:2", "product", "企业平台 MX-Cloud", {"category": "软件平台"}, "platform"),
    ("product:3", "product", "数据引擎 MX-Data", {"category": "数据引擎"}, "data"),
    ("product:4", "product", "AI芯片 MX-AI", {"category": "AI计算"}, "ai"),
    ("product:5", "product", "边缘网关 MX-Edge", {"category": "安全网关"}, "hardware"),
    # 分类
    ("category:1", "product_category", "智能硬件", {}, "default"),
    ("category:2", "product_category", "软件平台", {}, "default"),
    ("category:3", "product_category", "数据引擎", {}, "default"),
    ("category:4", "product_category", "AI计算", {}, "default"),
    ("category:5", "product_category", "安全网关", {}, "default"),
    # 案例（跨行业）
    ("case:1", "case", "银行数字化转型", {"industry": "金融"}, "finance"),
    ("case:2", "case", "制造智能工厂", {"industry": "制造"}, "manufacturing"),
    ("case:3", "case", "电商数据中台", {"industry": "电商"}, "ecommerce"),
    # 新闻
    ("news:1", "news", "MX-Cloud 3.0 发布", {}, "platform"),
    ("news:2", "news", "信创认证通过", {}, "hardware"),
    ("news:3", "news", "数据引擎趋势报告", {}, "data"),
]

KG_EDGES = [
    ("product:1", "belongs_to", "category:1"),
    ("product:2", "belongs_to", "category:2"),
    ("product:3", "belongs_to", "category:3"),
    ("product:4", "belongs_to", "category:4"),
    ("product:5", "belongs_to", "category:5"),
    ("case:1", "uses", "product:3"),
    ("case:1", "uses", "product:2"),
    ("case:2", "uses", "product:5"),
    ("case:2", "uses", "product:3"),
    ("case:3", "uses", "product:3"),
    ("case:3", "uses", "product:4"),
    ("news:1", "related_to", "product:2"),
    ("news:2", "related_to", "product:5"),
    ("news:3", "related_to", "product:3"),
    ("news:1", "related_to", "case:1"),
]

# ---------------- 角色 / 权限种子 ----------------
ROLES = [
    (1, "admin", "管理员：全部字段可见"),
    (2, "staff", "运营人员：业务字段可见"),
    (3, "guest", "访客：隐藏价格/联系方式等敏感字段"),
]
USERS = [
    (1, "admin", "admin", "系统管理员"),
    (2, "ops", "staff", "运营专员"),
    (3, "visitor", "guest", "访客"),
]
# 字段级权限演示：guest 角色查 product_list 时只允许看基础字段（隐藏 price）
FIELD_PERMISSIONS = [
    ("product_list", "guest", "id,name,category,image,summary,hot"),
    ("product_detail", "guest", "id,name,category,image,summary"),
]


# ---------------- 初始化函数 ----------------
def init_meta() -> "sqlite3.Connection":
    os.makedirs(BASE_DIR, exist_ok=True)
    conn = sqlite3.connect(META_DB)
    conn.row_factory = sqlite3.Row
    for ddl in META_SCHEMA:
        conn.execute(ddl)
    conn.commit()
    return conn


def init_business() -> "sqlite3.Connection":
    conn = sqlite3.connect(BUSINESS_DB)
    conn.row_factory = sqlite3.Row
    for ddl in BUSINESS_SCHEMA:
        conn.execute(ddl)
    # 幂等填充
    if conn.execute("SELECT COUNT(*) c FROM products").fetchone()["c"] == 0:
        conn.executemany(
            "INSERT INTO products(id,name,category,price,image,summary,specs_json,hot,recommend) "
            "VALUES(?,?,?,?,?,?,?,?,?)", PRODUCTS)
    if conn.execute("SELECT COUNT(*) c FROM news").fetchone()["c"] == 0:
        conn.executemany(
            "INSERT INTO news(id,title,category,date,views,image,summary,content) "
            "VALUES(?,?,?,?,?,?,?,?)", NEWS)
    if conn.execute("SELECT COUNT(*) c FROM cases").fetchone()["c"] == 0:
        conn.executemany(
            "INSERT INTO cases(id,title,customer,industry,image,summary,background,solution,results_json) "
            "VALUES(?,?,?,?,?,?,?,?,?)", CASES)
    if conn.execute("SELECT COUNT(*) c FROM team").fetchone()["c"] == 0:
        conn.executemany(
            "INSERT INTO team(id,name,role,bio,avatar) VALUES(?,?,?,?,?)", TEAM)
    conn.commit()
    return conn


def seed_all(meta: "sqlite3.Connection", business: "sqlite3.Connection") -> None:
    """注入数据源、SQL 定义、图谱、角色权限、业务数据。"""
    # 数据源
    if meta.execute("SELECT COUNT(*) c FROM datasources").fetchone()["c"] == 0:
        meta.execute(
            "INSERT INTO datasources(name,driver,config_json,enabled,created_at,updated_at) "
            "VALUES(?,?,?,?,?,?)",
            ["default", "sqlite", json.dumps({"dsn": BUSINESS_DB}), 1,
             int(__import__("time").time()), int(__import__("time").time())])
    # SQL 定义（幂等：已存在则跳过）
    for row in DSQL_SQLS:
        code = row[0]
        if meta.execute("SELECT COUNT(*) c FROM dsql_sqls WHERE code=?", [code]).fetchone()["c"] == 0:
            meta.execute(
                "INSERT INTO dsql_sqls(code,name,template,datasource,cache_ttl,status,version,"
                "description,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)",
                [row[0], row[1], row[2], row[3], row[4], row[5], 1, row[6],
                 int(__import__("time").time()), int(__import__("time").time())])
    # 图谱
    if meta.execute("SELECT COUNT(*) c FROM kg_vertices").fetchone()["c"] == 0:
        ts = int(__import__("time").time())
        meta.executemany(
            "INSERT INTO kg_vertices(vid,type,label,props,domain,created_at,updated_at) "
            "VALUES(?,?,?,?,?,?,?)",
            [(v[0], v[1], v[2], json.dumps(v[3], ensure_ascii=False), v[4], ts, ts) for v in KG_VERTICES])
        meta.executemany(
            "INSERT INTO kg_edges(source,relation,target,weight,created_at,updated_at) "
            "VALUES(?,?,?,?,?,?)",
            [(e[0], e[1], e[2], 1.0, ts, ts) for e in KG_EDGES])
    # 角色 / 用户 / 权限
    if meta.execute("SELECT COUNT(*) c FROM roles").fetchone()["c"] == 0:
        meta.executemany("INSERT INTO roles(id,name,description) VALUES(?,?,?)", ROLES)
        meta.executemany("INSERT INTO users(id,username,role,display_name) VALUES(?,?,?,?)", USERS)
        meta.executemany(
            "INSERT INTO field_permissions(resource,role,allowed_fields) VALUES(?,?,?)",
            FIELD_PERMISSIONS)
    meta.commit()


def reset_and_seed() -> tuple["sqlite3.Connection", "sqlite3.Connection"]:
    meta = init_meta()
    business = init_business()
    seed_all(meta, business)
    return meta, business
