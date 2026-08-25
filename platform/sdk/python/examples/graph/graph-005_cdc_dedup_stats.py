import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
client.cdcNew("consumer-005")
result = client.cdcDedupStats("consumer-005")
if not result["ok"] or result["totalEvents"] <= 0:
    sys.exit(1)
print("XJ-OK: graph-005_cdc_dedup_stats")
sys.exit(0)
