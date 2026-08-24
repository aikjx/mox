import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.sparkRoundtrip2k3k()
if not result["ok"] or result["consistency"] != "verified":
    sys.exit(1)
print("XJ-OK: graph-012_spark_roundtrip_2k_3k")
sys.exit(0)
