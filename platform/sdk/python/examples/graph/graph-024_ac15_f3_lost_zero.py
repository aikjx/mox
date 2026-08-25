import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F3LostZero()
if not result["passed"] or result["lossRate"] != 0:
    sys.exit(1)
print("XJ-OK: graph-024_ac15_f3_lost_zero")
sys.exit(0)
