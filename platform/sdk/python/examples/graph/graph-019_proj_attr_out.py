import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projAttrOut("node-person-1")
if not result["exported"] or not result["attributes"]["name"]:
    sys.exit(1)
print("XJ-OK: graph-019_proj_attr_out")
sys.exit(0)
