import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("cold-bucket")
client.putObject("cold-bucket", "archive.bin", "archived")
result = client.lifecycleColdToHotRestore("cold-bucket", "archive.bin", 7)
if not result["restored"] or result["restoreDays"] != 7:
    sys.exit(1)
print("XJ-OK: cloud-027_lifecycle_cold_to_hot_restore")
sys.exit(0)
