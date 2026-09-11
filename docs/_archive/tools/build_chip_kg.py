#!/usr/bin/env python3
"""
芯擎科技 — 知识图谱构建脚本
构建实体: 产品/技术/制程/架构/行业/应用场景/客户
构建关系: 属于/应用于/采用/基于/合作/包含/竞争
运行: python tools/build_chip_kg.py
"""
import os, sys, json, sqlite3
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIZ_DB = os.path.join(ROOT, "platform", "mox-server", "mox_business.db")
META_DB = os.path.join(ROOT, "platform", "mox-server", "mox_meta.db")
APP_KEY = "xinengine"

def now_iso():
    return datetime.now(CST).isoformat()

def get_conn(db_path):
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn

def init_kg_tables():
    """初始化知识图谱表"""
    conn = get_conn(BIZ_DB)
    conn.executescript("""
    DROP TABLE IF EXISTS kg_entities;
    DROP TABLE IF EXISTS kg_relations;
    CREATE TABLE kg_entities (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        entity_type TEXT NOT NULL,
        name TEXT NOT NULL,
        properties TEXT DEFAULT '{}',
        description TEXT DEFAULT '',
        created_at TEXT, updated_at TEXT,
        UNIQUE(app_key, entity_type, name)
    );
    CREATE TABLE kg_relations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT DEFAULT 'default',
        source_id INTEGER NOT NULL,
        target_id INTEGER NOT NULL,
        relation_type TEXT NOT NULL,
        properties TEXT DEFAULT '{}',
        created_at TEXT,
        UNIQUE(app_key, source_id, target_id, relation_type)
    );
    CREATE INDEX idx_kg_entities_type ON kg_entities(app_key, entity_type);
    CREATE INDEX idx_kg_relations_source ON kg_relations(app_key, source_id);
    CREATE INDEX idx_kg_relations_target ON kg_relations(app_key, target_id);
    """)
    conn.commit()
    conn.close()
    print("  ✓ 知识图谱表结构就绪")

