"""Archive final checks and exact source hashes without hiding failed attempts."""
import hashlib
import json
import re
from datetime import datetime, timezone
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
names = [
    "rust_final", "iam_core_tests", "clippy_final", "frontend_build",
    "frontend_contract", "frontend_pagination_clean", "frontend_module_complete",
    "locale_final", "registry", "doc_links_final", "ports", "scoped_format",
    "scoped_diff_final",
]
checks = {name: json.loads((folder / f"{name}.json").read_text(encoding="utf-8")) for name in names}
assert all(check["exit_code"] == 0 for check in checks.values()), checks
def log(name):
    return (folder / f"{name}.log").read_bytes().decode("utf-8", errors="replace")
rust_passed = sum(int(n) for name in ["rust_final", "iam_core_tests"]
                  for n in re.findall(r"test result: ok\. (\d+) passed", log(name)))
assert rust_passed == 24, rust_passed
assert re.search(r"\bpass 7\b", log("frontend_contract"))
assert re.search(r"Tests\s+2 passed", log("frontend_pagination_clean"))
attempts = {}
for path in sorted(folder.glob("*.json")):
    if path.stem in names or path.stem in ["summary", "source-manifest"]:
        continue
    value = json.loads(path.read_text(encoding="utf-8"))
    if "exit_code" in value:
        attempts[path.stem] = value
summary = {
    "status": "completed_for_documented_scope",
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "checks": checks,
    "final_checks_passed": len(checks),
    "rust_tests_passed": rust_passed,
    "frontend_contract_tests_passed": 7,
    "actual_component_tests_passed": 2,
    "registry_tests_passed": 1,
    "prior_attempts_retained": attempts,
    "warnings_observed": {"ports": 284, "doc_links": 216, "frontend_module_comments": 26},
    "business_service_mocks_added": False,
    "component_stubs_added": False,
    "full_enterprise_ready": False,
    "browser_full_e2e_verified": False,
    "cross_host_verified": False,
    "large_scale_performance_verified": False,
    "whole_history_audit_integrity_verified": False,
    "external_audit_clients_require_pagination_migration": True,
}
(folder / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
paths = [
    "platform/domains/platform/core/mox-platform-iam-core/src/audit_query.rs",
    "platform/domains/platform/core/mox-platform-iam-core/src/lib.rs",
    "platform/domains/platform/core/mox-platform-iam-core/src/ddl.sql",
    "platform/gateway/mox-platform-gateway-svc/src/system/security.rs",
    "platform/gateway/mox-platform-gateway-svc/src/alliance/experts_streams.rs",
    "platform/gateway/mox-platform-gateway-svc/tests/inbox_http.rs",
    "frontend-ui/src/api/system.api.js",
    "frontend-ui/src/utils/api-key.js",
    "frontend-ui/src/utils/index.js",
    "frontend-ui/src/utils/audit.node-test.mjs",
    "frontend-ui/src/components/common/DataTable.vue",
    "frontend-ui/src/components/common/DataTable.pagination.test.js",
    "frontend-ui/src/modules/admin-lowcode/pages/audit.page.js",
    "frontend-ui/src/modules/admin-lowcode/index.js",
    "frontend-ui/src/modules/admin-lowcode/contract/endpoints.js",
    "frontend-ui/src/modules/admin-lowcode/contract/pageSchema.js",
    "frontend-ui/src/modules/admin-lowcode/composables/useCrudPage.js",
    "frontend-ui/src/modules/admin-lowcode/engine/SchemaCrudPage.vue",
    "frontend-ui/src/views/admin/panels/AdminAudit.vue",
    "docs/modules/iam/README.md",
    "docs/modules/iam/API-KEYS.md",
    "docs/modules/iam/AUDIT.md",
    "docs/modules/REAL-IMPLEMENTATION-STATUS.md",
    "docs/architecture/frontend/LOWCODE-PAGE-RUNTIME.md",
    "reports/markdown/20261002-iam-audit-query.md",
]
manifest = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in paths}
(folder / "source-manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
print(f"final_checks={len(checks)} rust_tests={rust_passed} frontend_tests=9 source_files={len(manifest)}")
