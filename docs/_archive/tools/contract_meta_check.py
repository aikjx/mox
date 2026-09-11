#!/usr/bin/env python3
"""Static verification of CRATE_ID / ENGINE_NAME / CRATE_META contract declarations.

Replaces the former workspace-wide Rust test (`_tmp_t2_crate_meta`), which had to
`extern crate` 16 members and therefore forced the orchestrator to declare them as
dev-dependencies purely for testing. Verifying the declarations statically keeps the
same guarantees without turning a test harness into a fan-out hotspot.

Checks per crate: CRATE_ID is UUIDv5, ENGINE_NAME matches, CRATE_META.id aliases
CRATE_ID, name resolves to the package name, owner is declared, layer matches.
Across crates: CRATE_ID and ENGINE_NAME are unique and the AIS layer census holds.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# crate -> (ENGINE_NAME, AisLayer). Layer census derived from this table.
EXPECTED = {
    "mox-flow-operator-core": ("mox::operator_core", "L6Kernel"),
    "mox-flow-operator-wasm-svc": ("mox::operator_wasm", "L4Services"),
    "mox-kg-algo-core": ("mox::graph_algorithms", "L4Services"),
    "mox-flow-optimizer-core": ("mox::optimizer", "L4Services"),
    "mox-ai-flow-svc": ("mox::mox_ai_flow_svc", "L4Services"),
    "mox-ai-expert-svc": ("mox::mox_expert", "L4Services"),
    "mox-flow-bridge-svc": ("mox::hermes_flow_bridge", "L4Services"),
    "mox-data-catalog-svc": ("mox::business_catalog", "L4Services"),
    "mox-ai-agent-svc": ("mox::ai_agent", "L4Services"),
    "mox-market-template-svc": ("mox::template_market", "L4Services"),
    "mox-platform-orchestrator-svc": ("mox::mox_platform_orchestrator_svc", "L3Orchestration"),
    "mox-platform-system-core": ("mox::mox_system", "L7Infrastructure"),
    "mox-flow-primiflow-svc": ("mox::primiflow_core", "L4Services"),
    "mox-flow-fusion-svc": ("mox::primiflow_fusion", "L4Services"),
    "mox-kg-hub-svc": ("mox::kg_hub", "L4Services"),
    "mox-platform-foundation": ("mox::mox_common_meta", "L5Domain"),
}

EXPECTED_OWNER = "mox-core"

RE_CRATE_ID = re.compile(r'pub const CRATE_ID:\s*&str\s*=\s*"([^"]+)"')
RE_ENGINE = re.compile(r'pub const ENGINE_NAME:\s*&str\s*=\s*"([^"]+)"')
RE_META = re.compile(r'pub const CRATE_META:.*?\{(.*?)\n\};', re.S)
RE_META_ID = re.compile(r'\bid:\s*([A-Za-z_][A-Za-z0-9_]*)\s*,')
RE_META_NAME = re.compile(r'\bname:\s*(.+?)\s*,')
RE_META_LAYER = re.compile(r'\blayer:\s*.*?AisLayer::(\w+)')
RE_META_OWNER = re.compile(r'\bowner:\s*"([^"]*)"')


def is_uuid_v5(value):
    if len(value) != 36:
        return False
    for index, char in enumerate(value):
        if index in (8, 13, 18, 23):
            if char != "-":
                return False
        elif char not in "0123456789abcdefABCDEF":
            return False
    parts = value.split("-")
    return len(parts) == 5 and parts[2][:1] == "5"


def load_packages():
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True, text=True, encoding="utf-8", check=True, cwd=ROOT,
    )
    data = json.loads(result.stdout)
    return {p["name"]: Path(p["manifest_path"]).parent for p in data["packages"]}


def find_lib(crate_dir):
    """Locate the file declaring the contract constants.

    Most crates declare them in `lib.rs`, but several place them in a dedicated
    `constants.rs`, so the whole source tree is searched rather than one path.
    """
    src = crate_dir / "src"
    if not src.is_dir():
        return None
    for candidate in sorted(src.rglob("*.rs")):
        try:
            if "pub const CRATE_META" in candidate.read_text(encoding="utf-8-sig", errors="replace"):
                return candidate
        except OSError:
            continue
    return None


def parse(path):
    source = path.read_text(encoding="utf-8-sig", errors="replace")
    meta_block = RE_META.search(source)
    meta = {}
    if meta_block:
        block = meta_block.group(1)
        meta["id"] = (RE_META_ID.search(block).group(1) if RE_META_ID.search(block) else None)
        name = RE_META_NAME.search(block)
        meta["name"] = name.group(1) if name else None
        layer = RE_META_LAYER.search(block)
        meta["layer"] = layer.group(1) if layer else None
        owner = RE_META_OWNER.search(block)
        meta["owner"] = owner.group(1) if owner else None
    crate_id = RE_CRATE_ID.search(source)
    engine = RE_ENGINE.search(source)
    return {
        "crate_id": crate_id.group(1) if crate_id else None,
        "engine_name": engine.group(1) if engine else None,
        "meta": meta,
    }


def check():
    packages = load_packages()
    findings = []
    seen_ids = {}
    seen_engines = {}
    layers = []

    for crate, (engine_expected, layer_expected) in sorted(EXPECTED.items()):
        crate_dir = packages.get(crate)
        if crate_dir is None:
            findings.append((crate, "crate not present in workspace"))
            continue
        lib = find_lib(crate_dir)
        if lib is None:
            findings.append((crate, "no src/lib.rs or src/main.rs found"))
            continue
        info = parse(lib)
        meta = info["meta"]

        if info["crate_id"] is None:
            findings.append((crate, "CRATE_ID not declared"))
        elif not is_uuid_v5(info["crate_id"]):
            findings.append((crate, "CRATE_ID is not UUIDv5: %s" % info["crate_id"]))
        else:
            seen_ids.setdefault(info["crate_id"], []).append(crate)

        if info["engine_name"] is None:
            findings.append((crate, "ENGINE_NAME not declared"))
        elif info["engine_name"] != engine_expected:
            findings.append((crate, "ENGINE_NAME is %r, expected %r"
                             % (info["engine_name"], engine_expected)))
        else:
            seen_engines.setdefault(info["engine_name"], []).append(crate)

        if not meta:
            findings.append((crate, "CRATE_META not declared"))
            continue
        if meta["id"] != "CRATE_ID":
            findings.append((crate, "CRATE_META.id does not alias CRATE_ID: %r" % meta["id"]))
        if meta["name"] != 'env!("CARGO_PKG_NAME")':
            findings.append((crate, "CRATE_META.name is %s, expected env!(\"CARGO_PKG_NAME\")"
                             % meta["name"]))
        if meta["owner"] != EXPECTED_OWNER:
            findings.append((crate, "CRATE_META.owner is %r, expected %r"
                             % (meta["owner"], EXPECTED_OWNER)))
        if meta["layer"] != layer_expected:
            findings.append((crate, "CRATE_META.layer is %r, expected %r"
                             % (meta["layer"], layer_expected)))
        else:
            layers.append(layer_expected)

    for value, owners in seen_ids.items():
        if len(owners) > 1:
            findings.append(("<global>", "CRATE_ID duplicated across %s" % ", ".join(owners)))
    for value, owners in seen_engines.items():
        if len(owners) > 1:
            findings.append(("<global>", "ENGINE_NAME duplicated across %s" % ", ".join(owners)))

    # Crates outside the contract table keep their legacy identifiers. They are reported
    # as warnings, not failures: tightening them is a separate migration, and silently
    # passing them would hide the drift while silently failing would block unrelated work.
    warnings = []
    for crate, crate_dir in sorted(packages.items()):
        if crate in EXPECTED:
            continue
        src = crate_dir / "src"
        if not src.is_dir():
            continue
        for candidate in sorted(src.rglob("*.rs")):
            try:
                text = candidate.read_text(encoding="utf-8-sig", errors="replace")
            except OSError:
                continue
            found = RE_CRATE_ID.search(text)
            if found and not is_uuid_v5(found.group(1)):
                warnings.append((crate, "CRATE_ID is not UUIDv5: %s" % found.group(1)))

    census = {}
    for layer in layers:
        census[layer] = census.get(layer, 0) + 1
    expected_census = {}
    for _, layer in EXPECTED.values():
        expected_census[layer] = expected_census.get(layer, 0) + 1
    if census != expected_census:
        findings.append(("<global>", "AIS layer census %s does not match expected %s"
                         % (census, expected_census)))
    return findings, warnings


def main():
    findings, warnings = check()
    for crate, message in warnings:
        print("  ! %s: %s" % (crate, message))
    if not findings:
        print("Contract meta check: PASS (%d crates verified, %d legacy warnings)"
              % (len(EXPECTED), len(warnings)))
        return 0
    print("Contract meta check: FAIL (%d findings)" % len(findings))
    for crate, message in findings:
        print("  - %s: %s" % (crate, message))
    return 1


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    sys.exit(main())
