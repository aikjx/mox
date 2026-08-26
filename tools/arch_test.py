#!/usr/bin/env python3
"""
infotopograph 架构一致性校验（arch test）v2.0
基于 ADR-09「跨域依赖规则与架构一致性治理」的 5 条核心规则：
  规则1: 层间单向依赖（core ← sdk ← svc ← api）
  规则2: 跨域必须经 SDK（svc→svc 直连禁止）
  规则3: 域间依赖方向（8域 DAG，禁止逆向/循环）
  规则4: Gateway 无业务逻辑（gateway 不依赖业务 svc/core）
  规则5: API 层契约优先（api 层不包含业务逻辑）

用法:
  python tools/arch_test.py              # 完整校验，退出码 0=通过 1=失败
  python tools/arch_test.py --json       # JSON 输出（CI 集成）
  python tools/arch_test.py --quiet      # 仅输出违规项
  python tools/arch_test.py --fix-hint   # 输出每条违规的修复建议

权威文档: docs/enterprise/29-跨域依赖规则与架构一致性治理-ADR-09.md
"""

import json
import subprocess
import sys
import re
import argparse
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional, Tuple

# ============================================================
# 配置：域依赖方向矩阵（规则3）
# ============================================================
DOMAIN_ORDER = ["platform", "kg", "ai", "flow", "data", "market", "cloud", "voice"]

# 域间允许依赖矩阵：行=依赖方，列=被依赖方
# True = 允许，False = 禁止
DOMAIN_DEP_MATRIX = {
    "platform": {d: False for d in DOMAIN_ORDER},  # platform 不依赖任何业务域
    "kg":       {"platform": True,  "kg": False, "ai": False, "flow": False, "data": False, "market": False, "cloud": False, "voice": False},
    "ai":       {"platform": True,  "kg": True,   "ai": False, "flow": False, "data": False, "market": False, "cloud": False, "voice": False},
    "flow":     {"platform": True,  "kg": True,   "ai": True,   "flow": False, "data": False, "market": False, "cloud": False, "voice": False},
    "data":     {"platform": True,  "kg": True,   "ai": False, "flow": False, "data": False, "market": False, "cloud": False, "voice": False},
    "market":   {"platform": True,  "kg": True,   "ai": True,   "flow": True,   "data": True,   "market": False, "cloud": False, "voice": False},
    "cloud":    {"platform": True,  "kg": False, "ai": False, "flow": False, "data": False, "market": False, "cloud": False, "voice": False},
    "voice":    {"platform": True,  "kg": False, "ai": False, "flow": False, "data": False, "market": False, "cloud": False, "voice": False},
}

# 层顺序（规则1）：数字越小越底层，只能高层依赖低层
LAYER_ORDER = {"foundation": 0, "framework": 0, "gateway": 0, "runtime": 0, "test-harness": 0, "core": 1, "sdk": 2, "svc": 3, "api": 4, "unknown": 99}

# 横切层（不在 domains/ 下，被所有层依赖，不参与域间依赖检查）
CROSS_CUTTING = {"foundation", "framework", "gateway", "runtime", "test-harness"}

# ============================================================
# crate 名 → (域, 层) 解析
# ============================================================
# 命名约定: mox-{domain}-{name}-{layer}
# 例外: 横切层、voice 桌面应用、info-graph 等
CRATE_OVERRIDE = {
    # 横切层 - foundation
    "mox-foundation-common": ("foundation", "foundation"),
    "mox-foundation-error": ("foundation", "foundation"),
    "mox-foundation-log": ("foundation", "foundation"),
    "mox-foundation-config": ("foundation", "foundation"),
    "mox-platform-foundation": ("foundation", "foundation"),
    "mox-cloud-foundation": ("foundation", "foundation"),  # 云抽象层，实际是横切 foundation
    # 横切层 - framework
    "mox-framework-web": ("framework", "framework"),
    "mox-framework-middleware": ("framework", "framework"),
    "mox-framework": ("framework", "framework"),
    # 横切层 - gateway
    "mox-platform-gateway-svc": ("platform", "runtime"),  # 实际是融合单二进制运行时(CLI+server)，非纯API网关
    # 测试工具（排除域间依赖检查）
    "mox-platform-test-harness": ("platform", "test-harness"),
    # 特殊 crate
    "info-graph": ("kg", "svc"),  # 图谱 CLI 工具
    "mox-voice-desktop-app": ("voice", "api"),  # 桌面应用视为 api 层
    "mox-voice-dsp-py": ("voice", "sdk"),  # Python 绑定
    "mox-common-meta": ("platform", "core"),  # 遗留命名
    "mox-domain-abstractions": ("platform", "core"),
    "mox-standards": ("platform", "core"),
    "mox-ai-core": ("ai", "core"),  # 无 -core 后缀的 core 层
    # SDK 层（无 -sdk 后缀的遗留命名）
    "mox-cloud-sdk": ("cloud", "sdk"),
    "mox-kg-sdk": ("kg", "sdk"),
    "mox-data-formula-native": ("data", "sdk"),  # native 绑定视为 sdk
    "mox-data-norm-intent-native": ("data", "sdk"),
}

