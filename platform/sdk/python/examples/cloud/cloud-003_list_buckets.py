import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("bucket-a")
client.createBucket("bucket-b")
result = client.listBuckets()
if not result["ok"]:
    sys.exit(1)
print("XJ-OK: cloud-003_list_buckets")
sys.exit(0)
