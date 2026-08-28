#!/usr/bin/env python3
"""
MOX 一键数据导出工具 — MXDEF v1.0 格式
适配真实双数据库架构: mox_meta.db + mox_business.db
"""
import sqlite3, json, hashlib, os, sys, argparse, gzip
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
FORMAT = "MXDEF"
VERSION = "1.0"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# === L1 内核表 (mox_meta.db, 跨系统通用) ===
META_KERNEL_TABLES = {
    "sql_templates": "dsql_sqls",
    "datasources": "datasources",
    "apps": "apps",
    "roles": "roles",
    "field_permissions": "field_permissions",
    "users": "users",
}
# === L2 知识图谱 (mox_meta.db) ===
META_KG_TABLES = {"entities": "kg_vertices", "relations": "kg_edges"}
# === L3 运行时 (mox_meta.db, 不导出) ===
META_RUNTIME_TABLES = {"ai_requests", "audit_logs", "publish_logs", "sqlite_sequence"}

# === L2 业务表 (mox_business.db) ===
BUSINESS_TABLES = {
    "products": "products", "news": "news", "cases": "cases",
    "team": "team", "banners": "banners", "messages": "messages",
}

SENSITIVE_REDACT = {"password", "secret", "token", "api_key", "private_key", "access_key"}
SENSITIVE_MASK = {"email", "phone", "mobile", "contact", "tel"}


def mask_value(field, value):
    if value is None:
        return None
    s = str(value)
    if field.lower() in SENSITIVE_REDACT:
        return "__REDACTED__"
    if field.lower() in SENSITIVE_MASK:
        if "@" in s and len(s) > 5:
            return s[0] + "***" + s[s.index("@"):]
        if len(s) >= 7:
            return s[:3] + "****" + s[-4:]
    return value


def export_table(cur, table, include_sensitive=False):
    try:
        cur.execute(f'SELECT * FROM "{table}"')
    except sqlite3.OperationalError:
        return []
    cols = [d[0] for d in cur.description]
    rows = cur.fetchall()
    result = []
    for row in rows:
        record = {}
        for i, col in enumerate(cols):
            val = row[i]
            if not include_sensitive:
                val = mask_value(col, val)
            record[col] = val
        result.append(record)
    return result


