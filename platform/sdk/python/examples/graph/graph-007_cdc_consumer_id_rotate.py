import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
client.cdcNew("consumer-old-7")
result = client.cdcConsumerIdRotate("consumer-old-7", "consumer-new-7")
if not result["rotated"]:
    sys.exit(1)
print("XJ-OK: graph-007_cdc_consumer_id_rotate")
sys.exit(0)
