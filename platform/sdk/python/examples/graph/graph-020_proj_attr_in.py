import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projAttrIn("node-imp-1", {"score": 99, "level": "gold"})
if not result["imported"] or result["imported"] < 1:
    sys.exit(1)
print("XJ-OK: graph-020_proj_attr_in")
sys.exit(0)
