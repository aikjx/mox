import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("obj-bucket")
result = client.putObject("obj-bucket", "hello.txt", "Hello World")
if not result["ok"] or not result["etag"]:
    sys.exit(1)
print("XJ-OK: cloud-006_put_object")
sys.exit(0)
