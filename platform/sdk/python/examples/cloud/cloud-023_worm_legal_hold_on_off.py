import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("lh-bucket")
client.putObject("lh-bucket", "hold.txt", "legal-hold-content")
client.wormLegalHoldOnOff("lh-bucket", "hold.txt", True)
result = client.wormLegalHoldOnOff("lh-bucket", "hold.txt", False)
if result["legalHold"] != "OFF":
    sys.exit(1)
print("XJ-OK: cloud-023_worm_legal_hold_on_off")
sys.exit(0)
