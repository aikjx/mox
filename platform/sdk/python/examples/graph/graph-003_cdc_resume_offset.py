import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
client.cdcNew("consumer-003")
result = client.cdcResumeOffset("consumer-003", 42)
if not result["ok"] or result["resumedOffset"] != 42:
    sys.exit(1)
print("XJ-OK: graph-003_cdc_resume_offset")
sys.exit(0)
