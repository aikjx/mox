import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("worm-bucket")
client.putObject("worm-bucket", "critical.txt", "important data")
result = client.wormRetention1y("worm-bucket", "critical.txt")
if not result["ok"] or result["retention"]["days"] != 365:
    sys.exit(1)
print("XJ-OK: cloud-022_worm_retention_1y")
sys.exit(0)
