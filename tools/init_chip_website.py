#!/usr/bin/env python3
"""
芯擎科技官网 — MOX平台初始化数据脚本
运行: python tools/init_chip_website.py
功能: 创建应用注册 + 业务数据 + DSQL模板
"""
import os, sys, json, sqlite3
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
META_DB = os.path.join(ROOT, "platform", "mox-server", "mox_meta.db")
BIZ_DB = os.path.join(ROOT, "platform", "mox-server", "mox_business.db")

APP_KEY = "xinengine"
APP_NAME = "芯擎科技"

def now_iso():
    return datetime.now(CST).isoformat()

def get_conn(db_path):
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn

def init_meta():
    """初始化元数据库表（重建，确保结构一致）"""
    conn = get_conn(META_DB)
    conn.executescript("""
    DROP TABLE IF EXISTS dsql_apps;
    DROP TABLE IF EXISTS dsql_sqls;
    DROP TABLE IF EXISTS dsql_datasources;
    CREATE TABLE dsql_apps (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT UNIQUE NOT NULL,
        app_name TEXT NOT NULL,
        description TEXT DEFAULT '',
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT
    );
    CREATE TABLE IF NOT EXISTS dsql_sqls (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        sql_code TEXT NOT NULL,
        sql_name TEXT DEFAULT '',
        datasource TEXT DEFAULT 'default',
        sql_template TEXT NOT NULL,
        version INTEGER DEFAULT 1,
        cache_ttl INTEGER DEFAULT 0,
        description TEXT DEFAULT '',
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT,
        UNIQUE(app_key, sql_code, version)
    );
    CREATE TABLE IF NOT EXISTS dsql_datasources (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        ds_code TEXT UNIQUE NOT NULL,
        ds_name TEXT DEFAULT '',
        db_type TEXT DEFAULT 'sqlite',
        host TEXT DEFAULT '', port INTEGER DEFAULT 0,
        database TEXT DEFAULT '', username TEXT DEFAULT '',
        password TEXT DEFAULT '', options TEXT DEFAULT '{}',
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT
    );
    """)
    conn.commit()
    conn.close()

def init_biz():
    """初始化业务数据库表（重建，确保结构一致）"""
    conn = get_conn(BIZ_DB)
    conn.executescript("""
    DROP TABLE IF EXISTS products;
    DROP TABLE IF EXISTS news;
    DROP TABLE IF EXISTS cases;
    DROP TABLE IF EXISTS team;
    DROP TABLE IF EXISTS messages;
    CREATE TABLE products (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        name TEXT NOT NULL,
        category TEXT DEFAULT '',
        spec TEXT DEFAULT '',
        icon TEXT DEFAULT '',
        badge TEXT DEFAULT '',
        description TEXT DEFAULT '',
        features TEXT DEFAULT '[]',
        sort_order INTEGER DEFAULT 0,
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT
    );
    CREATE TABLE IF NOT EXISTS news (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        title TEXT NOT NULL,
        category TEXT DEFAULT '',
        icon TEXT DEFAULT '',
        summary TEXT DEFAULT '',
        content TEXT DEFAULT '',
        publish_date TEXT DEFAULT '',
        view_count INTEGER DEFAULT 0,
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT
    );
    CREATE TABLE IF NOT EXISTS cases (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        title TEXT NOT NULL,
        industry TEXT DEFAULT '',
        icon TEXT DEFAULT '',
        description TEXT DEFAULT '',
        sort_order INTEGER DEFAULT 0,
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT
    );
    CREATE TABLE IF NOT EXISTS team (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        name TEXT NOT NULL,
        title TEXT DEFAULT '',
        icon TEXT DEFAULT '',
        description TEXT DEFAULT '',
        sort_order INTEGER DEFAULT 0,
        status TEXT DEFAULT 'active',
        created_at TEXT, updated_at TEXT
    );
    CREATE TABLE IF NOT EXISTS messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        name TEXT DEFAULT '',
        company TEXT DEFAULT '',
        phone TEXT DEFAULT '',
        email TEXT DEFAULT '',
        type TEXT DEFAULT 'product',
        message TEXT DEFAULT '',
        status TEXT DEFAULT 'new',
        created_at TEXT
    );
    """)
    conn.commit()
    conn.close()

def register_app():
    """注册应用"""
    conn = get_conn(META_DB)
    ts = now_iso()
    conn.execute("""INSERT OR REPLACE INTO dsql_apps (app_key, app_name, description, status, created_at, updated_at)
        VALUES (?,?,?,?,?,?)""", (APP_KEY, APP_NAME, "芯擎科技企业官网 — 国产高性能芯片设计公司", "active", ts, ts))
    conn.commit()
    conn.close()
    print(f"  ✓ 应用注册: {APP_NAME} ({APP_KEY})")

