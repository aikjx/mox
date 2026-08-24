import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F1DoubleIdempotent("op-idempotent-1")
if not result["passed"] or not result["sameResult"]:
    sys.exit(1)
print("XJ-OK: graph-023_ac15_f1_double_idempotent")
sys.exit(0)