def build_entities():
    """构建知识图谱实体"""
    entities = []
    ts = now_iso()

    # === 产品实体 (6) ===
    products = [
        ("XE-A2 智算芯片", "product", {"spec":"5nm / 256TOPS / 120W", "category":"AI计算", "badge":"hot"}),
        ("XE-N1 边缘AI芯片", "product", {"spec":"7nm / 64TOPS / 15W", "category":"AI计算", "badge":"new"}),
        ("XE-T1 智能终端SoC", "product", {"spec":"12nm / 8核CPU / GPU", "category":"智能终端", "badge":"pro"}),
        ("XE-V1 车规级芯片", "product", {"spec":"16nm / AEC-Q100", "category":"汽车电子", "badge":"pro"}),
        ("XE-I1 物联网芯片", "product", {"spec":"22nm / 5mW待机", "category":"物联网", "badge":"hot"}),
        ("XE-S1 安全芯片", "product", {"spec":"28nm / 国密 / HSM", "category":"物联网", "badge":"new"}),
    ]
    for name, etype, props in products:
        entities.append((etype, name, json.dumps(props, ensure_ascii=False), f"芯擎科技{name}"))

    # === 技术实体 (8) ===
    techs = [
        ("RISC-V CPU架构", "technology", {"level":"架构设计", "patents":15}),
        ("自研NPU张量引擎", "technology", {"level":"AI加速", "precision":"INT8/FP16/FP32"}),
        ("7nm先进制程", "technology", {"level":"物理设计", "node":"7nm"}),
        ("5nm先进制程", "technology", {"level":"物理设计", "node":"5nm"}),
        ("低功耗设计", "technology", {"level":"功耗优化", "standby":"5mW"}),
        ("国密算法引擎", "technology", {"level":"安全", "algorithms":"SM2/SM3/SM4"}),
        ("车载功能安全", "technology", {"level":"汽车电子", "level_safety":"ASIL-B"}),
        ("多模态视频编解码", "technology", {"level":"多媒体", "resolution":"8K"}),
    ]
    for name, etype, props in techs:
        entities.append((etype, name, json.dumps(props, ensure_ascii=False), name))

    # === 行业实体 (6) ===
    industries = [
        ("AI云计算", "industry", {}),
        ("智能驾驶", "industry", {}),
        ("智慧城市", "industry", {}),
        ("工业物联网", "industry", {}),
        ("智能家居", "industry", {}),
        ("金融科技", "industry", {}),
    ]
    for name, etype, props in industries:
        entities.append((etype, name, json.dumps(props, ensure_ascii=False), name + "行业"))

    # === 应用场景实体 (6) ===
    scenarios = [
        ("AI推理集群", "scenario", {"scale":"千万级日活"}),
        ("智能座舱", "scenario", {"feature":"多屏互动/语音交互"}),
        ("智能安防摄像头", "scenario", {"scale":"2万路"}),
        ("工业传感器网络", "scenario", {"scale":"5000+节点"}),
        ("家庭智能中控", "scenario", {"feature":"语音/多设备联动"}),
        ("金融身份认证", "scenario", {"certification":"金融级"}),
    ]
    for name, etype, props in scenarios:
        entities.append((etype, name, json.dumps(props, ensure_ascii=False), name))

    # === 客户实体 (6) ===
    customers = [
        ("某头部云服务商", "customer", {"type":"云计算"}),
        ("某新能源车企", "customer", {"type":"汽车制造"}),
        ("某一线城市公安", "customer", {"type":"政府"}),
        ("某制造集团", "customer", {"type":"工业制造"}),
        ("某头部家电品牌", "customer", {"type":"消费电子"}),
        ("某国有银行", "customer", {"type":"金融"}),
    ]
    for name, etype, props in customers:
        entities.append((etype, name, json.dumps(props, ensure_ascii=False), name))

    # 插入实体
    conn = get_conn(BIZ_DB)
    entity_ids = {}
    for etype, name, props, desc in entities:
        cur = conn.execute("""INSERT OR IGNORE INTO kg_entities (app_key, entity_type, name, properties, description, created_at, updated_at)
            VALUES (?,?,?,?,?,?,?)""", (APP_KEY, etype, name, props, desc, ts, ts))
        if cur.lastrowid:
            entity_ids[f"{etype}:{name}"] = cur.lastrowid
        else:
            row = conn.execute("SELECT id FROM kg_entities WHERE app_key=? AND entity_type=? AND name=?", (APP_KEY, etype, name)).fetchone()
            entity_ids[f"{etype}:{name}"] = row["id"]
    conn.commit()
    conn.close()

    print(f"  ✓ 知识图谱实体: {len(entities)} 个")
    print(f"    产品:6 技术:8 行业:6 场景:6 客户:6")
    return entity_ids

