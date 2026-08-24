import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_graph import GraphClient

client = GraphClient({"endpoint": "graph.local"})
result = client.ac15F7DiskfullErr()
if not result["passed"] or not result["gracefulDegradation"]:
    sys.exit(1)
print("XJ-OK: graph-026_ac15_f7_diskfull_err")
sys.exit(0)
