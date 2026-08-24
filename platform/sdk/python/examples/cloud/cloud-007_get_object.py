import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("obj-bucket")
client.putObject("obj-bucket", "data.txt", "my data content")
result = client.getObject("obj-bucket", "data.txt")
if not result["found"] or result["data"] != "my data content":
    sys.exit(1)
print("XJ-OK: cloud-007_get_object")
sys.exit(0)
