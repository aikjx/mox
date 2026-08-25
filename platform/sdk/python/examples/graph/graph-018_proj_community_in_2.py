import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projCommunityIn2("comm-002")
if not result["ok"] or len(result["tags"]) == 0:
    sys.exit(1)
print("XJ-OK: graph-018_proj_community_in_2")
sys.exit(0)