def insert_products():
    """插入产品数据"""
    products = [
        ("XE-A2 智算芯片","AI计算","5nm / 256TOPS / 120W","🧠","hot","面向AI推理与训练的高性能计算芯片，自研NPU架构，支持大模型部署。",json.dumps(["5nm制程","256TOPS","INT8/FP16","PCIe 5.0"],ensure_ascii=False),1),
        ("XE-N1 边缘AI芯片","AI计算","7nm / 64TOPS / 15W","⚡","new","边缘计算场景专用AI芯片，低功耗高性能，适用于智能摄像头、机器人等。",json.dumps(["7nm制程","64TOPS","15W功耗","多路视频编解码"],ensure_ascii=False),2),
        ("XE-T1 智能终端SoC","智能终端","12nm / 8核CPU / GPU","📱","pro","面向智能平板、学习机、工业终端的高性能SoC，集成CPU/GPU/NPU/ISP。",json.dumps(["8核CPU","Mali GPU","5TOPS NPU","4K显示"],ensure_ascii=False),3),
        ("XE-V1 车规级芯片","汽车电子","16nm / AEC-Q100 / -40~125℃","🚗","pro","通过AEC-Q100车规认证，适用于智能座舱、ADAS辅助驾驶、车载网关。",json.dumps(["AEC-Q100","功能安全ASIL-B","车载以太网","多路CAN FD"],ensure_ascii=False),4),
        ("XE-I1 物联网芯片","物联网","22nm / 双核 / 5mW待机","📡","hot","超低功耗物联网芯片，支持WiFi6/BLE5.2/Zigbee，适用于智能家居、传感器网络。",json.dumps(["5mW待机","WiFi6","BLE5.2","多种传感器接口"],ensure_ascii=False),5),
        ("XE-S1 安全芯片","物联网","28nm / 国密 / HSM","🔐","new","内置硬件安全模块，支持国密SM2/SM3/SM4，适用于金融、政务、身份认证场景。",json.dumps(["国密算法","HSM硬件安全","安全启动","真随机数"],ensure_ascii=False),6),
    ]
    conn = get_conn(BIZ_DB)
    ts = now_iso()
    conn.execute(f"DELETE FROM products WHERE app_key='{APP_KEY}'")
    for p in products:
        conn.execute("""INSERT INTO products (app_key,name,category,spec,icon,badge,description,features,sort_order,status,created_at,updated_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?)""", (APP_KEY,*p,"active",ts,ts))
    conn.commit()
    conn.close()
    print(f"  ✓ 产品数据: {len(products)} 条")

def insert_news():
    """插入新闻数据"""
    news = [
        ("芯擎科技发布5nm AI芯片XE-A2，算力达256TOPS","产品发布","🚀","全新一代AI计算芯片XE-A2正式量产，采用5nm先进制程，峰值算力256TOPS，性能功耗比提升40%。","","2025-08-20",1280),
        ("芯擎车规芯片XE-V1通过AEC-Q100 Grade 2认证","公司动态","🏆","XE-V1车规级芯片正式通过AEC-Q100 Grade 2认证，工作温度范围-40℃~105℃，可用于智能座舱与ADAS。","","2025-07-15",860),
        ("2025边缘AI芯片趋势：低功耗与大模型成为核心竞争力","行业洞察","📊","边缘AI芯片正从单纯算力竞争转向算力+功耗+生态综合竞争，端侧大模型部署成为新热点。","","2025-06-28",1520),
        ("芯擎科技与某头部车企达成战略合作","公司动态","🤝","双方将在智能座舱、ADAS辅助驾驶领域展开深度合作，芯擎XE-V1芯片将搭载于下一代车型。","","2025-06-10",720),
        ("XE-I1物联网芯片出货量突破5000万颗","产品发布","📈","XE-I1超低功耗物联网芯片累计出货量突破5000万颗，广泛应用于智能家居、工业传感等领域。","","2025-05-20",980),
        ("RISC-V架构在高性能计算领域的机遇与挑战","行业洞察","🔬","RISC-V开源指令集正从嵌入式向高性能计算渗透，生态完善和软件兼容是关键挑战。","","2025-05-08",1150),
    ]
    conn = get_conn(BIZ_DB)
    ts = now_iso()
    conn.execute(f"DELETE FROM news WHERE app_key='{APP_KEY}'")
    for n in news:
        conn.execute("""INSERT INTO news (app_key,title,category,icon,summary,content,publish_date,view_count,status,created_at,updated_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?)""", (APP_KEY,*n,"active",ts,ts))
    conn.commit()
    conn.close()
    print(f"  ✓ 新闻数据: {len(news)} 条")

