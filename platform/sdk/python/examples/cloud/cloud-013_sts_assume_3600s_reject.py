import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
result = client.stsAssume("arn:xuanji:iam::role/long", 3600)
if result["ok"]:
    sys.exit(1)
print("XJ-OK: cloud-013_sts_assume_3600s_reject")
sys.exit(0)
