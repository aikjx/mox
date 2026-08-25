import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("src-bucket")
client.createBucket("dst-bucket")
client.putObject("src-bucket", "src.txt", "copy-me")
result = client.copyObject("src-bucket", "src.txt", "dst-bucket", "dst.txt")
if not result["ok"]:
    sys.exit(1)
print("XJ-OK: cloud-010_copy_object")
sys.exit(0)
