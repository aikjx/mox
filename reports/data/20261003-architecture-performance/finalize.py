"""Calculate measurements and record current evidence; do not declare global readiness."""
import hashlib
import json
import math
import re
import statistics
from pathlib import Path
folder = Path(__file__).resolve().parent
root = folder.parents[2]
log = (folder / "performance_final.log").read_text(encoding="utf-8")
performance = json.loads(re.search(r"PERFORMANCE_JSON=(\{[^\n]+\})",log)[1])
samples = sorted(performance["fast_delivery_microseconds"])
performance.update({"sample_count":len(samples),"median_microseconds":statistics.median(samples),
    "p95_nearest_rank_microseconds":samples[math.ceil(len(samples)*0.95)-1],"max_microseconds":max(samples),
    "baseline":"Unrelated tenant failed 500ms deadline while actual slow receiver remained held", "production_slo":False})
names = ["metadata","registry","registry_tests","catalog_final","rust_final","performance_final","contract","clippy",
    "diagrams","paths_selftest","paths","docs","ports","diff","format","api_selftest","formulas_selftest"]
checks = []
for name in names:
    file = folder / (name + ".json")
    checks.append(dict(json.loads(file.read_text(encoding="utf-8")),name=name) if file.exists() else {"name":name,"status":"pending"})
rust_log = (folder / "rust_final.log").read_text(encoding="utf-8")
rust_tests = sum(int(n) for n in re.findall(r"test result: ok\. (\d+) passed",rust_log))
paths = ["platform/gateway/mox-platform-gateway-svc/src/alliance/" + file for file in ["experts_events.rs","experts_streams.rs"]]
paths += ["platform/gateway/mox-platform-gateway-svc/tests/webhook_performance.rs"]
paths += ["docs/expert-alliance/" + file for file in ["09-deployment-templates.md","21-event-delivery-contract.md","INDEX.md","CURRENT-ARCHITECTURE.md","16-decision-and-state-ledger.md"]]
paths += ["docs/modules/CODE-CATALOG.md","docs/modules/REAL-IMPLEMENTATION-STATUS.md",
    "docs/specifications/tasks/20261002-enterprise-module-normalization/todo.md","reports/markdown/20261003-architecture-performance.md"]
summary = {"date":"2026-10-03","status":"verified_increment_with_open_global_acceptance",
    "module_structure":{"modules":26,"requirements":78,"workspace_packages":150,"whole_modules_accepted_this_batch":0},
    "rust_integration_tests":rust_tests,"unique_rust_tests":rust_tests,"separate_performance_repeat_counted_as_new_test":False,
    "frontend_contract_tests":157,"registry_unit_tests":6,"performance":performance,"checks":checks,
    "no_new_mock":True,"browser_e2e":False,"remote_ci":False,"production_slo":False,"actual_cloud_oss_roundtrip":False,
    "full_workspace_build_or_test":False,"existing_clippy_library_warnings":69,
    "historical_failures":["performance_red: test setup missed consumer readiness; corrected without altering production",
        "performance_red_ready: actual unrelated tenant timeout above 500ms", "catalog_red: generated inventory drift; regenerated canonical source"],
    "remaining":["transactional business outbox","ordered/idempotent signed delivery","DNS pinning","tenant fairness/quotas",
        "knowledge and file source version linking","unified KB migration","actual OSS and model integration","full browser and cross-host SLO"],
    "source_sha256":{path:hashlib.sha256((root/path).read_bytes()).hexdigest() for path in paths}}
(folder / "summary.json").write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding="utf-8")
print("Rust unique tests:",rust_tests,"delivery samples:",len(samples),"median us:",performance["median_microseconds"],"p95 us:",performance["p95_nearest_rank_microseconds"])
