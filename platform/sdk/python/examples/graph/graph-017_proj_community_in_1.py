import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.projCommunityIn1("comm-001")
if not result["ok"] or result["nodes"] < 100:
    sys.exit(1)
print("XJ-OK: graph-017_proj_community_in_1")
sys.exit(0)
