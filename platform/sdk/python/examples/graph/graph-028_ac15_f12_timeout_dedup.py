import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F12TimeoutDedup(3000)
if not result["passed"] or result["duplicatesHandled"] < 1:
    sys.exit(1)
print("XJ-OK: graph-028_ac15_f12_timeout_dedup")
sys.exit(0)
