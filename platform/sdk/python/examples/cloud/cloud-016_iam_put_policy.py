import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
doc = {"Version": "2012", "Statement": [{"Effect": "Allow", "Action": "s3:*"}]}
result = client.iamPutPolicy("AdminPolicy", doc)
if not result["ok"] or not result["version"]:
    sys.exit(1)
print("XJ-OK: cloud-016_iam_put_policy")
sys.exit(0)
