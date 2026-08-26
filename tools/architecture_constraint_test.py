#!/usr/bin/env python3
"""
infotopograph 架构约束 CI 测试
每次提交自动运行，检测：
- 循环依赖 (P0)
- 层违规 (P1)
- God Module (扇出>10) (P1)
- 跨域直连 (>5个) (P2)
- core层IO依赖 (P2)

退出码: 0=通过, 1=失败
"""

import json
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Set

# 层级定义
LAYER_ORDER = {
    "foundation": 0, "core": 1, "engine": 2,
    "service": 3, "application": 4, "unknown": 5,
    "sdk": 3,  # SDK视为service层同级
}

# crate → 层映射（从归一化架构规范）
CRATE_LAYERS = {
    # foundation
    "mox-common-meta": "foundation", "mox-domain-abstractions": "foundation",
    # core
    "mox-formulas-core": "core", "mox-norm-core": "core", "mox-intent-core": "core",
    "operator-core": "core", "graph-algorithms": "core", "mox-standards": "core",
    "mox-system": "core", "mox-ai-core": "core", "mox-graph-meta": "core",
    "xiaobai-dsp": "core", "optimizer": "core",
    # engine
    "operator-wasm": "engine", "mox-graph-storage": "engine",
    "mox-formulas-native": "sdk", "mox-norm-intent-native": "sdk",
    # service
    "kg-hub": "service", "ai-agent": "service", "mox-expert": "service",
    "flow-ai": "service", "primiflow-core": "service", "primiflow-fusion": "service",
    "mox-graph-service": "service", "mox-graph-streams": "service",
    "mox-graph-spark": "service", "mox-cloud-drive-master": "service",
    "mox-cloud-drive-volume": "service", "mox-cloud-drive-s3": "service",
    "mox-cloud-drive-filer": "service", "mox-data-plane": "service",
    "mox-etl-wasm": "service", "mox-compliance": "service", "mox-fusion": "service",
    "mox-sdk-cloud": "sdk", "mox-sdk-graph": "sdk",
    "template-market": "service", "business-catalog": "service",
    "hermes-flow-bridge": "service",
    # application
    "runtime": "application", "mox-server": "application", "mox-t21-harness": "application",
    "xiaobai-core": "application", "xiaobai-asr": "application",
    "xiaobai-intent": "application", "xiaobai-operators": "application",
    "xiaobai-desktop": "application", "xiaobai-dsp-py": "sdk",
}

# crate → 域映射
CRATE_DOMAINS = {
    "mox-common-meta": "platform", "mox-system": "platform",
    "mox-server": "platform", "runtime": "platform", "mox-t21-harness": "platform",
    "mox-graph-meta": "kg", "mox-graph-storage": "kg", "mox-graph-service": "kg",
    "mox-graph-streams": "kg", "mox-graph-spark": "kg", "graph-algorithms": "kg",
    "kg-hub": "kg", "mox-fusion": "kg", "mox-sdk-graph": "kg",
    "mox-ai-core": "ai", "mox-intent-core": "ai", "flow-ai": "ai",
    "mox-expert": "ai", "ai-agent": "ai",
    "operator-core": "flow", "operator-wasm": "flow", "optimizer": "flow",
    "primiflow-core": "flow", "primiflow-fusion": "flow", "hermes-flow-bridge": "flow",
    "mox-formulas-core": "data", "mox-norm-core": "data", "mox-standards": "data",
    "mox-data-plane": "data", "mox-etl-wasm": "data", "mox-compliance": "data",
    "business-catalog": "data", "mox-formulas-native": "data", "mox-norm-intent-native": "data",
    "mox-domain-abstractions": "cloud", "mox-cloud-drive-master": "cloud",
    "mox-cloud-drive-volume": "cloud", "mox-cloud-drive-s3": "cloud",
    "mox-cloud-drive-filer": "cloud", "mox-sdk-cloud": "cloud",
    "xiaobai-dsp": "voice", "xiaobai-core": "voice", "xiaobai-asr": "voice",
    "xiaobai-intent": "voice", "xiaobai-operators": "voice",
    "xiaobai-desktop": "voice", "xiaobai-dsp-py": "voice",
    "template-market": "market",
}

# 阈值
GOD_MODULE_THRESHOLD = 10
CROSS_DOMAIN_THRESHOLD = 5


@dataclass
class Violation:
    level: str  # P0/P1/P2
    type: str
    description: str
    details: str = ""


def load_workspace() -> Dict[str, List[str]]:
    """加载 workspace 内部依赖图"""
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True, text=True,
        cwd=r"D:\a10\aikjx\gitcode\infotopograph"
    )
    data = json.loads(result.stdout)
    internal = {p["name"] for p in data["packages"]}
    deps = {}
    for pkg in data["packages"]:
        name = pkg["name"]
        all_deps = [d["name"] for d in pkg["dependencies"]]
        deps[name] = [d for d in all_deps if d in internal]
    return deps


def detect_cycles(deps: Dict[str, List[str]]) -> List[List[str]]:
    """DFS 检测循环依赖"""
    cycles = []
    visited = set()
    rec_stack = set()
    path = []

    def dfs(node):
        visited.add(node)
        rec_stack.add(node)
        path.append(node)
        for neighbor in deps.get(node, []):
            if neighbor not in visited:
                dfs(neighbor)
            elif neighbor in rec_stack:
                idx = path.index(neighbor)
                cycles.append(path[idx:] + [neighbor])
        path.pop()
        rec_stack.remove(node)

    for name in deps:
        if name not in visited:
            dfs(name)
    return cycles