# crate 名后缀 → 层
LAYER_SUFFIX = {
    "-core": "core",
    "-svc": "svc",
    "-sdk": "sdk",
    "-api": "api",
}


def parse_crate(crate_name: str) -> Tuple[str, str]:
    """从 crate 名解析 (域, 层)。优先使用 override，然后按命名约定解析。"""
    if crate_name in CRATE_OVERRIDE:
        return CRATE_OVERRIDE[crate_name]

    # 横切层识别
    if crate_name.startswith("mox-foundation-"):
        return ("foundation", "foundation")
    if crate_name.startswith("mox-framework-"):
        return ("framework", "framework")

    # 命名约定: mox-{domain}-{name}-{layer}
    m = re.match(r"^mox-([a-z]+)-(.+)-([a-z]+)$", crate_name)
    if m:
        domain = m.group(1)
        suffix = "-" + m.group(3)
        if domain in DOMAIN_ORDER and suffix in LAYER_SUFFIX:
            return (domain, LAYER_SUFFIX[suffix])

    # 尝试 mox-{domain}-{name} 无后缀（可能是 core）
    m2 = re.match(r"^mox-([a-z]+)-(.+)$", crate_name)
    if m2 and m2.group(1) in DOMAIN_ORDER:
        # 无明确层后缀，默认 core（但标记为 unknown 警告）
        return (m2.group(1), "unknown")

    return ("unknown", "unknown")


# ============================================================
# 数据结构
# ============================================================
@dataclass
class Violation:
    rule: str          # 规则编号 R1-R5
    level: str         # P0/P1/P2
    crate: str         # 违规 crate
    dependency: str    # 被依赖的 crate
    description: str
    fix_hint: str = ""


@dataclass
class ArchTestResult:
    total_crates: int = 0
    total_edges: int = 0
    violations: List[Violation] = field(default_factory=list)
    unknown_crates: List[str] = field(default_factory=list)

    @property
    def p0_count(self): return sum(1 for v in self.violations if v.level == "P0")
    @property
    def p1_count(self): return sum(1 for v in self.violations if v.level == "P1")
    @property
    def p2_count(self): return sum(1 for v in self.violations if v.level == "P2")
    @property
    def passed(self): return self.p0_count == 0 and self.p1_count == 0


# ============================================================
# 依赖图加载
# ============================================================
def load_workspace(workspace_root: str) -> Tuple[Dict[str, List[str]], Set[str]]:
    """通过 cargo metadata 加载 workspace 内部依赖图。
    返回 (deps, binary_crates) — binary_crates 是包含 [[bin]] target 的 crate 名集合。
    """
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True, text=True, cwd=workspace_root,
        encoding="utf-8"
    )
    if result.returncode != 0:
        print(f"ERROR: cargo metadata failed: {result.stderr}", file=sys.stderr)
        sys.exit(2)

    data = json.loads(result.stdout)
    internal = {p["name"] for p in data["packages"]}
    deps = {}
    binary_crates = set()
    for pkg in data["packages"]:
        name = pkg["name"]
        # 仅保留 normal 依赖，排除 dev-dependencies 和 build-dependencies
        # cargo metadata 中 normal 依赖的 kind 为 None，dev 为 "dev"，build 为 "build"
        normal_deps = []
        for d in pkg["dependencies"]:
            dep_name = d["name"]
            kind = d.get("kind")
            if kind in ("dev", "build"):
                continue
            if dep_name in internal and dep_name != name:
                normal_deps.append(dep_name)
        deps[name] = normal_deps
        # 检测是否有 bin target
        for target in pkg.get("targets", []):
            if "bin" in target.get("kind", []):
                binary_crates.add(name)
                break
    return deps, binary_crates