def build_relations(entity_ids):
    """构建知识图谱关系"""
    ts = now_iso()
    relations = []

    def eid(etype, name):
        return entity_ids.get(f"{etype}:{name}")

    # === 产品-属于-产品线 (6) ===
    # 产品线作为隐含分类，用行业关联替代

    # === 产品-采用-技术 (12) ===
    relations += [
        (eid("product","XE-A2 智算芯片"), eid("technology","5nm先进制程"), "adopts", {}),
        (eid("product","XE-A2 智算芯片"), eid("technology","自研NPU张量引擎"), "adopts", {}),
        (eid("product","XE-N1 边缘AI芯片"), eid("technology","7nm先进制程"), "adopts", {}),
        (eid("product","XE-N1 边缘AI芯片"), eid("technology","自研NPU张量引擎"), "adopts", {}),
        (eid("product","XE-N1 边缘AI芯片"), eid("technology","多模态视频编解码"), "adopts", {}),
        (eid("product","XE-T1 智能终端SoC"), eid("technology","RISC-V CPU架构"), "adopts", {}),
        (eid("product","XE-T1 智能终端SoC"), eid("technology","低功耗设计"), "adopts", {}),
        (eid("product","XE-V1 车规级芯片"), eid("technology","车载功能安全"), "adopts", {}),
        (eid("product","XE-V1 车规级芯片"), eid("technology","RISC-V CPU架构"), "adopts", {}),
        (eid("product","XE-I1 物联网芯片"), eid("technology","低功耗设计"), "adopts", {}),
        (eid("product","XE-S1 安全芯片"), eid("technology","国密算法引擎"), "adopts", {}),
        (eid("product","XE-S1 安全芯片"), eid("technology","低功耗设计"), "adopts", {}),
    ]

    # === 产品-应用于-行业 (6) ===
    relations += [
        (eid("product","XE-A2 智算芯片"), eid("industry","AI云计算"), "applied_to", {}),
        (eid("product","XE-V1 车规级芯片"), eid("industry","智能驾驶"), "applied_to", {}),
        (eid("product","XE-N1 边缘AI芯片"), eid("industry","智慧城市"), "applied_to", {}),
        (eid("product","XE-I1 物联网芯片"), eid("industry","工业物联网"), "applied_to", {}),
        (eid("product","XE-T1 智能终端SoC"), eid("industry","智能家居"), "applied_to", {}),
        (eid("product","XE-S1 安全芯片"), eid("industry","金融科技"), "applied_to", {}),
    ]

    # === 产品-应用于-场景 (6) ===
    relations += [
        (eid("product","XE-A2 智算芯片"), eid("scenario","AI推理集群"), "used_in", {}),
        (eid("product","XE-V1 车规级芯片"), eid("scenario","智能座舱"), "used_in", {}),
        (eid("product","XE-N1 边缘AI芯片"), eid("scenario","智能安防摄像头"), "used_in", {}),
        (eid("product","XE-I1 物联网芯片"), eid("scenario","工业传感器网络"), "used_in", {}),
        (eid("product","XE-T1 智能终端SoC"), eid("scenario","家庭智能中控"), "used_in", {}),
        (eid("product","XE-S1 安全芯片"), eid("scenario","金融身份认证"), "used_in", {}),
    ]

    # === 客户-采用-产品 (6) ===
    relations += [
        (eid("customer","某头部云服务商"), eid("product","XE-A2 智算芯片"), "uses", {}),
        (eid("customer","某新能源车企"), eid("product","XE-V1 车规级芯片"), "uses", {}),
        (eid("customer","某一线城市公安"), eid("product","XE-N1 边缘AI芯片"), "uses", {}),
        (eid("customer","某制造集团"), eid("product","XE-I1 物联网芯片"), "uses", {}),
        (eid("customer","某头部家电品牌"), eid("product","XE-T1 智能终端SoC"), "uses", {}),
        (eid("customer","某国有银行"), eid("product","XE-S1 安全芯片"), "uses", {}),
    ]

    # === 客户-属于-行业 (6) ===
    relations += [
        (eid("customer","某头部云服务商"), eid("industry","AI云计算"), "belongs_to", {}),
        (eid("customer","某新能源车企"), eid("industry","智能驾驶"), "belongs_to", {}),
        (eid("customer","某一线城市公安"), eid("industry","智慧城市"), "belongs_to", {}),
        (eid("customer","某制造集团"), eid("industry","工业物联网"), "belongs_to", {}),
        (eid("customer","某头部家电品牌"), eid("industry","智能家居"), "belongs_to", {}),
        (eid("customer","某国有银行"), eid("industry","金融科技"), "belongs_to", {}),
    ]

    # === 场景-属于-行业 (6) ===
    relations += [
        (eid("scenario","AI推理集群"), eid("industry","AI云计算"), "belongs_to", {}),
        (eid("scenario","智能座舱"), eid("industry","智能驾驶"), "belongs_to", {}),
        (eid("scenario","智能安防摄像头"), eid("industry","智慧城市"), "belongs_to", {}),
        (eid("scenario","工业传感器网络"), eid("industry","工业物联网"), "belongs_to", {}),
        (eid("scenario","家庭智能中控"), eid("industry","智能家居"), "belongs_to", {}),
        (eid("scenario","金融身份认证"), eid("industry","金融科技"), "belongs_to", {}),
    ]

    # === 技术-基于-技术 (2) ===
    relations += [
        (eid("technology","5nm先进制程"), eid("technology","7nm先进制程"), "evolved_from", {}),
        (eid("technology","自研NPU张量引擎"), eid("technology","RISC-V CPU架构"), "integrates", {}),
    ]

    # 插入关系
    conn = get_conn(BIZ_DB)
    count = 0
    for src, tgt, rtype, props in relations:
        if src and tgt:
            conn.execute("""INSERT OR IGNORE INTO kg_relations (app_key, source_id, target_id, relation_type, properties, created_at)
                VALUES (?,?,?,?,?,?)""", (APP_KEY, src, tgt, rtype, json.dumps(props, ensure_ascii=False), ts))
            count += 1
    conn.commit()
    conn.close()
    print(f"  ✓ 知识图谱关系: {count} 条")
    print(f"    采用:12 应用于行业:6 应用于场景:6 客户使用:6 客户属于行业:6 场景属于行业:6 技术演进:2")

