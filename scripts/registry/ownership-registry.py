# A 项·概念归属登记表生成器（归一化治理 · SSOT 概念归属模式）
#
# 用法：
#   1. 先重新生成重复基线：python scripts/validation/normalization-scan.py --top 0 \
#        --baseline-txt platform/arch-test/baseline/normalization.txt \
#        --json platform/arch-test/baseline/normalization.json
#   2. 再生成本登记表：    python scripts/registry/ownership-registry.py
#
# 输入：platform/arch-test/baseline/normalization.json（跨 crate 重复符号全集）
# 输出：platform/arch-test/baseline/ownership-registry.csv
#   列：kind,name,crates,decision,owner,rationale
#   decision 取值：
#     merge-pending-diff  抽取残渣候选：权威=目标 crate，合并前须逐字段核对
#                         （字段/默认值/序列化/生命周期全一致才可合并）
#     impl-reconciliation 固有 impl 方法面重叠：core 与 svc 各自实现同名方法（平行演化），
#                         须逐方法裁决权威实现后迁移（机械合并止步）
#     keep-distinct       语义已分叉（同名≠同义），保留各自模型 + 显式转换，禁止合并
#     triage-pending      尚未甄别（跨域组合，需人工语义核对）
#
# 治理口径（与 platform/arch-test 扩散检测门禁配套）：
#   - 同名不等于重复债务；SSOT = 「每个明确概念有权威归属」，不是「全仓名字只能定义一次」
#   - 合并只对字段级一致的抽取残渣执行，且随 P2 阶段5 引擎接线一并迁移
#   - keep-distinct 项禁止合并；扩散门禁会拦截其出现第三副本
#   - 新种子判定：人工逐字段核验后填入 SEED 字典（带核验日期）
from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, ValueError):
    pass

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "platform/arch-test/baseline/normalization.json"
OUT = ROOT / "platform/arch-test/baseline/ownership-registry.csv"

data = json.loads(SRC.read_text(encoding="utf-8"))

# ── 自动归属规则：按副本 crate 集合形态判定（依据 P2 阶段5 抽取方向） ──────────
SHAPE_RULES = [
    # (形态的 crate 集合 == 该集合, 权威 crate, decision, rationale)
    ({"mox-ai-alliance-engine", "mox-ai-expert-svc"}, "mox-ai-alliance-engine",
     "merge-pending-diff", "P2阶段5抽取残渣:engine=权威目标,expert-svc=待迁移生产副本,合并随引擎接线执行"),
    ({"mox-ai-expert-core", "mox-ai-expert-svc"}, "mox-ai-expert-core",
     "merge-pending-diff", "expert-core=核心权威,expert-svc=历史副本,合并前逐字段核对"),
    ({"mox-ai-expert-svc", "mox-pipeline-framework"}, "mox-pipeline-framework",
     "merge-pending-diff", "管线框架类型应引用框架 crate,expert-svc 本地副本待迁移"),
    ({"mox-ai-expert-svc", "mox-audit"}, "mox-audit",
     "merge-pending-diff", "审计类型应引用 mox-audit,expert-svc 本地副本待迁移"),
    ({"mox-ai-expert-proto", "mox-ai-expert-svc"}, "mox-ai-expert-proto",
     "merge-pending-diff", "领域协议应以 proto 层为单一真源,expert-svc 副本待迁移"),
]

# ── 种子判定：人工逐字段核验（带日期），优先级最高 ─────────────────────────────
SEED = {
    "enum::AlliancePhase": (
        "mox-ai-alliance-engine", "merge-pending-diff",
        "已核验(2026-09-21):两侧逐字节一致(同derives/serde/7变体同判别值),可安全合并",
    ),
    "enum::GateGrade": (
        "mox-ai-alliance-engine", "merge-pending-diff",
        "已核验(2026-09-21):两侧逐字节一致(含impl label/passed),可安全合并",
    ),
    "enum::ChatRole": (
        "各自拥有", "keep-distinct",
        "已核验(2026-09-21):语义已分叉禁止合并——engine=LLM传输消息(3变体+serde小写);"
        "expert-svc=agent会话消息(4变体含Tool,无serde);跨边界用显式转换",
    ),
    # 批次3 取证（2026-09-21）：固有 impl 方法面重叠（core 与 svc 各自实现同名方法，
    # 属平行演化行为）——机械合并止步，逐方法裁决权威实现后迁移（P2 阶段4 实质工作）
    "struct::CodeIR": (
        "mox-ai-expert-svc(行为权威)", "impl-reconciliation",
        "已核验(2026-09-21):方法面重叠[is_empty,new]——core 与 svc 各有实现体,需逐方法裁决",
    ),
    "struct::DimensionedFlow": (
        "mox-ai-expert-svc(行为权威)", "impl-reconciliation",
        "已核验(2026-09-21):方法面重叠[dimensions_of,from_base,tag]",
    ),
    "struct::CompatibilityRegistry": (
        "mox-ai-expert-svc(行为权威)", "impl-reconciliation",
        "已核验(2026-09-21):方法面重叠[apply_to_pools,new,register_loop,register_mcp,register_skill]",
    ),
    "struct::ReconcileConflict": (
        "mox-ai-expert-svc(行为权威)", "impl-reconciliation",
        "已核验(2026-09-21):方法面重叠[escalated_same_priority,semantic]",
    ),
    "struct::AlgoVerification": (
        "mox-ai-expert-svc(行为权威)", "impl-reconciliation",
        "已核验(2026-09-21):方法面重叠[check]——验证逻辑两套实现,裁决后迁移",
    ),
    "struct::TenantPolicy": (
        "mox-ai-expert-svc(行为权威)", "impl-reconciliation",
        "已核验(2026-09-21):方法面重叠[from_tenant,strength_of]",
    ),
}

rows = []
auto_counts = {}
for kind, items in sorted(data.items()):
    for it in sorted(items, key=lambda x: x["name"]):
        if not it.get("cross_crate"):
            continue
        key = f"{kind}::{it['name']}"
        crates = ";".join(it["crates"])
        if key in SEED:
            owner, decision, rationale = SEED[key]
        else:
            crate_set = set(it["crates"])
            owner, decision, rationale = "", "triage-pending", "跨域组合,需人工语义甄别(同名≠同义)"
            for shape, o, d, r in SHAPE_RULES:
                if crate_set == shape:
                    owner, decision, rationale = o, d, r
                    break
        auto_counts[decision] = auto_counts.get(decision, 0) + 1
        rows.append({
            "kind": kind, "name": it["name"], "crates": crates,
            "decision": decision, "owner": owner, "rationale": rationale,
        })

rows.sort(key=lambda r: (r["decision"], r["kind"], r["name"]))
OUT.parent.mkdir(parents=True, exist_ok=True)
with OUT.open("w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(f, fieldnames=["kind", "name", "crates", "decision", "owner", "rationale"])
    writer.writeheader()
    writer.writerows(rows)

print(f"概念归属登记表已生成：{OUT}")
print(f"共 {len(rows)} 项：")
for d, n in sorted(auto_counts.items()):
    print(f"  {d:20s} {n}")
