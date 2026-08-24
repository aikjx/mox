import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
nodes = [{"id": "n1", "label": "User"}, {"id": "n2", "label": "User"}]
edges = [{"id": "e1", "src": "n1", "dst": "n2", "label": "KNOWS"}]
result = client.sparkWriterBulk(nodes, edges)
if not result["ok"] or result["nodesWritten"] != 2 or result["edgesWritten"] != 1:
    sys.exit(1)
print("XJ-OK: graph-010_spark_writer_bulk")
sys.exit(0)
