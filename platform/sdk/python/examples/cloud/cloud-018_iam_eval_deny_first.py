import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
result = client.iamEvalDenyFirst(["s3:ListBucket", "s3:DeleteBucket"], "arn::bucket/*")
if result["decision"] != "DENY" or len(result["denied"]) == 0:
    sys.exit(1)
print("XJ-OK: cloud-018_iam_eval_deny_first")
sys.exit(0)
