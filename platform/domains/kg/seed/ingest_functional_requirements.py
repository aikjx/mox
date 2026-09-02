#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
功能需求知识图谱批量录入脚本
读取 functional-requirements-graph-seed.json，幂等录入到 KG API
用法:
  python ingest_functional_requirements.py           # 实际录入
  python ingest_functional_requirements.py --dry-run # 仅统计不录入
"""
import json
import sys
import time
import argparse
import urllib.request
import urllib.error
from pathlib import Path

# ========== 配置 ==========
BASE_URL = "http://localhost:8080"
SEED_PATH = Path(__file__).parent / "functional-requirements-graph-seed.json"
NODE_DELAY = 0.05  # 节点录入间隔(秒)
EDGE_DELAY = 0.03  # 边录入间隔(秒)
TIMEOUT = 30


def http_get(path):
    """GET 请求"""
    url = f"{BASE_URL}{path}"
    req = urllib.request.Request(url, method="GET")
    req.add_header("Accept", "application/json")
    with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
        data = resp.read().decode("utf-8")
        return json.loads(data)


def http_post(path, body):
    """POST 请求，body 为 dict"""
    url = f"{BASE_URL}{path}"
    data = json.dumps(body, ensure_ascii=False).encode("utf-8")
    req = urllib.request.Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Accept", "application/json")
    with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
        resp_data = resp.read().decode("utf-8")
        return json.loads(resp_data)


def get_existing_nodes():
    """获取现有图谱全部节点 {id: node}（用于幂等/差异检查）"""
    try:
        result = http_get("/api/graph")
        nodes = result.get("data", {}).get("nodes", [])
        return {n["id"]: n for n in nodes if "id" in n}
    except Exception as e:
        print(f"[WARN] 获取现有节点失败: {e}")
        return {}


def get_existing_edges():
    """获取现有图谱中所有边（用于幂等检查）"""
    try:
        result = http_get("/api/graph")
        edges = result.get("data", {}).get("edges", [])
        existing = set()
        for e in edges:
            s = e.get("source", "")
            t = e.get("target", "")
            r = e.get("relation_type", e.get("relation", ""))
            existing.add(f"{s}->{t}:{r}")
        return existing
    except Exception as e:
        print(f"[WARN] 获取现有边失败: {e}")
        return set()


def ingest_nodes(nodes, dry_run=False):
    """批量录入/更新节点（幂等 upsert：已存在但 properties 有差异则更新）"""
    existing = get_existing_nodes()
    print(f"[INFO] 现有节点数: {len(existing)}")

    added = 0
    updated = 0
    skipped = 0
    failed = 0
    total = len(nodes)

    for i, node in enumerate(nodes):
        nid = node["id"]
        body = {
            "id": node["id"],
            "label": node.get("label", node["id"]),
            "node_type": node["node_type"],
            "properties": node.get("properties", {}),
        }
        if nid in existing:
            old_props = existing[nid].get("properties") or {}
            if old_props == body["properties"]:
                skipped += 1
            else:
                # 差异更新（seed 为准，覆盖旧 properties）
                if dry_run:
                    updated += 1
                else:
                    try:
                        resp = http_post("/api/graph/node", body)
                        if resp.get("success"):
                            updated += 1
                            existing[nid]["properties"] = body["properties"]
                        else:
                            failed += 1
                            print(f"  [FAIL] 更新节点 {nid}: {resp}")
                    except Exception as e:
                        failed += 1
                        print(f"  [ERROR] 更新节点 {nid}: {e}")
        else:
            if dry_run:
                added += 1
                continue
            try:
                resp = http_post("/api/graph/node", body)
                if resp.get("success"):
                    added += 1
                    existing[nid] = body
                else:
                    failed += 1
                    print(f"  [FAIL] 节点 {nid}: {resp}")
            except Exception as e:
                failed += 1
                print(f"  [ERROR] 节点 {nid}: {e}")

        if (i + 1) % 100 == 0:
            print(f"  进度: {i+1}/{total} (新增={added}, 更新={updated}, 跳过={skipped}, 失败={failed})")

        time.sleep(NODE_DELAY)

    print(f"[节点录入完成] 总计={total}, 新增={added}, 更新={updated}, 跳过={skipped}, 失败={failed}")
    return added, skipped, failed, updated


def ingest_edges(edges, dry_run=False):
    """批量录入边"""
    existing_edges = get_existing_edges()
    print(f"[INFO] 现有边数: {len(existing_edges)}")

    added = 0
    skipped = 0
    failed = 0
    total = len(edges)

    for i, edge in enumerate(edges):
        s = edge["source"]
        t = edge["target"]
        r = edge["relation_type"]
        key = f"{s}->{t}:{r}"

        if key in existing_edges:
            skipped += 1
            if (i + 1) % 100 == 0:
                print(f"  进度: {i+1}/{total} (新增={added}, 跳过={skipped}, 失败={failed})")
            continue

        if dry_run:
            added += 1
            continue

        body = {
            "source": s,
            "target": t,
            "weight": edge.get("weight", 0.5),
            "relation_type": r,
        }
        try:
            resp = http_post("/api/graph/edge", body)
            if resp.get("success"):
                added += 1
                existing_edges.add(key)
            else:
                failed += 1
                print(f"  [FAIL] 边 {key}: {resp}")
        except Exception as e:
            failed += 1
            print(f"  [ERROR] 边 {key}: {e}")

        if (i + 1) % 100 == 0:
            print(f"  进度: {i+1}/{total} (新增={added}, 跳过={skipped}, 失败={failed})")

        time.sleep(EDGE_DELAY)

    print(f"[边录入完成] 总计={total}, 新增={added}, 跳过={skipped}, 失败={failed}")
    return added, skipped, failed


def main():
    parser = argparse.ArgumentParser(description="功能需求 KG 批量录入")
    parser.add_argument("--dry-run", action="store_true", help="仅统计不实际录入")
    parser.add_argument("--seed", type=str, default=str(SEED_PATH), help="种子数据文件路径")
    args = parser.parse_args()

    seed_path = Path(args.seed)
    if not seed_path.exists():
        print(f"[ERROR] 种子文件不存在: {seed_path}")
        sys.exit(1)

    print(f"{'='*60}")
    print(f"  功能需求知识图谱录入 {'(DRY-RUN)' if args.dry_run else ''}")
    print(f"  种子文件: {seed_path}")
    print(f"  KG API:   {BASE_URL}")
    print(f"{'='*60}")

    # 读取种子数据
    with open(seed_path, "r", encoding="utf-8") as f:
        seed = json.load(f)

    nodes = seed.get("nodes", [])
    edges = seed.get("edges", [])

    # 按类型统计
    type_counts = {}
    for n in nodes:
        nt = n["node_type"]
        type_counts[nt] = type_counts.get(nt, 0) + 1

    rel_counts = {}
    for e in edges:
        rt = e["relation_type"]
        rel_counts[rt] = rel_counts.get(rt, 0) + 1

    print(f"\n[种子数据统计]")
    print(f"  节点总数: {len(nodes)}")
    for nt, cnt in sorted(type_counts.items()):
        print(f"    {nt}: {cnt}")
    print(f"  边总数: {len(edges)}")
    for rt, cnt in sorted(rel_counts.items()):
        print(f"    {rt}: {cnt}")

    # 测试 API 连通性
    if not args.dry_run:
        try:
            stats = http_get("/api/graph/stats")
            print(f"\n[API 连通性] OK, 当前图谱: {stats.get('data', {})}")
        except Exception as e:
            print(f"\n[ERROR] KG API 不可达: {e}")
            sys.exit(1)

    # 录入节点
    print(f"\n{'─'*40}")
    print(f"[1/2] 录入节点...")
    n_added, n_skipped, n_failed, n_updated = ingest_nodes(nodes, dry_run=args.dry_run)

    # 录入边
    print(f"\n{'─'*40}")
    print(f"[2/2] 录入边...")
    e_added, e_skipped, e_failed = ingest_edges(edges, dry_run=args.dry_run)

    # 最终统计
    print(f"\n{'='*60}")
    print(f"  录入完成 {'(DRY-RUN)' if args.dry_run else ''}")
    print(f"{'='*60}")
    print(f"  节点: 种子={len(nodes)}, 新增={n_added}, 更新={n_updated}, 跳过={n_skipped}, 失败={n_failed}")
    print(f"  边:   种子={len(edges)}, 新增={e_added}, 跳过={e_skipped}, 失败={e_failed}")

    if not args.dry_run:
        try:
            final_stats = http_get("/api/graph/stats")
            d = final_stats.get("data", {})
            print(f"\n  图谱最终统计: nodes={d.get('nodes')}, edges={d.get('edges')}, "
                  f"density={d.get('density')}, components={d.get('components')}")
        except Exception as e:
            print(f"\n  [WARN] 获取最终统计失败: {e}")

    if n_failed > 0 or e_failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
