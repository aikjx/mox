import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F8CbPlusAudit()
if not result["passed"] or not result["callbackInvoked"]:
    sys.exit(1)
print("XJ-OK: graph-027_ac15_f8_cb_plus_audit")
sys.exit(0)
