import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.cdcNew("consumer-001", {"offset": 0})
if not result["ok"] or result["consumerId"] != "consumer-001":
    sys.exit(1)
print("XJ-OK: graph-001_cdc_new")
sys.exit(0)
