import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
client.cdcNew("consumer-004")
result = client.cdc100kViaWriter("consumer-004", 5000)
if not result["ok"] or result["total"] != 100000:
    sys.exit(1)
print("XJ-OK: graph-004_cdc_100k_via_writer")
sys.exit(0)
