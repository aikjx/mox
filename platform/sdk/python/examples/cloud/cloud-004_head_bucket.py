import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("head-test")
result = client.headBucket("head-test")
if not result["exists"]:
    sys.exit(1)
print("XJ-OK: cloud-004_head_bucket")
sys.exit(0)
