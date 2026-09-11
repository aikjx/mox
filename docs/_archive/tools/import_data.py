#!/usr/bin/env python3
"""
MOX 一键数据导入工具 — MXDEF v1.0 格式
适配真实双数据库架构: mox_meta.db + mox_business.db
幂等upsert / 分层导入 / dry-run预览 / checksum校验
"""
import sqlite3, json, os, sys, argparse, hashlib, gzip
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

META_KERNEL_TABLES = {
    "sql_templates": "dsql_sqls", "datasources": "datasources",
    "apps": "apps", "roles": "roles", "field_permissions": "field_permissions",
    "users": "users",
}
META_KG_TABLES = {"entities": "kg_vertices", "relations": "kg_edges"}
BUSINESS_TABLES = {
    "products": "products", "news": "news", "cases": "cases",
    "team": "team", "banners": "banners", "messages": "messages",
}
ALL_META_TABLES = {**META_KERNEL_TABLES, **META_KG_TABLES}


def verify_checksum(data):
    if "checksum" not in data:
        return True, "无checksum(跳过)"
    expected = data["checksum"]
    payload = {k: v for k, v in data.items() if k != "checksum"}
    actual = "sha256:" + hashlib.sha256(json.dumps(payload, ensure_ascii=False, sort_keys=True).encode()).hexdigest()
    return (actual == expected, "通过" if actual == expected else f"不匹配")


def table_cols(cur, table):
    cur.execute(f'PRAGMA table_info("{table}")')
    return [r[1] for r in cur.fetchall()]


def upsert(cur, table, record, unique_key="id"):
    cols = table_cols(cur, table)
    if not cols:
        return False
    valid = {k: v for k, v in record.items() if k in cols}
    if not valid:
        return False
    keys = list(valid.keys())
    placeholders = ", ".join(["?"] * len(keys))
    col_names = ", ".join(keys)
    if unique_key in keys:
        update = ", ".join([f"{k}=excluded.{k}" for k in keys if k != unique_key])
        sql = f'INSERT INTO "{table}" ({col_names}) VALUES ({placeholders}) ON CONFLICT({unique_key}) DO UPDATE SET {update}'
    else:
        sql = f'INSERT OR IGNORE INTO "{table}" ({col_names}) VALUES ({placeholders})'
    try:
        cur.execute(sql, [valid[k] for k in keys])
        return True
    except Exception as e:
        print(f"    ERROR {table}: {e}")
        return False


def import_section(cur, section, table_map, dry_run=False):
    stats = {}
    for key, table in table_map.items():
        records = section.get(key, [])
        if not records:
            stats[key] = 0
            continue
        ok = 0
        for rec in records:
            if dry_run:
                ok += 1
            elif upsert(cur, table, rec):
                ok += 1
        stats[key] = ok
        print(f"    {key:20s} {ok:4d}/{len(records):4d}")
    return stats


def load_export(filepath):
    if filepath.endswith(".gz"):
        with gzip.open(filepath, "rt", encoding="utf-8") as f:
            return json.load(f)
    with open(filepath, "r", encoding="utf-8") as f:
        return json.load(f)


