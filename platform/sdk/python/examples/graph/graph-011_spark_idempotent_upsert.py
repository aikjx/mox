import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
nodes = [
    {"id": "nu1", "label": "User"},
    {"id": "nu2", "label": "User"},
    {"id": "nu1", "label": "User"}
]
result = client.sparkIdempotentUpsert(nodes)
if not result["idempotent"] or result["updated"] < 1:
    sys.exit(1)
print("XJ-OK: graph-011_spark_idempotent_upsert")
sys.exit(0)
