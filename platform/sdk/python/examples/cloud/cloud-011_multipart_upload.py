import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("mp-bucket")
parts = [
    {"partNumber": 1, "data": "AAA"},
    {"partNumber": 2, "data": "BBB"},
    {"partNumber": 3, "data": "CCC"}
]
result = client.multipartUpload("mp-bucket", "large.bin", parts)
if not result["ok"] or result["parts"] != 3:
    sys.exit(1)
print("XJ-OK: cloud-011_multipart_upload")
sys.exit(0)
