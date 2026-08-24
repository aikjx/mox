import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("temp-bucket")
client.deleteBucket("temp-bucket")
print("XJ-OK: cloud-002_delete_bucket")
sys.exit(0)