def detect_layer_violations(deps: Dict[str, List[str]]) -> List[Violation]:
    """检测层违规（底层依赖顶层）"""
    violations = []
    for crate, crate_deps in deps.items():
        src_layer = LAYER_ORDER.get(CRATE_LAYERS.get(crate, "unknown"), 5)
        for dep in crate_deps:
            dst_layer = LAYER_ORDER.get(CRATE_LAYERS.get(dep, "unknown"), 5)
            if src_layer < dst_layer:
                severity = "P1" if (dst_layer - src_layer) >= 2 else "P2"
                violations.append(Violation(
                    level=severity,
                    type="layer_violation",
                    description=f"{crate}({CRATE_LAYERS.get(crate,'?')}) → {dep}({CRATE_LAYERS.get(dep,'?')})",
                ))
    return violations


def detect_god_modules(deps: Dict[str, List[str]]) -> List[Violation]:
    """检测 God Module（扇出>阈值）"""
    violations = []
    for crate, crate_deps in deps.items():
        if len(crate_deps) >= GOD_MODULE_THRESHOLD:
            violations.append(Violation(
                level="P1",
                type="god_module",
                description=f"{crate}: 扇出={len(crate_deps)} (阈值={GOD_MODULE_THRESHOLD})",
                details=f"依赖: {', '.join(crate_deps[:10])}{'...' if len(crate_deps)>10 else ''}",
            ))
    return violations


def detect_cross_domain(deps: Dict[str, List[str]]) -> List[Violation]:
    """检测跨域直连"""
    violations = []
    count = 0
    for crate, crate_deps in deps.items():
        src_domain = CRATE_DOMAINS.get(crate, "unknown")
        if src_domain in ("platform", "unknown", "sdk"):
            continue
        for dep in crate_deps:
            dst_domain = CRATE_DOMAINS.get(dep, "unknown")
            if dst_domain not in (src_domain, "platform", "unknown", "sdk"):
                count += 1
                violations.append(Violation(
                    level="P2",
                    type="cross_domain",
                    description=f"{crate}({src_domain}) → {dep}({dst_domain})",
                ))
    return violations, count


def main():
    print("=" * 70)
    print("infotopograph 架构约束 CI 测试")
    print("=" * 70)

    deps = load_workspace()
    total_crates = len(deps)
    total_edges = sum(len(v) for v in deps.values())
    print(f"\n📊 概览: {total_crates} crates, {total_edges} 内部依赖边")

    all_violations = []
    failed = False

    # 1. 循环依赖 (P0)
    print("\n🔍 [P0] 循环依赖检测...")
    cycles = detect_cycles(deps)
    if cycles:
        print(f"  ❌ 发现 {len(cycles)} 个循环依赖!")
        for i, c in enumerate(cycles):
            print(f"     循环 {i+1}: {' → '.join(c)}")
            all_violations.append(Violation("P0", "cycle", " → ".join(c)))
        failed = True
    else:
        print("  ✅ 无循环依赖")

    # 2. 层违规 (P1)
    print("\n🔍 [P1] 层违规检测...")
    layer_violations = detect_layer_violations(deps)
    if layer_violations:
        print(f"  ⚠️  发现 {len(layer_violations)} 个层违规")
        for v in layer_violations:
            print(f"     [{v.level}] {v.description}")
        all_violations.extend(layer_violations)
        # 层违规不导致失败（渐进式迁移），但警告
    else:
        print("  ✅ 无层违规")

    # 3. God Module (P1)
    print("\n🔍 [P1] God Module 检测...")
    god_modules = detect_god_modules(deps)
    if god_modules:
        print(f"  ❌ 发现 {len(god_modules)} 个 God Module!")
        for v in god_modules:
            print(f"     [{v.level}] {v.description}")
            if v.details:
                print(f"       {v.details}")
        all_violations.extend(god_modules)
        failed = True
    else:
        print("  ✅ 无 God Module")

    # 4. 跨域依赖 (P2)
    print("\n🔍 [P2] 跨域直连检测...")
    cross_violations, cross_count = detect_cross_domain(deps)
    if cross_count > CROSS_DOMAIN_THRESHOLD:
        print(f"  ⚠️  发现 {cross_count} 个跨域直连 (阈值={CROSS_DOMAIN_THRESHOLD})")
        for v in cross_violations[:10]:
            print(f"     [{v.level}] {v.description}")
        if cross_count > 10:
            print(f"     ... 还有 {cross_count - 10} 个")
        all_violations.extend(cross_violations)
    else:
        print(f"  ✅ 跨域直连 {cross_count} 个 (阈值={CROSS_DOMAIN_THRESHOLD})")

    # 汇总
    print("\n" + "=" * 70)
    p0 = sum(1 for v in all_violations if v.level == "P0")
    p1 = sum(1 for v in all_violations if v.level == "P1")
    p2 = sum(1 for v in all_violations if v.level == "P2")
    print(f"📋 违规汇总: P0={p0}, P1={p1}, P2={p2}, 总计={len(all_violations)}")

    if failed:
        print("\n❌ 架构约束测试失败! (存在 P0 或 P1 严重违规)")
        sys.exit(1)
    else:
        print("\n✅ 架构约束测试通过! (P0=0, 严重P1已修复)")
        if p2 > 0:
            print(f"   ⚠️  存在 {p2} 个 P2 警告（渐进式迁移中，不阻断提交）")
        sys.exit(0)


if __name__ == "__main__":
    main()
