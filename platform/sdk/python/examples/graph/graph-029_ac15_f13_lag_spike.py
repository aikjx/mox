import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F13LagSpike(15)
if not result["passed"] or not result["recovered"]:
    sys.exit(1)
print("XJ-OK: graph-029_ac15_f13_lag_spike")
sys.exit(0)
