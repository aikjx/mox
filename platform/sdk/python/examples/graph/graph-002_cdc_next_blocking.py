import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
client.cdcNew("consumer-002")
result = client.cdcNextBlocking("consumer-002")
if not result["ok"] or len(result["events"]) == 0:
    sys.exit(1)
print("XJ-OK: graph-002_cdc_next_blocking")
sys.exit(0)
