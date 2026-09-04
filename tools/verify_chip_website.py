#!/usr/bin/env python3
"""验证芯片公司官网的知识图谱和AI完成度"""
import sqlite3, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIZ_DB = os.path.join(ROOT, "platform", "mox-server", "mox_business.db")
META_DB = os.path.join(ROOT, "platform", "mox-server", "mox_meta.db")

print("=" * 60)
print("  芯擎科技官网 — mox 模块化系统架构完成度验证")
print("=" * 60)

# 1. 业务数据
print("\n【1】业务数据")
c = sqlite3.connect(BIZ_DB)
for table in ["products", "news", "cases", "team", "messages"]:
    try:
        cnt = c.execute(f"SELECT COUNT(*) FROM {table} WHERE app_key=?", ("xinengine",)).fetchone()[0]
        print(f"  ✓ {table}: {cnt} 条")
    except Exception as e:
        print(f"  ✗ {table}: {e}")

# 2. 知识图谱
print("\n【2】知识图谱")
kg_tables = []
for row in c.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall():
    name = row[0].lower()
    if any(k in name for k in ["kg", "graph", "entit", "relat", "vertex", "edge"]):
        kg_tables.append(row[0])

if kg_tables:
    print(f"  找到KG表: {kg_tables}")
    for t in kg_tables:
        try:
            cnt = c.execute(f"SELECT COUNT(*) FROM {t} WHERE app_key=?", ("xinengine",)).fetchone()[0]
            print(f"    {t} (xinengine): {cnt}")
        except:
            try:
                cnt = c.execute(f"SELECT COUNT(*) FROM {t}").fetchone()[0]
                print(f"    {t} (total): {cnt}")
            except Exception as e:
                print(f"    {t}: 查询失败 {e}")
else:
    print("  ✗ 未找到知识图谱相关表")

c.close()

# 3. DSQL模板
print("\n【3】DSQL动态SQL模板")
c2 = sqlite3.connect(META_DB)
try:
    rows = c2.execute("SELECT sql_code, sql_name FROM dsql_sqls WHERE app_key=?", ("xinengine",)).fetchall()
    print(f"  ✓ {len(rows)} 条模板:")
    for r in rows:
        print(f"    - {r[0]}: {r[1]}")
except Exception as e:
    print(f"  ✗ 查询失败: {e}")

# 4. AI相关
print("\n【4】AI助手配置")
ai_tables = []
for row in c2.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall():
    name = row[0].lower()
    if any(k in name for k in ["ai", "chat", "assistant", "prompt", "conversation"]):
        ai_tables.append(row[0])
if ai_tables:
    print(f"  找到AI表: {ai_tables}")
else:
    print("  - 未找到独立AI配置表（AI能力可能内置在服务代码中）")

# 5. 应用注册
print("\n【5】应用注册")
try:
    app = c2.execute("SELECT app_key, app_name, status FROM dsql_apps WHERE app_key=?", ("xinengine",)).fetchone()
    if app:
        print(f"  ✓ {app[1]} ({app[0]}) 状态: {app[2]}")
    else:
        print("  ✗ 未找到应用注册")
except Exception as e:
    print(f"  ✗ 查询失败: {e}")

c2.close()

# 6. 前端文件
print("\n【6】前端文件")
frontend = os.path.join(ROOT, "frontend-ui", "chip-website", "index.html")
if os.path.exists(frontend):
    size = os.path.getsize(frontend)
    with open(frontend, "r", encoding="utf-8") as f:
        lines = len(f.readlines())
    print(f"  ✓ index.html: {size/1024:.1f} KB / {lines} 行")
else:
    print("  ✗ 前端文件不存在")

# 7. 初始化脚本
print("\n【7】初始化脚本")
init_script = os.path.join(ROOT, "tools", "init_chip_website.py")
if os.path.exists(init_script):
    print(f"  ✓ init_chip_website.py 存在")
else:
    print("  ✗ 初始化脚本不存在")

print("\n" + "=" * 60)
print("  验证完成")
print("=" * 60)
