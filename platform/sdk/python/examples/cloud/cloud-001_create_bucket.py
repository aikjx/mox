import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("my-bucket-001")
print("XJ-OK: cloud-001_create_bucket")
sys.exit(0)
