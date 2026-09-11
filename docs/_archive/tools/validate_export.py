#!/usr/bin/env python3
"""MOX 导出文件校验工具 — MXDEF v1.0 (双数据库架构)"""
import json, sys, hashlib, os, gzip


def load(fp):
    if fp.endswith(".gz"):
        with gzip.open(fp, "rt", encoding="utf-8") as f:
            return json.load(f)
    with open(fp, "r", encoding="utf-8") as f:
        return json.load(f)


def validate(filepath):
    data = load(filepath)
    errors, warnings = [], []

    if data.get("format") != "MXDEF":
        errors.append(f"格式错误: 期望MXDEF, 实际{data.get('format')}")
    ver = data.get("version", "")
    if not ver.startswith("1."):
        warnings.append(f"版本{ver}可能不兼容")

    if "checksum" in data:
        payload = {k: v for k, v in data.items() if k != "checksum"}
        actual = "sha256:" + hashlib.sha256(json.dumps(payload, ensure_ascii=False, sort_keys=True).encode()).hexdigest()
        if actual == data["checksum"]:
            print("  [PASS] checksum校验通过")
        else:
            errors.append("checksum不匹配(文件可能被篡改)")

    meta = data.get("meta", {})
    for sec in ["kernel", "business", "knowledge_graph"]:
        declared = meta.get("record_count", {}).get(sec, 0)
        actual = sum(len(v) for v in data.get(sec, {}).values())
        if declared != actual:
            warnings.append(f"{sec}记录数不一致: meta声明{declared}, 实际{actual}")

    sensitive = {"password", "secret", "token", "api_key", "private_key"}
    for sec in ["kernel", "business"]:
        for key, records in data.get(sec, {}).items():
            for rec in records:
                for f in sensitive:
                    if f in rec and rec[f] not in (None, "", "__REDACTED__"):
                        warnings.append(f"{sec}.{key}含未脱敏敏感字段: {f}")

    # 知识图谱外键检查
    entities = data.get("knowledge_graph", {}).get("entities", [])
    eids = {e.get("vid") or e.get("id") for e in entities}
    for rel in data.get("knowledge_graph", {}).get("relations", []):
        src = rel.get("source") or rel.get("source_id")
        tgt = rel.get("target") or rel.get("target_id")
        if src and src not in eids:
            warnings.append(f"图谱关系{rel.get('id')}的source引用不存在实体")
        if tgt and tgt not in eids:
            warnings.append(f"图谱关系{rel.get('id')}的target引用不存在实体")

    # 重复ID检查
    for sec in ["kernel", "business", "knowledge_graph"]:
        for key, records in data.get(sec, {}).items():
            ids = [r.get("id") or r.get("vid") for r in records if r.get("id") or r.get("vid")]
            if len(ids) != len(set(ids)):
                warnings.append(f"{sec}.{key}存在重复id")

    print(f"\n{'='*55}")
    print(f"  文件: {filepath}")
    print(f"  大小: {os.path.getsize(filepath)/1024:.1f} KB")
    print(f"  格式: {data.get('format')} v{data.get('version')}")
    print(f"  架构: {meta.get('database_architecture', 'unknown')}")
    print(f"  应用: {meta.get('app_key')} ({meta.get('app_name')})")
    rc = meta.get("record_count", {})
    print(f"  内核: {rc.get('kernel', 0)} | 业务: {rc.get('business', 0)} | 图谱: {rc.get('knowledge_graph', 0)}")
    print(f"  错误: {len(errors)}")
    for e in errors:
        print(f"    [FAIL] {e}")
    print(f"  警告: {len(warnings)}")
    for w in warnings[:10]:
        print(f"    [WARN] {w}")
    if len(warnings) > 10:
        print(f"    ... 还有{len(warnings)-10}条警告")
    print(f"  结果: {'通过' if not errors else '失败'}")
    print(f"{'='*55}")
    return len(errors) == 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("用法: python tools/validate_export.py <export.json>")
        sys.exit(1)
    sys.exit(0 if validate(sys.argv[1]) else 1)
