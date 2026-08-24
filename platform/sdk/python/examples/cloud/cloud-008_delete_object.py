import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("obj-bucket")
client.putObject("obj-bucket", "todelete.txt", "temp")
client.deleteObject("obj-bucket", "todelete.txt")
result = client.getObject("obj-bucket", "todelete.txt")
if result["found"]:
    sys.exit(1)
print("XJ-OK: cloud-008_delete_object")
sys.exit(0)
