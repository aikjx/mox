import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
result = client.stsAssume("arn:mox:iam::role/readonly", 900)
if not result["ok"] or not result["credentials"]:
    sys.exit(1)
print("XJ-OK: cloud-012_sts_assume_900s_ok")
sys.exit(0)
