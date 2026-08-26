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
import os
import re
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
    manifest_path: str = ""
    deps: List[str] = field(default_factory=list)  # 内部依赖
    internal_deps: List[str] = field(default_factory=list)
    reverse_deps: List[str] = field(default_factory=list)
    layer: str = "unknown"
    domain: str = "unknown"


def detect_layer_domain(manifest_path: str) -> Tuple[str, str]:
    """从 Cargo.toml 路径自动检测层级和业务域（按域组织目录结构）

    路径模式:
      platform/foundation/<crate>        → layer=foundation, domain=foundation
      platform/framework/<crate>         → layer=framework,  domain=framework
      platform/gateway/<crate>           → layer=gateway,    domain=gateway
      platform/domains/<domain>/core/<crate>  → layer=core,    domain=<domain>
      platform/domains/<domain>/svc/<crate>   → layer=service, domain=<domain>
      platform/domains/<domain>/sdk/<crate>   → layer=sdk,     domain=<domain>
      platform/domains/<domain>/api/<crate>   → layer=api,     domain=<domain>
      platform/domains/<domain>/svcapi/<crate>→ layer=svcapi,  domain=<domain>
      projects/<crate>                     → layer=project,   domain=project
    """
    path = manifest_path.replace("\\", "/")

    # foundation 层
    if "/platform/foundation/" in path:
        return "foundation", "foundation"

    # framework 层
    if "/platform/framework" in path:
        return "framework", "framework"

    # gateway 层
    if "/platform/gateway/" in path:
        return "gateway", "gateway"

    # domains 层 - 按域组织
    m = re.search(r"/platform/domains/([^/]+)/(core|svc|sdk|api|svcapi)/", path)
    if m:
        domain = m.group(1)
        layer_map = {"core": "core", "svc": "service", "sdk": "sdk",
                     "api": "api", "svcapi": "svcapi"}
        return layer_map.get(m.group(2), "unknown"), domain

    # projects 层
    if "/projects/" in path:
        return "project", "project"

    return "unknown", "unknown"


# 层级定义（从底层到顶层）— 用于层违规检测
LAYER_ORDER = {
    "foundation": 0,
    "framework": 1,
    "core": 2,
    "api": 3,
    "svcapi": 3,
    "sdk": 4,
    "service": 5,
    "gateway": 6,
    "project": 7,
    "unknown": 99,
}


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
        manifest_path = pkg.get("manifest_path", "")
        layer, domain = detect_layer_domain(manifest_path)
        crates[name] = CrateInfo(
            name=name,
            version=pkg["version"],
            manifest_path=manifest_path,
            deps=all_deps,
            layer=layer,
            domain=domain,
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
    violations = []

    for name, info in crates.items():
        src_layer = LAYER_ORDER.get(info.layer, 99)
        for dep in info.internal_deps:
            if dep not in crates:
                continue
            dst_layer = LAYER_ORDER.get(crates[dep].layer, 99)
            if src_layer < dst_layer and src_layer != 99 and dst_layer != 99:
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
    # 按层级顺序输出
    for layer in sorted(layer_stats.keys(), key=lambda x: LAYER_ORDER.get(x, 99)):
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
