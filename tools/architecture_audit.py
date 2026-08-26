#!/usr/bin/env python3
"""
infotopograph 架构算法验证工具
- 循环依赖检测 (DFS)
- 耦合度分析 (入度/出度/扇入扇出)
- 模块独立性评分
- 层违规检测
- God Module 识别
- 归一化建议
"""

import json
import subprocess
import sys
from collections import defaultdict, deque
from dataclasses import dataclass, field
from typing import Dict, List, Set, Tuple

# 只分析 workspace 内部 crate 之间的依赖
INTERNAL_CRATES = set()

@dataclass
class CrateInfo:
    name: str
    version: str
    deps: List[str] = field(default_factory=list)  # 内部依赖
    internal_deps: List[str] = field(default_factory=list)
    reverse_deps: List[str] = field(default_factory=list)
    layer: str = "unknown"
    domain: str = "unknown"

# 层级定义（从底层到顶层）
LAYERS = {
    "foundation": ["mox-common-meta", "mox-domain-abstractions"],
    "core": [
        "mox-formulas-core", "mox-norm-core", "mox-intent-core",
        "operator-core", "graph-algorithms", "mox-standards", "mox-system",
        "mox-ai-core", "mox-graph-meta",
    ],
    "engine": [
        "operator-wasm", "optimizer", "mox-graph-storage", "mox-graph-algorithms",
        "mox-formulas-native", "mox-norm-intent-native", "xiaobai-dsp",
    ],
    "service": [
        "kg-hub", "ai-agent", "mox-expert", "flow-ai",
        "primiflow-core", "primiflow-fusion",
        "mox-graph-service", "mox-graph-streams", "mox-graph-spark",
        "mox-cloud-drive-master", "mox-cloud-drive-volume", "mox-cloud-drive-s3", "mox-cloud-drive-filer",
        "mox-data-plane", "mox-etl-wasm", "mox-compliance", "mox-fusion",
        "mox-sdk-cloud", "mox-sdk-graph",
    ],
    "application": [
        "runtime", "mox-server", "mox-t21-harness",
        "xiaobai-core", "xiaobai-asr", "xiaobai-intent", "xiaobai-operators", "xiaobai-desktop",
        "template-market", "business-catalog", "hermes-flow-bridge",
        "xiaobai-dsp-py",
    ],
}

# 业务域定义
DOMAINS = {
    "knowledge_graph": ["kg-hub", "mox-graph-storage", "mox-graph-service", "mox-graph-streams",
                         "mox-graph-spark", "mox-graph-meta", "graph-algorithms", "mox-fusion"],
    "ai_intelligent": ["ai-agent", "mox-ai-core", "mox-expert", "flow-ai", "mox-intent-core"],
    "flow_automation": ["operator-core", "operator-wasm", "optimizer", "primiflow-core",
                        "primiflow-fusion", "hermes-flow-bridge"],
    "data_governance": ["mox-data-plane", "mox-etl-wasm", "mox-compliance", "mox-standards",
                         "mox-norm-core", "mox-formulas-core", "business-catalog"],
    "cloud_storage": ["mox-cloud-drive-master", "mox-cloud-drive-volume", "mox-cloud-drive-s3",
                      "mox-cloud-drive-filer", "mox-domain-abstractions"],
    "platform": ["mox-common-meta", "mox-system", "mox-server", "runtime", "mox-t21-harness"],
    "xiaobai_voice": ["xiaobai-core", "xiaobai-asr", "xiaobai-intent", "xiaobai-operators",
                      "xiaobai-desktop", "xiaobai-dsp", "xiaobai-dsp-py"],
    "market": ["template-market"],
    "sdk": ["mox-sdk-cloud", "mox-sdk-graph", "mox-formulas-native", "mox-norm-intent-native"],
}


def get_layer(name: str) -> str:
    for layer, crates in LAYERS.items():
        if name in crates:
            return layer
    return "unknown"


def get_domain(name: str) -> str:
    for domain, crates in DOMAINS.items():
        if name in crates:
            return domain
    return "unknown"


