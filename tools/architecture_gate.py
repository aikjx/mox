#!/usr/bin/env python3
"""One release decision across both existing architecture policies, without relaxing either."""
import argparse
import json
import sys
from dataclasses import asdict
from pathlib import Path

import arch_test
import architecture_constraint_test as constraints


def combine(stages):
    findings = []
    for name, result in stages.items():
        findings.extend(dict(item, checker=name) for item in result["findings"])
    counts = {level: sum(item["level"] == level for item in findings) for level in ("P0", "P1", "P2")}
    return {"passed": not (counts["P0"] or counts["P1"]), "counts": counts,
            "stages": stages, "findings": findings}


def run():
    root = Path(__file__).resolve().parents[1]
    normal = arch_test.run_arch_test(str(root))
    deps = constraints.load_workspace()
    violations = [constraints.Violation("P0", "cycle", " -> ".join(c)) for c in constraints.detect_cycles(deps)]
    violations += constraints.detect_layer_violations(deps)
    violations += constraints.detect_god_modules(deps)
    cross, _ = constraints.detect_cross_domain(deps)
    # Always report individual cross-domain findings, including those below the aggregate warning threshold.
    violations += cross
    return combine({
        "normal_dependencies": {"crates": normal.total_crates, "edges": normal.total_edges,
            "unknown": normal.unknown_crates, "findings": [asdict(v) for v in normal.violations]},
        "declared_dependencies": {"crates": len(deps), "edges": sum(map(len, deps.values())),
            "unknown": sorted(k for k, v in constraints.CRATE_LAYERS.items() if v == "unknown"),
            "findings": [asdict(v) for v in violations]},
    })


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit the complete machine-readable report")
    parser.add_argument("--output", type=Path, help="also save the report as UTF-8 JSON")
    args = parser.parse_args()
    report = run()  # Metadata/checker failures intentionally do not become a passing report.
    payload = json.dumps(report, ensure_ascii=False, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload + "\n", encoding="utf-8")
    if args.json:
        print(payload)
    else:
        print("Architecture gate: " + ("PASS" if report["passed"] else "FAIL"))
        print(" ".join(f"{k}={v}" for k, v in report["counts"].items()))
        for item in report["findings"]:
            print(f'[{item["level"]}] {item["checker"]}: {item["description"]}')
        for name, stage in report["stages"].items():
            if stage["unknown"]:
                print(f'{name} unclassified: {", ".join(stage["unknown"])}')
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
