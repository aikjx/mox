import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
client.cdcNew("consumer-006")
result = client.cdcLagMonitor("consumer-006")
if not result["ok"] or not isinstance(result["lagMs"], int):
    sys.exit(1)
print("XJ-OK: graph-006_cdc_lag_monitor")
sys.exit(0)
