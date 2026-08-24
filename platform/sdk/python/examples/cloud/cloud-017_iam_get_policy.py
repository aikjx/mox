import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.iamPutPolicy("ReadOnly", {"Statement": []})
result = client.iamGetPolicy("ReadOnly")
if not result["found"] or not result["policy"]:
    sys.exit(1)
print("XJ-OK: cloud-017_iam_get_policy")
sys.exit(0)