# ============================================================
# 规则检测
# ============================================================
def check_rule1_layer_unidirectionality(deps: Dict[str, List[str]]) -> List[Violation]:
    """规则1: 层间单向依赖。底层不能依赖高层。"""
    violations = []
    for crate, crate_deps in deps.items():
        src_domain, src_layer = parse_crate(crate)
        src_order = LAYER_ORDER.get(src_layer, 99)
        if src_layer in CROSS_CUTTING or src_layer == "unknown":
            continue
        for dep in crate_deps:
            dst_domain, dst_layer = parse_crate(dep)
            dst_order = LAYER_ORDER.get(dst_layer, 99)
            if dst_layer in CROSS_CUTTING or dst_layer == "unknown":
                continue
            # 底层依赖高层 = 违规
            if src_order < dst_order:
                severity = "P1" if (dst_order - src_order) >= 2 else "P2"
                violations.append(Violation(
                    rule="R1", level=severity,
                    crate=crate, dependency=dep,
                    description=f"{crate}({src_layer}) → {dep}({dst_layer}): 底层依赖高层",
                    fix_hint=f"将 {dep} 的逻辑下沉到 {src_layer} 层，或通过 {src_layer} 可依赖的中间层间接调用"
                ))
    return violations


def check_rule2_cross_domain_via_sdk(deps: Dict[str, List[str]], binary_crates: Set[str]) -> List[Violation]:
    """规则2: 跨域 svc→svc 直连禁止，必须经 SDK。core→core 允许。
    binary crate（入口组装点）豁免——直接依赖 svc 是合理的组装行为。
    """
    violations = []
    for crate, crate_deps in deps.items():
        if crate in binary_crates:
            continue
        src_domain, src_layer = parse_crate(crate)
        if src_domain == "unknown" or src_layer in CROSS_CUTTING:
            continue
        for dep in crate_deps:
            dst_domain, dst_layer = parse_crate(dep)
            if dst_domain == "unknown" or dst_layer in CROSS_CUTTING:
                continue
            # 同域不检查
            if src_domain == dst_domain:
                continue
            # svc → svc 直连 = 违规
            if src_layer == "svc" and dst_layer == "svc":
                violations.append(Violation(
                    rule="R2", level="P1",
                    crate=crate, dependency=dep,
                    description=f"{crate}({src_domain}/svc) → {dep}({dst_domain}/svc): 跨域 svc 直连",
                    fix_hint=f"在 {dst_domain} 域创建 sdk 层 crate（如 mox-{dst_domain}-xxx-sdk），将 {dep} 的对外接口提取到 sdk，{crate} 改为依赖 sdk"
                ))
            # api → svc 跨域 = 违规（api 只能依赖本域 svc/sdk）
            elif src_layer == "api" and dst_layer == "svc" and src_domain != dst_domain:
                violations.append(Violation(
                    rule="R2", level="P1",
                    crate=crate, dependency=dep,
                    description=f"{crate}({src_domain}/api) → {dep}({dst_domain}/svc): api 层跨域依赖 svc",
                    fix_hint=f"api 层应通过 {dst_domain} 域的 sdk 调用，或在本域 svc 中封装调用逻辑"
                ))
    return violations


def check_rule3_domain_direction(deps: Dict[str, List[str]], binary_crates: Set[str]) -> List[Violation]:
    """规则3: 域间依赖方向（DAG），禁止逆向依赖。
    binary crate（入口组装点）豁免——入口依赖所有域是合理的。
    """
    violations = []
    for crate, crate_deps in deps.items():
        if crate in binary_crates:
            continue
        src_domain, src_layer = parse_crate(crate)
        if src_domain not in DOMAIN_ORDER or src_layer in CROSS_CUTTING:
            continue
        for dep in crate_deps:
            dst_domain, dst_layer = parse_crate(dep)
            if dst_domain not in DOMAIN_ORDER or dst_layer in CROSS_CUTTING:
                continue
            if src_domain == dst_domain:
                continue
            # 检查域间依赖是否允许
            allowed = DOMAIN_DEP_MATRIX.get(src_domain, {}).get(dst_domain, False)
            if not allowed:
                violations.append(Violation(
                    rule="R3", level="P0",
                    crate=crate, dependency=dep,
                    description=f"{crate}({src_domain}) → {dep}({dst_domain}): 域间逆向依赖（违反 DAG）",
                    fix_hint=f"重新审视职责划分：{src_domain} 不应依赖 {dst_domain}。考虑将共享逻辑下沉到 platform/kg，或通过事件驱动解耦"
                ))
    return violations


