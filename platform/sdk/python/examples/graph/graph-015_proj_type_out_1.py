import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projTypeOut1("node-abc")
if not result["ok"] or result["projectType"] != "GRAPH":
    sys.exit(1)
print("XJ-OK: graph-015_proj_type_out_1")
sys.exit(0)
