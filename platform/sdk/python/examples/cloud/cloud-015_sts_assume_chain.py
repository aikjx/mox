import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
roles = ["arn:role/A", "arn:role/B", "arn:role/C"]
result = client.stsAssumeChain(roles)
if not result["ok"] or result["length"] != 3:
    sys.exit(1)
print("XJ-OK: cloud-015_sts_assume_chain")
sys.exit(0)
