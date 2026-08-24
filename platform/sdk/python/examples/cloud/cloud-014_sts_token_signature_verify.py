import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
token = "mytoken123"
signature = f"sig-{token}"
result = client.stsTokenSignatureVerify(token, signature)
if not result["valid"]:
    sys.exit(1)
print("XJ-OK: cloud-014_sts_token_signature_verify")
sys.exit(0)
