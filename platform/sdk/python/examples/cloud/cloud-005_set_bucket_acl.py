import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("acl-bucket")
result = client.setBucketAcl("acl-bucket", "public-read")
if not result["ok"] or result["acl"] != "public-read":
    sys.exit(1)
print("XJ-OK: cloud-005_set_bucket_acl")
sys.exit(0)
