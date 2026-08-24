import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.sparkReaderPagedEdges(25, "0")
if not result["ok"] or len(result["edges"]) != 25:
    sys.exit(1)
print("XJ-OK: graph-009_spark_reader_paged_edges")
sys.exit(0)
