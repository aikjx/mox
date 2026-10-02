"""Record exact checks and source hashes for this documented increment."""
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
names = ["catalog_ci_tests", "capabilities_final", "diagram_parse", "code_catalog",
         "rust_tests_final", "frontend_contract", "frontend_components", "frontend_build",
         "clippy_final", "frontend_module", "locale", "api_bindings", "feedback_imports",
         "framework_imports", "theme_tokens", "formula_ci_final", "surface_selftest_utf8",
         "paths_selftest", "paths_ci_verified", "ports", "doc_links_release",
         "scoped_format", "scoped_diff_ci_final"]
checks = {name: json.loads((folder / (name + ".json")).read_text(encoding="utf-8")) for name in names}
assert all(check["exit_code"] == 0 for check in checks.values()), checks

def log(name):
    return (folder / (name + ".log")).read_bytes().decode("utf-8", errors="replace")

rust_count = sum(int(n) for n in re.findall(r"test result: ok\. (\d+) passed", log("rust_tests_final")))
assert rust_count == 21, rust_count
assert re.search(r"\bpass 12\b", log("frontend_contract"))
assert re.search(r"Tests\s+2 passed", log("frontend_components"))
assert "Ran 6 tests" in log("catalog_ci_tests")
assert "Parsed 35 diagrams" in log("diagram_parse")
registry = json.loads((root / "docs/modules/enterprise-capabilities/registry.json").read_text(encoding="utf-8"))
prior = {}
for path in sorted(folder.glob("*.json")):
    if path.stem in names or path.stem in ["summary", "source-manifest"]:
        continue
    value = json.loads(path.read_text(encoding="utf-8"))
    if "exit_code" in value:
        prior[path.stem] = value
summary = {
    "status": "completed_for_documented_increment",
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "module_count": len(registry["modules"]),
    "explicit_requirement_count": sum(len(r["requirements"]) for r in registry["modules"]),
    "mermaid_diagrams_parsed": 35,
    "checks": checks, "final_checks_passed": len(checks),
    "rust_real_storage_http_tests_passed": rust_count,
    "node_contract_tests_passed": 12, "actual_component_tests_passed": 2,
    "registry_invariant_tests_passed": 6,
    "prior_attempts_retained": prior,
    "business_service_mocks_added": False,
    "component_stubs_added": False,
    "all_modules_functionally_complete": False,
    "full_enterprise_ready": False,
    "browser_full_e2e_verified": False,
    "remote_ci_executed": False,
    "cross_host_verified": False,
    "external_oss_model_success_verified": False,
    "cross_page_send_recovery_implemented": False,
}
(folder / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
paths = [".github/workflows/ci.yml", "docs/README.md", "docs/CORE-CAPABILITIES.md",
         "docs/modules/README.md", "docs/modules/CODE-CATALOG.md",
         "docs/modules/REAL-IMPLEMENTATION-STATUS.md", "docs/modules/message-center/README.md",
         "scripts/registry/enterprise_capabilities.py", "scripts/registry/module_catalog.py",
         "scripts/tests/test_enterprise_capabilities.py", "frontend-ui/src/api/index.js",
         "frontend-ui/src/api/message.api.js", "frontend-ui/src/modules/index.js",
         "platform/gateway/mox-platform-gateway-svc/src/message_center/api.rs",
         "platform/gateway/mox-platform-gateway-svc/tests/inbox_http.rs",
         "reports/markdown/20261002-enterprise-capabilities.md"]
for directory in ["docs/modules/enterprise-capabilities",
                  "docs/specifications/tasks/20261002-enterprise-module-normalization",
                  "frontend-ui/src/modules/message-center"]:
    paths.extend(p.relative_to(root).as_posix() for p in (root / directory).rglob("*") if p.is_file())
manifest = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in sorted(set(paths))}
(folder / "source-manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
print(f"checks={len(checks)} modules={len(registry['modules'])} requirements={summary['explicit_requirement_count']} rust={rust_count} frontend=14 registry_tests=6 sources={len(manifest)}")
