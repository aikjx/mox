#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""10 万节点海量规模容量模型 — 证据生成器

锚定仓库既有实测/承诺基线（单一事实源）：
  - deploy/docs/ha-capacity-tco.md  §二 基准 S: 6 节点 = 100M 顶点 / 500M 边 / 192GB 内存 / 12TB 盘 / 96 cores / 读 60k 写 6k QPS
    缩放公式: 资源 ≈ 基准 × (边数/500M)^0.9
  - platform/domains/kg/svc/mox-kg-storage-svc/tests/t_perf_bench.rs: 单节点写基线 100k ops/s
  - tests/t_distributed_sharding.rs: VID SHA256 分片 100k 顶点 χ² 均匀; 分片数必须 2^k, 在线分裂 16→32
产出: reports/data/<ts>-capacity-model-100k.json + stdout 摘要表
"""
import json, math, os, sys, time

# ── 基线锚点（勿拍脑袋，全部指向权威源）──
BASE_NODES        = 6
BASE_VERTICES     = 100e6      # 100M
BASE_EDGES        = 500e6      # 500M
BASE_MEM_GB       = 192
BASE_DISK_TB      = 12
BASE_CPU          = 96
BASE_QPS_READ     = 60_000
BASE_QPS_WRITE    = 6_000
SCALING_EXP       = 0.9        # ha-capacity-tco §2 缓存收益非线性指数
NODE_OPS_S        = 100_000    # t_perf_bench 单节点内存态基线（上限参考）
NODE_RAFT_WRITE_S = 2_000      # ha-capacity-tco §2.5 单节点写 2k QPS（Raft commit 生产上限）
EDGE_BYTES        = 300        # 边物理均值（顶点≈边×3 密度按 1:5 顶点:边 隐含）
EC_8_3            = 11 / 8     # OSS/云盘纠删码膨胀
RF3               = 3.0        # KG 热数据三副本
UTILIZATION       = 0.6        # 生产水位（留故障域+再平衡余量）

# ── 层级归一化拓扑 ──
CELL_NODES        = 1024       # 2^10，一个 Cell 的爆炸半径上限（控制面 Raft 成员只到 Cell 级）
RACKS_PER_CELL    = 16
SHARDS_PER_CELL   = 4096       # 2^12，每 Cell 分片数（vid_hash_shard 要求 2^k）
TARGET_NODES      = 100_000

def naive_scale(edges):
    """ha-capacity-tco 公式：给定边数所需节点数（线性外推上限参考）"""
    return BASE_NODES * (edges / BASE_EDGES) ** SCALING_EXP

def solve_edges(nodes):
    """反解：naive 公式下 N 节点可支撑的边数"""
    return BASE_EDGES * (nodes / BASE_NODES) ** (1 / SCALING_EXP)

def fmt(n, unit=""):
    for div, suf in [(1e12, "T"), (1e9, "G"), (1e6, "M"), (1e3, "K")]:
        if n >= div:
            return f"{n/div:.2f}{suf}{unit}"
    return f"{n:.0f}{unit}"

def main():
    cells = math.ceil(TARGET_NODES / CELL_NODES)
    cluster_nodes = cells * CELL_NODES
    per_node_edges = (BASE_EDGES / BASE_NODES) * UTILIZATION
    total_edges = TARGET_NODES * per_node_edges
    total_vertices = total_edges / 5.0

    naive_nodes = naive_scale(total_edges)
    mem_gb  = BASE_MEM_GB  / BASE_NODES  * TARGET_NODES
    disk_tb = BASE_DISK_TB / BASE_NODES  * TARGET_NODES
    cpu     = BASE_CPU     / BASE_NODES  * TARGET_NODES
    qps_r   = BASE_QPS_READ  / BASE_NODES * TARGET_NODES * UTILIZATION
    qps_w   = BASE_QPS_WRITE / BASE_NODES * TARGET_NODES * UTILIZATION

    hot_raw_tb  = total_edges * EDGE_BYTES * RF3 / 1e12
    oss_ec_tb   = total_edges * EDGE_BYTES * EC_8_3 / 1e12
    import_days = total_edges / (NODE_RAFT_WRITE_S * TARGET_NODES * UTILIZATION) / 86400
    import_days_mem = total_edges / (NODE_OPS_S * TARGET_NODES * UTILIZATION) / 86400

    hb_global_pps = TARGET_NODES / 10 / 10   # 节点→机架聚合10:1→Cell聚合10:1
    meta_raft_members_per_cell = 5
    shard_replicas = SHARDS_PER_CELL * RF3
    shards_per_node = shard_replicas / CELL_NODES

    out = {
        "generated_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "assumptions": {
            "baseline": "ha-capacity-tco.md §二 基准S (6节点/100M顶点/500M边), 缩放指数0.9",
            "per_node_density": f"基线密度×水位{UTILIZATION:.0%} = {per_node_edges/1e6:.1f}M 边/节点",
            "cell": f"{CELL_NODES}节点(2^10) 一个Cell, {RACKS_PER_CELL}机架/Cell, {SHARDS_PER_CELL}分片/Cell(2^k)",
            "redundancy": f"KG热数据RF{RF3:.0f}, 冷数据/云盘 EC 8+3",
        },
        "cluster": {
            "target_nodes": TARGET_NODES,
            "cells": cells,
            "provisioned_nodes": cluster_nodes,
            "vertices_total": total_vertices,
            "edges_total": total_edges,
            "naive_formula_nodes_for_this_edges": naive_nodes,
        },
        "resources": {
            "memory_tb": mem_gb / 1024, "disk_tb_naive": disk_tb, "cpu_mcores": cpu / 1000,
            "qps_read": qps_r, "qps_write": qps_w,
            "hot_store_raw_tb": hot_raw_tb, "cold_oss_ec_tb": oss_ec_tb,
            "bulk_import_days": import_days,
            "bulk_import_days_inmem_upper_bound": import_days_mem,
        },
        "control_plane": {
            "heartbeat_pps_at_global_registry": hb_global_pps,
            "meta_raft_members_per_cell": meta_raft_members_per_cell,
            "shard_replicas_per_cell": shard_replicas,
            "shard_replicas_per_node": shards_per_node,
            "global_routing_entries": cells * SHARDS_PER_CELL,
        },
    }
    os.makedirs("reports/data", exist_ok=True)
    path = f"reports/data/{time.strftime('%Y%m%d-%H%M%S')}-capacity-model-100k.json"
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)

    print(f"{'指标':<28}{'值':>18}")
    print("-" * 48)
    r = out
    print(f"{'目标节点数':<28}{TARGET_NODES:>18,}")
    print(f"{'Cell 数 (爆炸半径单元)':<24}{cells:>18}")
    print(f"{'总顶点':<30}{fmt(total_vertices):>18}")
    print(f"{'总边':<31}{fmt(total_edges):>18}")
    print(f"{'naive公式所需节点(交叉校验)':<19}{naive_nodes:>18,.0f}")
    print(f"{'内存':<31}{fmt(mem_gb/1000,'TB'):>18}")
    print(f"{'读QPS@60%水位':<24}{fmt(qps_r):>18}")
    print(f"{'写QPS@60%水位':<24}{fmt(qps_w):>18}")
    print(f"{'热数据RF3':<28}{fmt(hot_raw_tb*1000,'GB'):>18}")
    print(f"{'冷数据OSS EC8+3':<23}{fmt(oss_ec_tb*1000,'GB'):>18}")
    print(f"{'全量导入天数@Raft写2k/节点':<20}{import_days:>18.2f}")
    print(f"{'导入上限参考@内存态100k/s':<20}{import_days_mem*24:>18.2f}h")
    print(f"{'全局心跳聚合后 pps':<24}{hb_global_pps:>18,.0f}")
    print(f"{'每节点 shard 副本数':<23}{shards_per_node:>18.1f}")
    print(f"\n证据 JSON: {path}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
