import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("pfx-bucket")
client.putObject("pfx-bucket", "logs/2024/a.log", "a")
client.putObject("pfx-bucket", "logs/2024/b.log", "b")
client.putObject("pfx-bucket", "other.txt", "c")
result = client.listPrefix("pfx-bucket", "logs/")
if len(result["objects"]) < 2:
    sys.exit(1)
print("XJ-OK: cloud-009_list_prefix")
sys.exit(0)
