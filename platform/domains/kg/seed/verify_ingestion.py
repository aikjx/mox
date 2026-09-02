#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
功能需求 KG 录入验证脚本
执行所有验证项并输出量化报告
"""
import json
import urllib.request
from collections import Counter, defaultdict

BASE_URL = "http://localhost:8080"
SEED_PATH = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\seed\functional-requirements-graph-seed.json"


def http_get(path):
    url = f"{BASE_URL}{path}"
    req = urllib.request.Request(url, method="GET")
    req.add_header("Accept", "application/json")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def main():
    print("=" * 70)
    print("  功能需求知识图谱录入验证报告")
    print("=" * 70)

    # 读取种子数据（预期值）
    with open(SEED_PATH, "r", encoding="utf-8") as f:
        seed = json.load(f)
    seed_nodes = seed["nodes"]
    seed_edges = seed["edges"]
    seed_node_ids = {n["id"] for n in seed_nodes}

    # ========== a. 回读验证 ==========
    print("\n【a. 回读验证】GET /api/graph")
    graph = http_get("/api/graph")
    graph_nodes = graph["data"]["nodes"]
    graph_edges = graph["data"]["edges"]
    print(f"  回读节点数: {len(graph_nodes)} (预期含系统架构15 + 种子{len(seed_nodes)} = {15 + len(seed_nodes)})")
    print(f"  回读边数:   {len(graph_edges)} (预期含系统架构17 + 种子{len(seed_edges)} = {17 + len(seed_edges)})")

    # 检查种子节点是否全部存在
    graph_node_ids = {n["id"] for n in graph_nodes}
    missing_nodes = seed_node_ids - graph_node_ids
    print(f"  种子节点缺失: {len(missing_nodes)} {'✓' if not missing_nodes else '✗ ' + str(missing_nodes)}")

    # 检查种子边是否全部存在
    graph_edge_keys = set()
    for e in graph_edges:
        s = e.get("source", "")
        t = e.get("target", "")
        r = e.get("relation_type", e.get("relation", ""))
        graph_edge_keys.add(f"{s}->{t}:{r}")
    seed_edge_keys = set()
    for e in seed_edges:
        seed_edge_keys.add(f"{e['source']}->{e['target']}:{e['relation_type']}")
    missing_edges = seed_edge_keys - graph_edge_keys
    print(f"  种子边缺失:   {len(missing_edges)} {'✓' if not missing_edges else '✗ ' + str(missing_edges)}")

    # ========== b. 按 node_type 统计 ==========
    print("\n【b. 按 node_type 统计】")
    seed_type_counts = Counter(n["node_type"] for n in seed_nodes)
    graph_type_counts = Counter(n["node_type"] for n in graph_nodes)
    print(f"  {'node_type':<20} {'种子(预期)':>12} {'回读(实际)':>12} {'匹配':>6}")
    print(f"  {'-'*20} {'-'*12} {'-'*12} {'-'*6}")
    all_match = True
    for nt in sorted(set(list(seed_type_counts.keys()) + list(graph_type_counts.keys()))):
        s_cnt = seed_type_counts.get(nt, 0)
        g_cnt = graph_type_counts.get(nt, 0)
        # 回读包含系统架构节点(可能有其他type)，所以只检查种子类型的数量
        match = "✓" if (nt in seed_type_counts and g_cnt >= s_cnt) else ("✗" if nt in seed_type_counts else "-")
        if nt in seed_type_counts and g_cnt < s_cnt:
            all_match = False
        print(f"  {nt:<20} {s_cnt:>12} {g_cnt:>12} {match:>6}")
    print(f"  系统架构节点(其他类型): {graph_type_counts.get('system', 0) + graph_type_counts.get('component', 0) + sum(v for k,v in graph_type_counts.items() if k not in seed_type_counts)}")

    # ========== c. 按域统计 feature ==========
    print("\n【c. 按域统计 feature 节点】")
    seed_feature_by_domain = Counter(
        n["properties"].get("domain", "unknown")
        for n in seed_nodes if n["node_type"] == "feature"
    )
    graph_feature_by_domain = Counter(
        n["properties"].get("domain", "unknown")
        for n in graph_nodes if n["node_type"] == "feature"
    )
    print(f"  {'domain':<15} {'种子(预期)':>12} {'回读(实际)':>12} {'匹配':>6}")
    print(f"  {'-'*15} {'-'*12} {'-'*12} {'-'*6}")
    for dom in sorted(seed_feature_by_domain.keys()):
        s_cnt = seed_feature_by_domain[dom]
        g_cnt = graph_feature_by_domain.get(dom, 0)
        match = "✓" if g_cnt >= s_cnt else "✗"
        print(f"  {dom:<15} {s_cnt:>12} {g_cnt:>12} {match:>6}")

    # ========== d. 缺口验证 ==========
    print("\n【d. 缺口与待决策验证】")
    gap_count = graph_type_counts.get("gap", 0)
    dec_count = graph_type_counts.get("pending_decision", 0)
    print(f"  gap 节点数:          {gap_count} (预期 20) {'✓' if gap_count == 20 else '✗'}")
    print(f"  pending_decision 数: {dec_count} (预期 6)  {'✓' if dec_count == 6 else '✗'}")

    # 列出 gap 节点
    gap_nodes = [n for n in graph_nodes if n["node_type"] == "gap"]
    print(f"  Gap 列表 (id / severity / status / affected_count):")
    for g in sorted(gap_nodes, key=lambda x: x["id"]):
        p = g["properties"]
        print(f"    {g['id']:<35} sev={p.get('severity','?'):<3} status={p.get('status','?'):<8} affected={p.get('affected_count',0)}")

    # ========== e. API 对接率验证 ==========
    print("\n【e. feature 级实现率验证】")
    feature_nodes = [n for n in graph_nodes if n["node_type"] == "feature"]
    status_counts = Counter(n["properties"].get("status", "unknown") for n in feature_nodes)
    total_features = len(feature_nodes)
    implemented = status_counts.get("implemented", 0)
    impl_rate = implemented / total_features * 100 if total_features > 0 else 0
    print(f"  feature 总数: {total_features}")
    for st, cnt in sorted(status_counts.items()):
        pct = cnt / total_features * 100
        print(f"    status={st:<15} count={cnt:<4} ({pct:.1f}%)")
    print(f"  implemented 比例: {implemented}/{total_features} = {impl_rate:.1f}%")
    print(f"  审计口径 API 对接率: ~85% (注: feature级实现率与API对接率是不同口径)")
    print(f"  口径说明: feature级status=implemented表示该功能点前后端均已实现；")
    print(f"           API对接率~85%是按前端348个API函数中约有后端支撑的比例统计。")

    # ========== f. 邻居查询验证 ==========
    print("\n【f. 邻居查询验证】")
    # 选一个有代表性的 feature
    test_feature = "feat_graph_core"
    try:
        neighbors = http_get(f"/api/graph/neighbors/{test_feature}")
        n_data = neighbors.get("data", {})
        # 兼容不同返回格式
        if isinstance(n_data, list):
            n_list = n_data
        elif isinstance(n_data, dict):
            n_list = n_data.get("neighbors", n_data.get("nodes", []))
        else:
            n_list = []
        print(f"  查询节点: {test_feature}")
        print(f"  邻居数量: {len(n_list)}")
        for nb in n_list[:10]:
            if isinstance(nb, dict):
                print(f"    - {nb.get('id','?'):<30} type={nb.get('node_type','?'):<15} label={nb.get('label','?')}")
            else:
                print(f"    - {nb}")
        if len(n_list) > 10:
            print(f"    ... 还有 {len(n_list)-10} 个邻居")
        print(f"  邻居查询: {'✓' if len(n_list) > 0 else '✗'}")
    except Exception as e:
        print(f"  邻居查询失败: {e}")

    # ========== g. stats 验证 ==========
    print("\n【g. stats 验证】GET /api/graph/stats")
    stats = http_get("/api/graph/stats")
    s_data = stats.get("data", {})
    print(f"  nodes:      {s_data.get('nodes')} (回读 {len(graph_nodes)}) {'✓' if s_data.get('nodes') == len(graph_nodes) else '✗'}")
    print(f"  edges:      {s_data.get('edges')} (回读 {len(graph_edges)}) {'✓' if s_data.get('edges') == len(graph_edges) else '✗'}")
    print(f"  density:    {s_data.get('density')}")
    print(f"  components: {s_data.get('components')}")

    # ========== 额外: 关系类型统计 ==========
    print("\n【补充: 关系类型统计】")
    seed_rel_counts = Counter(e["relation_type"] for e in seed_edges)
    graph_rel_counts = Counter(
        e.get("relation_type", e.get("relation", "unknown")) for e in graph_edges
    )
    print(f"  {'relation_type':<15} {'种子(预期)':>12} {'回读(实际)':>12}")
    print(f"  {'-'*15} {'-'*12} {'-'*12}")
    for rt in sorted(seed_rel_counts.keys()):
        s_cnt = seed_rel_counts[rt]
        g_cnt = graph_rel_counts.get(rt, 0)
        print(f"  {rt:<15} {s_cnt:>12} {g_cnt:>12}")

    # ========== 总结 ==========
    print("\n" + "=" * 70)
    print("  验证总结")
    print("=" * 70)
    checks = [
        ("回读节点数匹配", len(graph_nodes) == 15 + len(seed_nodes)),
        ("回读边数匹配", len(graph_edges) == 17 + len(seed_edges)),
        ("种子节点零缺失", len(missing_nodes) == 0),
        ("种子边零缺失", len(missing_edges) == 0),
        ("gap=20", gap_count == 20),
        ("pending_decision=6", dec_count == 6),
        ("邻居查询有结果", len(n_list) > 0 if 'n_list' in dir() else False),
        ("stats节点数一致", s_data.get('nodes') == len(graph_nodes)),
        ("stats边数一致", s_data.get('edges') == len(graph_edges)),
    ]
    passed = sum(1 for _, ok in checks if ok)
    for name, ok in checks:
        print(f"  {'✓' if ok else '✗'} {name}")
    print(f"\n  通过: {passed}/{len(checks)}")

    print(f"\n  种子数据文件: {SEED_PATH}")
    print(f"  录入脚本:     {SEED_PATH.replace('functional-requirements-graph-seed.json', 'ingest_functional_requirements.py')}")


if __name__ == "__main__":
    main()