def main():
    p = argparse.ArgumentParser(description="MOX 一键数据导入工具 MXDEF v1.0")
    p.add_argument("input", help="MXDEF导出文件(.json或.json.gz)")
    p.add_argument("--meta-db", default=os.path.join(ROOT, "platform/mox-server/mox_meta.db"), help="目标元数据库")
    p.add_argument("--business-db", default=os.path.join(ROOT, "platform/mox-server/mox_business.db"), help="目标业务数据库")
    p.add_argument("--kernel-only", action="store_true")
    p.add_argument("--business-only", action="store_true")
    p.add_argument("--dry-run", action="store_true", help="预览模式不写入")
    p.add_argument("--force", action="store_true", help="忽略checksum校验失败")
    p.add_argument("--purge", action="store_true", help="导入前清空目标表(危险)")
    args = p.parse_args()

    if not os.path.exists(args.input):
        print(f"ERROR: 文件不存在: {args.input}")
        sys.exit(1)

    data = load_export(args.input)
    if data.get("format") != "MXDEF":
        print(f"ERROR: 非MXDEF格式 (format={data.get('format')})")
        sys.exit(1)

    ok, msg = verify_checksum(data)
    print(f"[校验] checksum: {msg}")
    if not ok and not args.force:
        print("ERROR: checksum失败, 使用--force强制导入")
        sys.exit(1)

    meta = data.get("meta", {})
    rc = meta.get("record_count", {})
    print(f"\n{'='*55}")
    print(f"  源系统:   {meta.get('source_system')} v{meta.get('source_version')}")
    print(f"  源应用:   {meta.get('app_key')} ({meta.get('app_name')})")
    print(f"  架构:     {meta.get('database_architecture', 'dual_db')}")
    print(f"  导出时间: {meta.get('exported_at')}")
    print(f"  模式:     {'预览DRY-RUN' if args.dry_run else '正式导入'}")
    print(f"  内核记录: {rc.get('kernel', 0)}")
    print(f"  业务记录: {rc.get('business', 0)}")
    print(f"  图谱记录: {rc.get('knowledge_graph', 0)}")
    print(f"{'='*55}\n")

    os.makedirs(os.path.dirname(args.meta_db), exist_ok=True)
    meta_conn = sqlite3.connect(args.meta_db)
    meta_cur = meta_conn.cursor()
    total = {"kernel": 0, "business": 0, "knowledge_graph": 0}

    if args.purge and not args.dry_run:
        print("[警告] 清空元数据库目标表...")
        for table in ALL_META_TABLES.values():
            try:
                meta_cur.execute(f'DELETE FROM "{table}"')
            except Exception:
                pass

    if not args.business_only:
        print("[L1] 导入内核 (mox_meta.db)...")
        s = import_section(meta_cur, data.get("kernel", {}), META_KERNEL_TABLES, args.dry_run)
        total["kernel"] = sum(s.values())

    if not args.kernel_only:
        print("[L2] 导入知识图谱 (mox_meta.db)...")
        s = import_section(meta_cur, data.get("knowledge_graph", {}), META_KG_TABLES, args.dry_run)
        total["knowledge_graph"] = sum(s.values())

    if args.dry_run:
        meta_conn.rollback()
    else:
        meta_conn.commit()
    meta_conn.close()

    # 业务数据库
    if not args.kernel_only:
        biz_data = data.get("business", {})
        if biz_data:
            print("[L2] 导入业务数据 (mox_business.db)...")
            os.makedirs(os.path.dirname(args.business_db), exist_ok=True)
            biz_conn = sqlite3.connect(args.business_db)
            biz_cur = biz_conn.cursor()
            if args.purge and not args.dry_run:
                for table in BUSINESS_TABLES.values():
                    try:
                        biz_cur.execute(f'DELETE FROM "{table}"')
                    except Exception:
                        pass
            s = import_section(biz_cur, biz_data, BUSINESS_TABLES, args.dry_run)
            total["business"] = sum(s.values())
            if args.dry_run:
                biz_conn.rollback()
            else:
                biz_conn.commit()
            biz_conn.close()

    print(f"\n{'='*55}")
    if args.dry_run:
        print(f"  预览完成 (未写入数据库)")
    else:
        print(f"  导入完成")
    print(f"  L1 内核: {total['kernel']} 条")
    print(f"  L2 业务: {total['business']} 条")
    print(f"  L2 图谱: {total['knowledge_graph']} 条")
    print(f"  总计:    {sum(total.values())} 条")
    print(f"  元数据库: {args.meta_db}")
    print(f"  业务数据库: {args.business_db}")
    print(f"{'='*55}")


if __name__ == "__main__":
    main()
