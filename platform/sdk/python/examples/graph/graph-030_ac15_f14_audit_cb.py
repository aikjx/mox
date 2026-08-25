import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F14AuditCb("evt-audit-999")
if not result["passed"] or not result["auditLogged"] or not result["callbackFired"]:
    sys.exit(1)
print("XJ-OK: graph-030_ac15_f14_audit_cb")
sys.exit(0)