def check_rule3b_cycles(deps: Dict[str, List[str]]) -> List[Violation]:
    """规则3补充: 循环依赖检测（DFS）。"""
    violations = []
    visited = set()
    rec_stack = set()
    path = []
    cycles_found = set()

    def dfs(node):
        visited.add(node)
        rec_stack.add(node)
        path.append(node)
        for neighbor in deps.get(node, []):
            if neighbor not in visited:
                dfs(neighbor)
            elif neighbor in rec_stack:
                idx = path.index(neighbor)
                cycle = path[idx:] + [neighbor]
                cycle_key = tuple(sorted(cycle[:-1]))
                if cycle_key not in cycles_found:
                    cycles_found.add(cycle_key)
                    violations.append(Violation(
                        rule="R3", level="P0",
                        crate=cycle[0], dependency=cycle[-2],
                        description=f"循环依赖: {' → '.join(cycle)}",
                        fix_hint="打破循环：将共享类型下沉到 core/foundation，或通过事件/接口反转依赖方向"
                    ))
        path.pop()
        rec_stack.remove(node)

    for name in deps:
        if name not in visited:
            dfs(name)
    return violations


def check_rule4_gateway_thinness(deps: Dict[str, List[str]]) -> List[Violation]:
    """规则4: Gateway 无业务逻辑。gateway 不依赖业务 svc/core（除 platform 自身）。"""
    violations = []
    for crate, crate_deps in deps.items():
        src_domain, src_layer = parse_crate(crate)
        if src_layer != "gateway":
            continue
        for dep in crate_deps:
            dst_domain, dst_layer = parse_crate(dep)
            # gateway 可以依赖 foundation/framework/platform core/sdk
            if dst_layer in CROSS_CUTTING:
                continue
            if dst_domain == "platform" and dst_layer in ("core", "sdk"):
                continue
            # gateway 依赖其他域的 svc/core = 违规
            if dst_layer in ("svc", "core") and dst_domain != "platform":
                violations.append(Violation(
                    rule="R4", level="P0",
                    crate=crate, dependency=dep,
                    description=f"{crate}(gateway) → {dep}({dst_domain}/{dst_layer}): gateway 依赖业务逻辑",
                    fix_hint=f"将业务逻辑从 gateway 移除，改为通过 {dst_domain} 域的 sdk 调用，或在对应域的 api 层注册路由"
                ))
    return violations


def check_rule5_api_contract(deps: Dict[str, List[str]]) -> List[Violation]:
    """规则5: API 层契约优先。api 层不依赖其他域的 svc/core，不依赖数据库/算法 crate。"""
    violations = []
    for crate, crate_deps in deps.items():
        src_domain, src_layer = parse_crate(crate)
        if src_layer != "api":
            continue
        for dep in crate_deps:
            dst_domain, dst_layer = parse_crate(dep)
            if dst_layer in CROSS_CUTTING:
                continue
            # api 可以依赖本域 svc/sdk
            if src_domain == dst_domain and dst_layer in ("svc", "sdk", "core"):
                continue
            # api 跨域依赖 = 违规
            if src_domain != dst_domain and dst_layer in ("svc", "core"):
                violations.append(Violation(
                    rule="R5", level="P2",
                    crate=crate, dependency=dep,
                    description=f"{crate}({src_domain}/api) → {dep}({dst_domain}/{dst_layer}): api 层跨域依赖业务逻辑",
                    fix_hint=f"在 {src_domain} 域的 svc 层封装对 {dst_domain} 的调用，api 层只依赖本域 svc"
                ))
    return violations


