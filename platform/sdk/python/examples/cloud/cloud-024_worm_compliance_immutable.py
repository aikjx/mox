import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("comp-bucket")
client.putObject("comp-bucket", "imm.txt", "immutable content")
client.wormRetention1y("comp-bucket", "imm.txt")
result = client.wormComplianceImmutable("comp-bucket", "imm.txt")
if not result["immutable"] or result["canDelete"]:
    sys.exit(1)
print("XJ-OK: cloud-024_worm_compliance_immutable")
sys.exit(0)