def create_kg_dsql():
    """创建知识图谱DSQL查询模板"""
    templates = [
        ("xinengine_kg_entities", "图谱实体列表",
         "SELECT id, entity_type, name, properties, description FROM kg_entities WHERE app_key='xinengine' ORDER BY entity_type, id",
         300, "知识图谱实体查询"),
        ("xinengine_kg_relations", "图谱关系列表",
         "SELECT r.id, r.source_id, s.name as source_name, s.entity_type as source_type, r.target_id, t.name as target_name, t.entity_type as target_type, r.relation_type, r.properties FROM kg_relations r JOIN kg_entities s ON r.source_id=s.id JOIN kg_entities t ON r.target_id=t.id WHERE r.app_key='xinengine'",
         300, "知识图谱关系查询"),
        ("xinengine_kg_neighbors", "图谱邻接查询",
         "SELECT r.id, r.source_id, s.name as source_name, r.target_id, t.name as target_name, t.entity_type as target_type, r.relation_type FROM kg_relations r JOIN kg_entities s ON r.source_id=s.id JOIN kg_entities t ON r.target_id=t.id WHERE r.app_key='xinengine' AND (r.source_id=:entity_id OR r.target_id=:entity_id)",
         60, "知识图谱单跳邻接查询"),
        ("xinengine_kg_by_type", "按类型查实体",
         "SELECT id, entity_type, name, properties, description FROM kg_entities WHERE app_key='xinengine' AND entity_type=:entity_type ORDER BY id",
         300, "按实体类型查询"),
        ("xinengine_kg_search", "图谱语义搜索",
         "SELECT id, entity_type, name, properties, description FROM kg_entities WHERE app_key='xinengine' AND (name LIKE :keyword OR description LIKE :keyword OR properties LIKE :keyword)",
         60, "知识图谱关键词搜索"),
    ]
    conn = get_conn(META_DB)
    ts = now_iso()
    count = 0
    for code, name, sql, ttl, desc in templates:
        conn.execute("""INSERT OR REPLACE INTO dsql_sqls (app_key, sql_code, sql_name, datasource, sql_template, version, cache_ttl, description, status, created_at, updated_at)
            VALUES ('xinengine',?,?, 'default', ?, 1, ?, ?, 'active', ?, ?)""", (code, name, sql, ttl, desc, ts, ts))
        count += 1
    conn.commit()
    conn.close()
    print(f"  ✓ 知识图谱DSQL模板: {count} 条")

def main():
    print("=" * 60)
    print("  芯擎科技 — 知识图谱构建")
    print("=" * 60)

    print("\n[1/4] 初始化图谱表结构...")
    init_kg_tables()

    print("\n[2/4] 构建图谱实体...")
    entity_ids = build_entities()

    print("\n[3/4] 构建图谱关系...")
    build_relations(entity_ids)

    print("\n[4/4] 创建DSQL查询模板...")
    create_kg_dsql()

    # 统计
    conn = get_conn(BIZ_DB)
    e_cnt = conn.execute("SELECT COUNT(*) FROM kg_entities WHERE app_key=?", (APP_KEY,)).fetchone()[0]
    r_cnt = conn.execute("SELECT COUNT(*) FROM kg_relations WHERE app_key=?", (APP_KEY,)).fetchone()[0]
    types = conn.execute("SELECT entity_type, COUNT(*) FROM kg_entities WHERE app_key=? GROUP BY entity_type", (APP_KEY,)).fetchall()
    rtypes = conn.execute("SELECT relation_type, COUNT(*) FROM kg_relations WHERE app_key=? GROUP BY relation_type", (APP_KEY,)).fetchall()
    conn.close()

    print("\n" + "=" * 60)
    print("  知识图谱构建完成!")
    print(f"  实体总数: {e_cnt}")
    for t in types:
        print(f"    {t['entity_type']}: {t[1]}")
    print(f"  关系总数: {r_cnt}")
    for t in rtypes:
        print(f"    {t['relation_type']}: {t[1]}")
    print(f"  前端页面: frontend-ui/chip-website/index.html#/graph")
    print("=" * 60)

if __name__ == "__main__":
    main()
