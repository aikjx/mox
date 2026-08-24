import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.sparkRoundtrip5k8k()
if not result["ok"] or result["nodesWritten"] != 5120:
    sys.exit(1)
print("XJ-OK: graph-013_spark_roundtrip_5k_8k")
sys.exit(0)
