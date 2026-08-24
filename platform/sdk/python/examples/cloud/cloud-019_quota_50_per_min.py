import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
result = client.quota50PerMin(30)
if not result["withinLimit"] or result["remaining"] != 20:
    sys.exit(1)
print("XJ-OK: cloud-019_quota_50_per_min")
sys.exit(0)