def insert_cases():
    """插入案例数据"""
    cases = [
        ("某云服务商AI推理集群","AI计算","☁️","采用XE-A2芯片构建AI推理集群，支撑千万级日活用户的智能推荐与内容审核，TCO降低35%。",1),
        ("某新能源车企智能座舱","智能驾驶","🚙","XE-V1车规芯片搭载于智能座舱系统，支持多屏互动、语音交互、驾驶员监测，体验流畅。",2),
        ("某城市智慧安防项目","智慧城市","🏙️","XE-N1边缘AI芯片部署于2万路智能摄像头，实现实时人脸识别、行为分析、异常预警。",3),
        ("某制造集团智能工厂","工业物联网","🏭","XE-I1物联网芯片部署于5000+工业传感器，实现设备状态实时监测，故障率降低40%。",4),
        ("某头部家电品牌智能终端","智能家居","🏠","XE-T1智能终端SoC搭载于智能屏产品，支持语音交互、家庭中枢、多设备联动。",5),
        ("某银行安全认证系统","金融科技","🏦","XE-S1安全芯片用于银行U盾与身份认证终端，支持国密算法，通过金融级安全认证。",6),
    ]
    conn = get_conn(BIZ_DB)
    ts = now_iso()
    conn.execute(f"DELETE FROM cases WHERE app_key='{APP_KEY}'")
    for c in cases:
        conn.execute("""INSERT INTO cases (app_key,title,industry,icon,description,sort_order,status,created_at,updated_at)
            VALUES (?,?,?,?,?,?,?,?,?)""", (APP_KEY,*c,"active",ts,ts))
    conn.commit()
    conn.close()
    print(f"  ✓ 案例数据: {len(cases)} 条")

def insert_team():
    """插入团队数据"""
    team = [
        ("陈志远","创始人 & CEO","👤","前国际顶尖芯片公司首席架构师，20年芯片设计经验，主导过多款亿级出货量芯片。",1),
        ("林思琪","CTO & 联合创始人","👤","前知名半导体公司技术副总裁，专注CPU/GPU架构设计，拥有50+项芯片架构专利。",2),
        ("王浩然","副总裁 · 汽车电子","👤","前车规芯片产品线负责人，15年汽车电子经验，主导过多款通过AEC-Q100认证的车规芯片。",3),
    ]
    conn = get_conn(BIZ_DB)
    ts = now_iso()
    conn.execute(f"DELETE FROM team WHERE app_key='{APP_KEY}'")
    for t in team:
        conn.execute("""INSERT INTO team (app_key,name,title,icon,description,sort_order,status,created_at,updated_at)
            VALUES (?,?,?,?,?,?,?,?,?)""", (APP_KEY,*t,"active",ts,ts))
    conn.commit()
    conn.close()
    print(f"  ✓ 团队数据: {len(team)} 条")

def create_dsql_templates():
    """创建DSQL模板"""
    templates = [
        ("xinengine_products_list","产品列表","default",
         "SELECT id,name,category,spec,icon,badge,description,features,sort_order FROM products WHERE app_key='xinengine' AND status='active' ORDER BY sort_order ASC",
         60,"芯擎产品列表查询"),
        ("xinengine_news_list","新闻列表","default",
         "SELECT id,title,category,icon,summary,publish_date,view_count FROM news WHERE app_key='xinengine' AND status='active' ORDER BY publish_date DESC",
         120,"芯擎新闻列表查询"),
        ("xinengine_cases_list","案例列表","default",
         "SELECT id,title,industry,icon,description,sort_order FROM cases WHERE app_key='xinengine' AND status='active' ORDER BY sort_order ASC",
         120,"芯擎案例列表查询"),
        ("xinengine_team_list","团队列表","default",
         "SELECT id,name,title,icon,description,sort_order FROM team WHERE app_key='xinengine' AND status='active' ORDER BY sort_order ASC",
         300,"芯擎团队列表查询"),
        ("xinengine_message_create","留言创建","default",
         "INSERT INTO messages (app_key,name,company,phone,email,type,message,status,created_at) VALUES ('xinengine',:name,:company,:phone,:email,:type,:message,'new',:created_at)",
         0,"芯擎联系表单留言"),
    ]
    conn = get_conn(META_DB)
    ts = now_iso()
    count = 0
    for t in templates:
        conn.execute("""INSERT OR REPLACE INTO dsql_sqls (app_key,sql_code,sql_name,datasource,sql_template,version,cache_ttl,description,status,created_at,updated_at)
            VALUES ('xinengine',?,?,?,?,1,?,?, 'active',?,?)""", (t[0],t[1],t[2],t[3],t[4],t[5],ts,ts))
        count += 1
    conn.commit()
    conn.close()
    print(f"  ✓ DSQL模板: {count} 条")

def main():
    print("="*60)
    print("  芯擎科技官网 — MOX平台初始化")
    print("="*60)
    print("\n[1/4] 初始化数据库...")
    init_meta()
    init_biz()
    print("  ✓ 数据库表结构就绪")

    print("\n[2/4] 注册应用...")
    register_app()

    print("\n[3/4] 导入业务数据...")
    insert_products()
    insert_news()
    insert_cases()
    insert_team()

    print("\n[4/4] 创建DSQL模板...")
    create_dsql_templates()

    print("\n" + "="*60)
    print("  初始化完成!")
    print(f"  应用: {APP_NAME} ({APP_KEY})")
    print(f"  前端: frontend-ui/chip-website/index.html")
    print(f"  后端API: http://localhost:8600/api/dsql/execute/")
    print("="*60)

if __name__ == "__main__":
    main()
