#!/usr/bin/env python3
"""解析 `cargo test --workspace` 日志，输出每个测试二进制与每个 crate 的通过/失败/忽略数量。

用法:
    python scripts/parse_test_report.py logs/cargo_test_20260818.log

输出三部分:
  1. 每个测试二进制（harness）的明细
  2. 按 crate 归并的汇总
  3. 全量合计，并对 "failed != 0" 给出非零退出码，可直接用作 CI 卡点
"""
import collections
import re
import sys

# cargo 输出示例:
#      Running unittests src\lib.rs (target\debug\deps\ai_agent-dc674b8e0fc2a62c.exe)
#      Running tests\caomei_e2e.rs (target\debug\deps\caomei_e2e-3a8d96693bf9f0d9.exe)
RE_RUNNING = re.compile(
    r"Running\s+(?:unittests\s+)?(?P<src>\S+)\s+\(target[/\\]debug[/\\]deps[/\\](?P<bin>[A-Za-z0-9_]+)-[0-9a-f]+\.exe\)"
)
RE_DOCTEST = re.compile(r"^\s*Doc-tests\s+(?P<crate>[A-Za-z0-9_]+)\s*$")
RE_RESULT = re.compile(
    r"^test result:\s+(?P<status>ok|FAILED)\.\s+(?P<passed>\d+)\s+passed;\s+(?P<failed>\d+)\s+failed;\s+(?P<ignored>\d+)\s+ignored"
)

# 测试二进制名 -> 所属 crate（bin/集成测试的二进制名不等于 crate 名，需显式归属）
BIN_TO_CRATE = {
    "catalog": "business_catalog",
    "flowopt": "flow_ai",
    "operator_server": "runtime",
    "xuanji_system": "xuanji_system",
}


def crate_of(bin_name: str, src: str) -> str:
    """由测试二进制名与源文件路径推断所属 crate。"""
    if bin_name in BIN_TO_CRATE:
        return BIN_TO_CRATE[bin_name]
    # unittests src\lib.rs / src\main.rs -> 二进制名即 crate 名
    if src.replace("\\", "/").startswith("src/"):
        return bin_name
    # 集成测试 tests\xxx.rs -> 无法由名字推断，归入 UNMAPPED 由调用方补录
    return "(integration)"


def main(path: str) -> int:
    with open(path, encoding="utf-8", errors="replace") as fh:
        lines = fh.read().splitlines()

    current = None  # (label, crate)
    per_harness = collections.OrderedDict()
    for line in lines:
        m = RE_RUNNING.search(line)
        if m:
            bin_name, src = m.group("bin"), m.group("src")
            label = f"{bin_name} [{src}]"
            current = (label, crate_of(bin_name, src))
            continue
        m = RE_DOCTEST.match(line)
        if m:
            current = (f"{m.group('crate')} (doc-tests)", m.group("crate"))
            continue
        m = RE_RESULT.match(line)
        if m and current:
            label, crate = current
            slot = per_harness.setdefault(label, {"crate": crate, "p": 0, "f": 0, "i": 0})
            slot["p"] += int(m.group("passed"))
            slot["f"] += int(m.group("failed"))
            slot["i"] += int(m.group("ignored"))

    print(f"{'测试二进制 (harness)':<52}{'通过':>6}{'失败':>6}{'忽略':>6}")
    print("-" * 72)
    for label, v in per_harness.items():
        if v["p"] or v["f"] or v["i"]:
            print(f"{label:<52}{v['p']:>6}{v['f']:>6}{v['i']:>6}")

    per_crate = collections.defaultdict(lambda: {"p": 0, "f": 0, "i": 0})
    for v in per_harness.values():
        c = per_crate[v["crate"]]
        c["p"] += v["p"]
        c["f"] += v["f"]
        c["i"] += v["i"]

    print()
    print(f"{'crate 归并':<52}{'通过':>6}{'失败':>6}{'忽略':>6}")
    print("-" * 72)
    for crate, v in sorted(per_crate.items(), key=lambda kv: -kv[1]["p"]):
        print(f"{crate:<52}{v['p']:>6}{v['f']:>6}{v['i']:>6}")

    tp = sum(v["p"] for v in per_harness.values())
    tf = sum(v["f"] for v in per_harness.values())
    ti = sum(v["i"] for v in per_harness.values())
    nonempty = sum(1 for v in per_harness.values() if v["p"] or v["f"] or v["i"])
    print()
    print("-" * 72)
    print(f"合计: {tp} passed / {tf} failed / {ti} ignored")
    print(f"测试二进制总数: {len(per_harness)}（其中有用例的 {nonempty} 个）")
    return 1 if tf else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
