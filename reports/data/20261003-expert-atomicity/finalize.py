"""Summarize captured checks without replacing failed intermediate evidence."""
import hashlib
import json
import re
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
names = ["rust_verified", "contract_final", "clippy", "diagrams",
         "paths_selftest", "paths", "docs_final", "ports", "diff", "format",
         "frontend_module", "api_bindings"]
checks = {name: json.loads((folder / (name + ".json")).read_text(encoding="utf-8"))
          for name in names}
assert all(item["exit_code"] == 0 for item in checks.values()), checks
rust = (folder / "rust_verified.log").read_text(encoding="utf-8", errors="replace")
counts = [int(n) for n in re.findall(r"test result: ok\. (\d+) passed", rust)]
assert counts == [190, 1, 7, 1, 2, 2, 1, 1, 1], counts
front = (folder / "contract_final.log").read_text(encoding="utf-8", errors="replace")
assert "157 passed" in front
paths = [
    "platform/gateway/mox-platform-gateway-svc/src/alliance/experts_db.rs",
    "platform/gateway/mox-platform-gateway-svc/src/alliance/experts_registry.rs",
    "platform/gateway/mox-platform-gateway-svc/tests/expert_mutation_atomicity.rs",
    "frontend-ui/src/modules/expert-alliance/contract/contract.test.js",
    "docs/expert-alliance/21-event-delivery-contract.md",
    "docs/expert-alliance/22-expert-object-contract.md",
    "docs/expert-alliance/CURRENT-ARCHITECTURE.md",
    "docs/expert-alliance/16-decision-and-state-ledger.md",
    "docs/expert-alliance/INDEX.md",
    "docs/specifications/tasks/20261002-enterprise-module-normalization/todo.md",
]
summary = {
    "scope": "gateway library and eight selected real integration targets; frontend contract tests",
    "checks": checks,
    "rust_passed": sum(counts), "rust_library_passed": counts[0],
    "rust_integration_passed": sum(counts[1:]), "frontend_contract_passed": 157,
    "mermaid_diagrams": 40,
    "original_failure": {"log": "red.log", "cargo_exit_code": 101,
                         "expected_http": 503, "actual_http": 200},
    "intermediate_failures_retained": ["green.log", "registry_unit.log", "contract.log", "docs.log"],
    "warnings": {"clippy_existing_library": 69, "doc_historical_backtick_paths": 206,
                 "ports": 284, "frontend_comment_pairing": 26},
    "source_sha256": {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in paths},
    "not_accepted": ["reliable outbox delivery", "crash replay", "cross-process CAS",
                     "whole workspace", "production SLO", "real OSS", "browser end-to-end"],
}
(folder / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print("Checks:", len(checks), "Rust:", sum(counts), "Frontend:", 157)
