#!/usr/bin/env python3
"""
infotopograph 架构约束 CI 测试
每次提交自动运行，检测：
- 循环依赖 (P0)
- 层违规 (P1)
- God Module (扇出>=10) (P1)
- 跨域直连 (>5个) (P2)

退出码: 0=通过, 1=失败
"""

import json
import subprocess
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Dict, List

# 层级定义
LAYER_ORDER = {
    "foundation": 0, "core": 1, "engine": 2,
    "service": 3, "application": 4, "unknown": 5,
    "sdk": 3,  # SDK视为service层同级
}

# Populated from Cargo metadata manifest paths, never legacy crate names.
CRATE_LAYERS = {}
CRATE_DOMAINS = {}

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
    """加载含 normal/dev/build 与可选依赖的保守声明图，不代表单个部署产物的依赖图。"""
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True, text=True, encoding="utf-8", check=True,
        cwd=Path(__file__).resolve().parents[1]
    )
    data = json.loads(result.stdout)
    CRATE_LAYERS.clear()
    CRATE_DOMAINS.clear()
    internal = {p["name"] for p in data["packages"]}
    deps = {}
    for pkg in data["packages"]:
        name = pkg["name"]
        # Use actual workspace paths: the legacy name table predates mox-* renames.
        layer, domain = classify_manifest(pkg["manifest_path"])
        CRATE_LAYERS[name] = layer
        CRATE_DOMAINS[name] = domain
        all_deps = [d["name"] for d in pkg["dependencies"]]
        deps[name] = sorted({d for d in all_deps if d in internal})
    return deps


def classify_manifest(manifest_path):
    parts = Path(manifest_path.replace("\\", "/")).parts
    if "foundation" in parts or "shared" in parts or "framework" in parts:
        return "foundation", "platform"
    if "gateway" in parts:
        return "application", "platform"
    if "domains" in parts:
        index = parts.index("domains")
        domain = parts[index + 1]
        layer = parts[index + 2]
        if domain == "base":
            return "foundation", "platform"
        # API/proto contain contracts, not service implementations.
        if layer in ("api", "proto"):
            return "foundation", domain
        return {"svc": "service", "core": "core", "sdk": "sdk"}.get(layer, "unknown"), domain
    return "unknown", "unknown"


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
        if CRATE_LAYERS.get(crate, "unknown") == "unknown":
            continue
        src_layer = LAYER_ORDER.get(CRATE_LAYERS.get(crate, "unknown"), 5)
        for dep in crate_deps:
            if CRATE_LAYERS.get(dep, "unknown") == "unknown":
                continue
            dst_layer = LAYER_ORDER.get(CRATE_LAYERS.get(dep, "unknown"), 5)
            if src_layer < dst_layer:
                severity = "P1" if CRATE_LAYERS[crate] in ("foundation", "core") or (dst_layer - src_layer) >= 2 else "P2"
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
        if CRATE_LAYERS.get(crate) == "sdk":
            continue
        if src_domain in ("platform", "unknown", "sdk"):
            continue
        for dep in crate_deps:
            if CRATE_LAYERS.get(dep) in ("sdk", "foundation"):
                continue
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
    unknown = sorted(name for name in deps if CRATE_LAYERS.get(name) == "unknown")
    if unknown:
        print(f"  未分类（不参与层级检查）: {', '.join(unknown)}")

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
        failed = failed or any(v.level == "P1" for v in layer_violations)
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
    sys.stdout.reconfigure(encoding="utf-8")
    main()
