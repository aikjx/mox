import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("life-bucket")
result = client.lifecycleHotToWarm30d("life-bucket")
if not result["ok"] or result["rule"]["transition"]["days"] != 30:
    sys.exit(1)
print("XJ-OK: cloud-025_lifecycle_hot_to_warm_30d")
sys.exit(0)
