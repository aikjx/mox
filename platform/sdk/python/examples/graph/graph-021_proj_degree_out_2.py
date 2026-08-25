import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projDegreeOut2("hub-node")
if not result["ok"] or result["outDegree"] != 2:
    sys.exit(1)
print("XJ-OK: graph-021_proj_degree_out_2")
sys.exit(0)
