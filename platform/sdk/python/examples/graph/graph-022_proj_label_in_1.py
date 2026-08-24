import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projLabelIn1("node-multi", ["Employee", "Manager"])
if not result["ok"] or len(result["labelsApplied"]) != 2:
    sys.exit(1)
print("XJ-OK: graph-022_proj_label_in_1")
sys.exit(0)
