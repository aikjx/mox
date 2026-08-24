import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projTypeOut2("node-xyz")
if not result["ok"] or result["schemaVersion"] != 2:
    sys.exit(1)
print("XJ-OK: graph-016_proj_type_out_2")
sys.exit(0)
