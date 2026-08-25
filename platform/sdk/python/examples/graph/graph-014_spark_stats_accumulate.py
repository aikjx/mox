import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.sparkStatsAccumulate()
if not result["ok"] or not result["accumulated"]["totalTransactions"]:
    sys.exit(1)
print("XJ-OK: graph-014_spark_stats_accumulate")
sys.exit(0)