def main():
    p = argparse.ArgumentParser(description="MOX 一键数据导出工具 MXDEF v1.0")
    p.add_argument("--meta-db", default=os.path.join(ROOT, "platform/mox-server/mox_meta.db"), help="元数据库路径")
    p.add_argument("--business-db", default=os.path.join(ROOT, "platform/mox-server/mox_business.db"), help="业务数据库路径")
    p.add_argument("--app-key", default=None, help="导出指定应用(默认全部)")
    p.add_argument("--kernel-only", action="store_true", help="仅导出L1内核")
    p.add_argument("--business-only", action="store_true", help="仅导出L2业务数据")
    p.add_argument("--include-sensitive", action="store_true", help="包含敏感信息")
    p.add_argument("--split", action="store_true", help="分文件导出")
    p.add_argument("--gzip", action="store_true", help="gzip压缩")
    p.add_argument("--output", default=os.path.join(ROOT, "exports"), help="输出目录")
    args = p.parse_args()

    if not os.path.exists(args.meta_db):
        print(f"ERROR: 元数据库不存在: {args.meta_db}")
        sys.exit(1)

    t0 = datetime.now(CST)
    app_key = args.app_key or "all"

    export = {
        "format": FORMAT, "version": VERSION,
        "meta": {
            "exported_at": t0.isoformat(),
            "source_system": "mox-server", "source_version": "1.0.0",
            "app_key": app_key, "app_name": "",
            "database_architecture": "dual_db (meta + business)",
            "layers": {"kernel": not args.business_only, "business": not args.kernel_only, "runtime": False},
            "record_count": {"kernel": 0, "business": 0, "knowledge_graph": 0},
        },
        "kernel": {}, "business": {}, "knowledge_graph": {},
    }

    # === 连接元数据库 ===
    meta_conn = sqlite3.connect(args.meta_db)
    meta_cur = meta_conn.cursor()

    # L1 内核
    if not args.business_only:
        print("[L1] 导出内核配置 (mox_meta.db)...")
        for key, table in META_KERNEL_TABLES.items():
            data = export_table(meta_cur, table, args.include_sensitive)
            export["kernel"][key] = data
            export["meta"]["record_count"]["kernel"] += len(data)
            if key == "apps":
                for a in data:
                    if app_key == "all" or a.get("app_key") == app_key:
                        export["meta"]["app_name"] = a.get("name", a.get("app_name", ""))
                        break
            print(f"  {key:20s} {len(data):4d} 条")

        # L2 知识图谱
        if not args.kernel_only:
            print("[L2] 导出知识图谱 (mox_meta.db)...")
            for key, table in META_KG_TABLES.items():
                data = export_table(meta_cur, table, args.include_sensitive)
                export["knowledge_graph"][key] = data
                export["meta"]["record_count"]["knowledge_graph"] += len(data)
                print(f"  {key:20s} {len(data):4d} 条")

    meta_conn.close()

    # === 连接业务数据库 ===
    if not args.kernel_only and os.path.exists(args.business_db):
        print("[L2] 导出业务数据 (mox_business.db)...")
        biz_conn = sqlite3.connect(args.business_db)
        biz_cur = biz_conn.cursor()
        for key, table in BUSINESS_TABLES.items():
            data = export_table(biz_cur, table, args.include_sensitive)
            export["business"][key] = data
            export["meta"]["record_count"]["business"] += len(data)
            print(f"  {key:20s} {len(data):4d} 条")
        biz_conn.close()
    elif not args.kernel_only:
        print(f"  WARN: 业务数据库不存在: {args.business_db}, 跳过业务数据")

    # checksum
    payload = json.dumps({k: v for k, v in export.items() if k != "checksum"}, ensure_ascii=False, sort_keys=True)
    export["checksum"] = "sha256:" + hashlib.sha256(payload.encode("utf-8")).hexdigest()

    # 输出
    os.makedirs(args.output, exist_ok=True)
    ts = t0.strftime("%Y%m%d-%H%M%S")

    if args.split:
        out_path = os.path.join(args.output, f"mox-export-{app_key}-{ts}")
        os.makedirs(out_path, exist_ok=True)
        with open(os.path.join(out_path, "meta.json"), "w", encoding="utf-8") as f:
            json.dump(export["meta"], f, ensure_ascii=False, indent=2)
        for section in ["kernel", "business", "knowledge_graph"]:
            secdir = os.path.join(out_path, section)
            os.makedirs(secdir, exist_ok=True)
            for key, data in export[section].items():
                with open(os.path.join(secdir, f"{key}.json"), "w", encoding="utf-8") as f:
                    json.dump(data, f, ensure_ascii=False, indent=2)
    else:
        filename = f"mox-export-{app_key}-{ts}.json"
        out_path = os.path.join(args.output, filename)
        content = json.dumps(export, ensure_ascii=False, indent=2)
        if args.gzip:
            out_path += ".gz"
            with gzip.open(out_path, "wt", encoding="utf-8") as f:
                f.write(content)
        else:
            with open(out_path, "w", encoding="utf-8") as f:
                f.write(content)

    def get_size(path):
        if os.path.isfile(path):
            return os.path.getsize(path)
        return sum(os.path.getsize(os.path.join(dp, f)) for dp, _, fn in os.walk(path) for f in fn)

    size = get_size(out_path)
    elapsed = (datetime.now(CST) - t0).total_seconds()
    rc = export["meta"]["record_count"]
    print(f"\n{'='*55}")
    print(f"  导出完成: {out_path}")
    print(f"  文件大小: {size/1024:.1f} KB")
    print(f"  架构:     双数据库 (meta + business)")
    print(f"  L1 内核:  {rc['kernel']} 条")
    print(f"  L2 业务:  {rc['business']} 条")
    print(f"  L2 图谱:  {rc['knowledge_graph']} 条")
    print(f"  总记录:   {sum(rc.values())} 条")
    print(f"  耗时:     {elapsed:.2f}s")
    print(f"  Checksum: {export['checksum'][:32]}...")
    print(f"{'='*55}")


if __name__ == "__main__":
    main()
