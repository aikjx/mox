"""Summarize final captured commands and keep original failure evidence."""
import hashlib
import json
import re
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
names = ["rust_verified", "contract_final", "node_transport", "frontend_build",
         "clippy", "catalog", "diagrams", "paths_selftest", "paths", "docs",
         "ports", "frontend_module", "diff", "format", "api_bindings",
         "framework_imports", "feedback_imports"]
checks = {name: json.loads((folder / (name + ".json")).read_text(encoding="utf-8")) for name in names}
assert all(item["exit_code"] == 0 for item in checks.values()), checks
rust = (folder / "rust_verified.log").read_text(encoding="utf-8", errors="replace")
counts = [int(n) for n in re.findall(r"test result: ok\. (\d+) passed", rust)]
assert counts == [191, 1, 1, 1, 7, 1, 2, 2, 1, 1, 1], counts
assert "REAL_FRONTEND_RESUME: replay frame, 410 recovery signal, invalid cursor rejection passed" in rust
front = (folder / "contract_final.log").read_text(encoding="utf-8", errors="replace")
assert "160 passed" in front
node = (folder / "node_transport.log").read_text(encoding="utf-8", errors="replace")
assert "pass 4" in node
diagrams = (folder / "diagrams.log").read_text(encoding="utf-8", errors="replace")
diagram_count = int(re.search(r"validated (\d+)", diagrams).group(1))
paths = [
    "platform/gateway/mox-platform-gateway-svc/src/alliance/event_replay.rs",
    "platform/gateway/mox-platform-gateway-svc/src/alliance/experts_streams.rs",
    "platform/gateway/mox-platform-gateway-svc/src/alliance/mod.rs",
    "platform/gateway/mox-platform-gateway-svc/src/lib.rs",
    "platform/gateway/mox-platform-gateway-svc/src/alliance/experts_db.rs",
    "platform/gateway/mox-platform-gateway-svc/src/alliance/experts_registry.rs",
    "platform/gateway/mox-platform-gateway-svc/tests/event_stream_resume.rs",
    "frontend-ui/src/modules/expert-alliance/contract/event-stream.js",
    "frontend-ui/src/modules/expert-alliance/composables/useAllianceEventStream.js",
    "reports/data/20261003-event-resume/verify-stream.mjs",
    "docs/expert-alliance/21-event-delivery-contract.md",
    "docs/expert-alliance/CURRENT-ARCHITECTURE.md",
    "docs/expert-alliance/16-decision-and-state-ledger.md",
    "docs/expert-alliance/INDEX.md",
    "docs/modules/REAL-IMPLEMENTATION-STATUS.md",
    "docs/specifications/tasks/20261002-enterprise-module-normalization/todo.md",
]
summary = {
    "scope": "gateway library and ten selected integration targets; frontend contracts, real transport and build",
    "checks": checks,
    "rust_passed": sum(counts), "rust_library_passed": counts[0],
    "rust_integration_passed": sum(counts[1:]), "frontend_contract_passed": 160,
    "node_transport_passed": 4, "mermaid_diagrams": diagram_count,
    "real_frontend_probe": {"MOX_EVENT_RESUME_PROBE": str(folder / "verify-stream.mjs"),
                            "result_marker_present": True, "counted_in_rust_integration": True},
    "original_failures": ["red.json", "frontend_red_ready.json", "cors_red.json"],
    "probe_setup_failure_retained": "frontend_red.json",
    "warnings": {"clippy_existing_library": 69, "doc_historical_backtick_paths": 206,
                 "ports": 284, "frontend_comment_pairing": 26},
    "source_sha256": {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in paths},
    "not_accepted": ["Webhook outbox", "process crash recovery", "cross-instance realtime broadcast",
                     "automatic reconnect", "whole workspace", "real OSS", "browser end-to-end", "production SLO"],
}
(folder / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print("Checks:", len(checks), "Rust:", sum(counts), "Frontend:", 160, "Node:", 4, "Diagrams:", diagram_count)
