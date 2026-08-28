#!/usr/bin/env python3
"""检查MOX后端配置和数据结构"""
import sqlite3, json, os

META = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "platform", "mox-server", "mox_meta.db")
BIZ = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "platform", "mox-server", "mox_business.db")

def rows_to_dicts(rows):
    return [dict(r) for r in rows]

c = sqlite3.connect(META)
c.row_factory = sqlite3.Row

print("=== datasources ===")
for r in c.execute("SELECT * FROM datasources").fetchall():
    print(json.dumps(dict(r), ensure_ascii=False, indent=2))

print("\n=== apps ===")
for r in c.execute("SELECT * FROM apps").fetchall():
    d = dict(r)
    print(f"  {d.get('app_key')}: {d.get('name')} type={d.get('type')} status={d.get('status')}")

print("\n=== dsql_sqls schema ===")
for r in c.execute("PRAGMA table_info(dsql_sqls)").fetchall():
    print(f"  {r[1]} ({r[2]})")

print("\n=== dsql_sqls data (first 5) ===")
for r in c.execute("SELECT code, app_key, name, status, version, datasource FROM dsql_sqls ORDER BY updated_at DESC LIMIT 10").fetchall():
    print(f"  {r['code']}: {r['name']} status={r['status']} v{r['version']} ds={r['datasource']}")

print("\n=== kg_vertices schema ===")
for r in c.execute("PRAGMA table_info(kg_vertices)").fetchall():
    print(f"  {r[1]} ({r[2]})")

print("\n=== kg_vertices count by type ===")
for r in c.execute("SELECT type, COUNT(*) as c FROM kg_vertices GROUP BY type").fetchall():
    print(f"  {r['type']}: {r['c']}")

print("\n=== kg_edges count by relation ===")
for r in c.execute("SELECT relation, COUNT(*) as c FROM kg_edges GROUP BY relation").fetchall():
    print(f"  {r['relation']}: {r['c']}")

c.close()

# 业务库
c2 = sqlite3.connect(BIZ)
c2.row_factory = sqlite3.Row
print("\n=== mox_business tables ===")
for r in c2.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").fetchall():
    cnt = c2.execute(f"SELECT COUNT(*) FROM [{r['name']}]").fetchone()[0]
    print(f"  {r['name']}: {cnt} rows")
c2.close()
