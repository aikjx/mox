import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F6Partial(0.03)
if not result["passed"] or not result["handled"]:
    sys.exit(1)
print("XJ-OK: graph-025_ac15_f6_partial")
sys.exit(0)
