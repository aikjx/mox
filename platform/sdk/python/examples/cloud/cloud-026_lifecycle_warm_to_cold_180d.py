import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("life-bucket")
result = client.lifecycleWarmToCold180d("life-bucket")
if not result["ok"] or result["rule"]["transition"]["days"] != 180:
    sys.exit(1)
print("XJ-OK: cloud-026_lifecycle_warm_to_cold_180d")
sys.exit(0)