def load_workspace() -> Dict[str, CrateInfo]:
    """通过 cargo metadata 加载 workspace 依赖图"""
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True, text=True, cwd=r"D:\a10\aikjx\gitcode\infotopograph"
    )
    data = json.loads(result.stdout)

    crates = {}
    for pkg in data["packages"]:
        name = pkg["name"]
        INTERNAL_CRATES.add(name)
        all_deps = [d["name"] for d in pkg["dependencies"]]
        crates[name] = CrateInfo(
            name=name,
            version=pkg["version"],
            deps=all_deps,
            layer=get_layer(name),
            domain=get_domain(name),
        )

    # 过滤内部依赖
    for name, info in crates.items():
        info.internal_deps = [d for d in info.deps if d in INTERNAL_CRATES]

    # 计算反向依赖
    for name, info in crates.items():
        for dep in info.internal_deps:
            if dep in crates:
                crates[dep].reverse_deps.append(name)

    return crates


def detect_cycles(crates: Dict[str, CrateInfo]) -> List[List[str]]:
    """DFS 检测循环依赖"""
    cycles = []
    visited = set()
    rec_stack = set()
    path = []

    def dfs(node):
        visited.add(node)
        rec_stack.add(node)
        path.append(node)

        for neighbor in crates[node].internal_deps:
            if neighbor not in crates:
                continue
            if neighbor not in visited:
                dfs(neighbor)
            elif neighbor in rec_stack:
                # 找到环
                cycle_start = path.index(neighbor)
                cycle = path[cycle_start:] + [neighbor]
                cycles.append(cycle)

        path.pop()
        rec_stack.remove(node)

    for name in crates:
        if name not in visited:
            dfs(name)

    return cycles


def compute_metrics(crates: Dict[str, CrateInfo]):
    """计算耦合度/独立性指标"""
    results = []
    for name, info in crates.items():
        fan_out = len(info.internal_deps)  # 扇出（依赖数）
        fan_in = len(info.reverse_deps)    # 扇入（被依赖数）
        total_internal = fan_in + fan_out

        # 独立性评分 (0-100): 越高越独立
        # 公式: 100 - min(100, fan_out * 5 + fan_in * 2)
        independence = max(0, 100 - min(100, fan_out * 5 + fan_in * 2))

        # 耦合度评分 (0-100): 越高耦合越严重
        coupling = min(100, total_internal * 3)

        # 不稳定度 (Robert Martin): fan_out / (fan_in + fan_out)
        instability = fan_out / (fan_in + fan_out) if (fan_in + fan_out) > 0 else 0

        results.append({
            "name": name,
            "layer": info.layer,
            "domain": info.domain,
            "fan_out": fan_out,
            "fan_in": fan_in,
            "total_deps": total_internal,
            "independence": independence,
            "coupling": coupling,
            "instability": round(instability, 3),
            "deps": info.internal_deps,
            "reverse_deps": info.reverse_deps,
        })

    return sorted(results, key=lambda x: x["coupling"], reverse=True)


def detect_layer_violations(crates: Dict[str, CrateInfo]) -> List[Dict]:
    """检测层违规（底层依赖顶层）"""
    layer_order = {"foundation": 0, "core": 1, "engine": 2, "service": 3, "application": 4, "unknown": 5}
    violations = []

    for name, info in crates.items():
        src_layer = layer_order.get(info.layer, 5)
        for dep in info.internal_deps:
            if dep not in crates:
                continue
            dst_layer = layer_order.get(crates[dep].layer, 5)
            if src_layer < dst_layer:  # 底层依赖顶层 = 违规
                violations.append({
                    "crate": name,
                    "crate_layer": info.layer,
                    "depends_on": dep,
                    "dep_layer": crates[dep].layer,
                    "severity": "HIGH" if (dst_layer - src_layer) >= 2 else "MEDIUM",
                })

    return violations


