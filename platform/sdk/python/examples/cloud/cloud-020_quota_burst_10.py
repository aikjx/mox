import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
result = client.quotaBurst10(15)
if not result["throttled"]:
    sys.exit(1)
print("XJ-OK: cloud-020_quota_burst_10")
sys.exit(0)
