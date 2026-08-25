import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("stats-bucket")
result = client.lifecycleBucketStats("stats-bucket")
if not result["stats"]["totalObjects"] or result["stats"]["totalBytes"] <= 0:
    sys.exit(1)
print("XJ-OK: cloud-028_lifecycle_bucket_stats")
sys.exit(0)