def detect_domain_violations(crates: Dict[str, CrateInfo]) -> List[Dict]:
    """检测跨域依赖（非平台域依赖其他业务域）"""
    violations = []
    for name, info in crates.items():
        if info.domain in ("platform", "unknown"):
            continue
        for dep in info.internal_deps:
            if dep not in crates:
                continue
            dep_domain = crates[dep].domain
            if dep_domain not in (info.domain, "platform", "unknown") and dep_domain != "sdk":
                violations.append({
                    "crate": name,
                    "crate_domain": info.domain,
                    "depends_on": dep,
                    "dep_domain": dep_domain,
                })
    return violations


def identify_god_modules(metrics: List[Dict], threshold: int = 10) -> List[Dict]:
    """识别 God Module（依赖数超过阈值）"""
    return [m for m in metrics if m["fan_out"] >= threshold]


def generate_report(crates, cycles, metrics, layer_violations, domain_violations, god_modules):
    """生成架构审计报告"""
    report = []
    report.append("=" * 80)
    report.append("infotopograph 架构算法验证报告")
    report.append("=" * 80)
    report.append("")

    # 1. 概览
    report.append("## 1. 概览")
    report.append(f"- 总 crate 数: {len(crates)}")
    report.append(f"- 内部依赖边数: {sum(len(c.internal_deps) for c in crates.values())}")
    report.append(f"- 循环依赖: {len(cycles)} 个")
    report.append(f"- 层违规: {len(layer_violations)} 个")
    report.append(f"- 跨域依赖: {len(domain_violations)} 个")
    report.append(f"- God Module: {len(god_modules)} 个")
    report.append("")

    # 2. 循环依赖
    report.append("## 2. 循环依赖检测")
    if cycles:
        for i, cycle in enumerate(cycles):
            report.append(f"  循环 {i+1}: {' → '.join(cycle)}")
    else:
        report.append("  ✅ 无循环依赖")
    report.append("")

    # 3. God Module
    report.append("## 3. God Module 识别 (扇出 >= 10)")
    for m in god_modules:
        report.append(f"  🔴 {m['name']} (层:{m['layer']}, 域:{m['domain']})")
        report.append(f"     扇出: {m['fan_out']}, 扇入: {m['fan_in']}, 耦合度: {m['coupling']}")
        report.append(f"     依赖: {', '.join(m['deps'][:10])}{'...' if len(m['deps'])>10 else ''}")
    report.append("")

    # 4. 层违规
    report.append("## 4. 层违规检测 (底层依赖顶层)")
    if layer_violations:
        for v in layer_violations:
            icon = "🔴" if v["severity"] == "HIGH" else "🟡"
            report.append(f"  {icon} [{v['severity']}] {v['crate']}({v['crate_layer']}) → {v['depends_on']}({v['dep_layer']})")
    else:
        report.append("  ✅ 无层违规")
    report.append("")

    # 5. 跨域依赖
    report.append("## 5. 跨域依赖检测")
    if domain_violations:
        for v in domain_violations:
            report.append(f"  🟡 {v['crate']}({v['crate_domain']}) → {v['depends_on']}({v['dep_domain']})")
    else:
        report.append("  ✅ 无跨域依赖")
    report.append("")

    # 6. 独立性评分排行
    report.append("## 6. 模块独立性评分 (Top 10 最差)")
    low_independence = sorted(metrics, key=lambda x: x["independence"])[:10]
    for m in low_independence:
        bar = "█" * (m["independence"] // 5) + "░" * (20 - m["independence"] // 5)
        report.append(f"  {m['independence']:3d} {bar} {m['name']} (扇出:{m['fan_out']}, 扇入:{m['fan_in']})")
    report.append("")

    # 7. 按层级统计
    report.append("## 7. 按层级统计")
    layer_stats = defaultdict(lambda: {"count": 0, "fan_out": 0, "fan_in": 0})
    for m in metrics:
        layer_stats[m["layer"]]["count"] += 1
        layer_stats[m["layer"]]["fan_out"] += m["fan_out"]
        layer_stats[m["layer"]]["fan_in"] += m["fan_in"]
    for layer in ["foundation", "core", "engine", "service", "application", "unknown"]:
        s = layer_stats[layer]
        report.append(f"  {layer:12s}: {s['count']:3d} crates, 扇出={s['fan_out']:3d}, 扇入={s['fan_in']:3d}")
    report.append("")

    # 8. 按业务域统计
    report.append("## 8. 按业务域统计")
    domain_stats = defaultdict(lambda: {"count": 0, "fan_out": 0, "fan_in": 0})
    for m in metrics:
        domain_stats[m["domain"]]["count"] += 1
        domain_stats[m["domain"]]["fan_out"] += m["fan_out"]
        domain_stats[m["domain"]]["fan_in"] += m["fan_in"]
    for domain, s in sorted(domain_stats.items(), key=lambda x: -x[1]["count"]):
        report.append(f"  {domain:20s}: {s['count']:3d} crates, 扇出={s['fan_out']:3d}, 扇入={s['fan_in']:3d}")
    report.append("")

    # 9. 优化建议
    report.append("## 9. 优化建议")
    report.append("")
    report.append("### 9.1 紧急 (P0)")
    if god_modules:
        report.append("- 🔴 拆分 God Module:")
        for m in god_modules:
            report.append(f"  - {m['name']}: 按业务域拆分为独立服务")
    if layer_violations:
        report.append("- 🔴 修复层违规:")
        for v in layer_violations:
            if v["severity"] == "HIGH":
                report.append(f"  - {v['crate']} → {v['depends_on']}: 通过接口抽象/事件解耦")
    report.append("")
    report.append("### 9.2 高优 (P1)")
    if domain_violations:
        report.append("- 🟡 减少跨域依赖:")
        for v in domain_violations[:5]:
            report.append(f"  - {v['crate']}({v['crate_domain']}) → {v['depends_on']}({v['dep_domain']}): 考虑通过平台层中转")
    report.append("- 🟡 建设 mox-framework 基础框架层: 统一认证/租户/调度/通知/可观测")
    report.append("- 🟡 三层分离: 每个服务拆为 api / service-api / service")
    report.append("")
    report.append("### 9.3 中优 (P2)")
    report.append("- 🟢 业务域重组: 按知识图谱/AI/流程/数据/平台 5域重组")
    report.append("- 🟢 归一化命名: 统一 mox-<domain>-<layer> 命名规范")
    report.append("- 🟢 接口契约归一化: 统一 gRPC .proto + JSON-RPC 方法命名")
    report.append("")

    report.append("=" * 80)
    report.append("报告结束")
    report.append("=" * 80)

    return "\n".join(report)


def main():
    print("正在加载 workspace 依赖图...")
    crates = load_workspace()
    print(f"加载了 {len(crates)} 个 crate")

    print("\n正在检测循环依赖...")
    cycles = detect_cycles(crates)
    print(f"发现 {len(cycles)} 个循环依赖")

    print("\n正在计算耦合度/独立性指标...")
    metrics = compute_metrics(crates)

    print("\n正在检测层违规...")
    layer_violations = detect_layer_violations(crates)
    print(f"发现 {len(layer_violations)} 个层违规")

    print("\n正在检测跨域依赖...")
    domain_violations = detect_domain_violations(crates)
    print(f"发现 {len(domain_violations)} 个跨域依赖")

    print("\n正在识别 God Module...")
    god_modules = identify_god_modules(metrics, threshold=10)
    print(f"发现 {len(god_modules)} 个 God Module")

    print("\n" + "=" * 80)
    report = generate_report(crates, cycles, metrics, layer_violations, domain_violations, god_modules)
    print(report)

    # 保存报告
    report_path = r"D:\a10\aikjx\gitcode\infotopograph\docs\architecture-audit-report.txt"
    with open(report_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\n报告已保存到: {report_path}")

    # 保存 JSON 数据供后续使用
    json_path = r"D:\a10\aikjx\gitcode\infotopograph\docs\architecture-metrics.json"
    json_data = {
        "total_crates": len(crates),
        "cycles": cycles,
        "layer_violations": layer_violations,
        "domain_violations": domain_violations,
        "god_modules": [m["name"] for m in god_modules],
        "metrics": metrics,
    }
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(json_data, f, ensure_ascii=False, indent=2)
    print(f"指标数据已保存到: {json_path}")


if __name__ == "__main__":
    main()