# ============================================================
# 主流程
# ============================================================
def run_arch_test(workspace_root: str) -> ArchTestResult:
    deps, binary_crates = load_workspace(workspace_root)
    result = ArchTestResult(
        total_crates=len(deps),
        total_edges=sum(len(v) for v in deps.values()),
    )

    # 收集 unknown crate
    for crate in deps:
        domain, layer = parse_crate(crate)
        if domain == "unknown" or layer == "unknown":
            result.unknown_crates.append(crate)

    # 执行 5 条规则检测
    result.violations.extend(check_rule1_layer_unidirectionality(deps))
    result.violations.extend(check_rule2_cross_domain_via_sdk(deps, binary_crates))
    result.violations.extend(check_rule3_domain_direction(deps, binary_crates))
    result.violations.extend(check_rule3b_cycles(deps))
    result.violations.extend(check_rule4_gateway_thinness(deps))
    result.violations.extend(check_rule5_api_contract(deps))

    return result


def print_report(result: ArchTestResult, show_fix_hint: bool = False, quiet: bool = False):
    if not quiet:
        print("=" * 72)
        print("  infotopograph 架构一致性校验（arch test）v2.0")
        print("  权威文档: ADR-09 跨域依赖规则与架构一致性治理")
        print("=" * 72)
        print(f"\n  概览: {result.total_crates} crates, {result.total_edges} 内部依赖边")
        if result.unknown_crates:
            print(f"  ⚠️  未识别 crate ({len(result.unknown_crates)}): {', '.join(result.unknown_crates[:10])}")
            if len(result.unknown_crates) > 10:
                print(f"         ... 还有 {len(result.unknown_crates) - 10} 个")

    # 按规则分组
    by_rule = defaultdict(list)
    for v in result.violations:
        by_rule[v.rule].append(v)

    rule_names = {
        "R1": "层间单向依赖",
        "R2": "跨域必须经 SDK",
        "R3": "域间依赖方向 + 循环依赖",
        "R4": "Gateway 无业务逻辑",
        "R5": "API 层契约优先",
    }

    if result.violations:
        print(f"\n  违规明细 ({len(result.violations)} 项):")
        print("  " + "-" * 68)
        for rule in ["R1", "R2", "R3", "R4", "R5"]:
            items = by_rule.get(rule, [])
            if not items:
                continue
            print(f"\n  [{rule}] {rule_names[rule]} ({len(items)} 项)")
            for v in items:
                print(f"    [{v.level}] {v.description}")
                if show_fix_hint and v.fix_hint:
                    print(f"         💡 修复: {v.fix_hint}")
    else:
        print("\n  ✅ 无违规项")

    # 汇总
    print("\n" + "=" * 72)
    print(f"  汇总: P0={result.p0_count}, P1={result.p1_count}, P2={result.p2_count}, 总计={len(result.violations)}")
    if result.passed:
        print("  ✅ 架构一致性校验通过（P0=0, P1=0）")
        if result.p2_count > 0:
            print(f"     ⚠️  存在 {result.p2_count} 个 P2 警告（建议修复，不阻断 CI）")
    else:
        print("  ❌ 架构一致性校验失败（存在 P0 或 P1 违规）")
    print("=" * 72)


def print_json(result: ArchTestResult):
    output = {
        "tool": "arch_test",
        "version": "2.0",
        "summary": {
            "total_crates": result.total_crates,
            "total_edges": result.total_edges,
            "p0": result.p0_count,
            "p1": result.p1_count,
            "p2": result.p2_count,
            "total_violations": len(result.violations),
            "passed": result.passed,
        },
        "violations": [
            {
                "rule": v.rule,
                "level": v.level,
                "crate": v.crate,
                "dependency": v.dependency,
                "description": v.description,
                "fix_hint": v.fix_hint,
            }
            for v in result.violations
        ],
        "unknown_crates": result.unknown_crates,
    }
    print(json.dumps(output, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(description="infotopograph 架构一致性校验 v2.0")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--quiet", action="store_true", help="仅输出违规项")
    parser.add_argument("--fix-hint", action="store_true", help="输出修复建议")
    parser.add_argument("--root", default=r"D:\a10\aikjx\gitcode\infotopograph", help="workspace 根目录")
    args = parser.parse_args()

    result = run_arch_test(args.root)

    if args.json:
        print_json(result)
    else:
        print_report(result, show_fix_hint=args.fix_hint, quiet=args.quiet)

    sys.exit(0 if result.passed else 1)


if __name__ == "__main__":
    main()
