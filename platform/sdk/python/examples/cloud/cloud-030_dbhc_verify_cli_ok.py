import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from mox_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("dbhc-bucket")
client.dbhcAppend1kBlocks("dbhc-bucket", "chain.log", 3)
result = client.dbhcVerifyCliOk("dbhc-bucket", "chain.log")
if not result["verified"]:
    sys.exit(1)
print("XJ-OK: cloud-030_dbhc_verify_cli_ok")
sys.exit(0)
