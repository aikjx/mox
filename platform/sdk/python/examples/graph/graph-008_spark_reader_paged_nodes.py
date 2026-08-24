import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.sparkReaderPagedNodes(50, "1000")
if not result["ok"] or len(result["nodes"]) != 50:
    sys.exit(1)
print("XJ-OK: graph-008_spark_reader_paged_nodes")
sys.exit(0)
