import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
result = client.quotaRetryAfterHeader(150)
if not result["throttled"] or result["retryAfterSeconds"] <= 0:
    sys.exit(1)
print("XJ-OK: cloud-021_quota_retry_after_header")
sys.exit(0)
