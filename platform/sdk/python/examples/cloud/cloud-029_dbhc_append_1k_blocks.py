import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
from xuanji_sdk_cloud import CloudClient

client = CloudClient({"region": "cn-east-1"})
client.createBucket("dbhc-bucket")
result = client.dbhcAppend1kBlocks("dbhc-bucket", "chain.log", 5)
if not result["ok"] or result["blocksAppended"] != 5 or result["totalSize"] != 5 * 1024:
    sys.exit(1)
print("XJ-OK: cloud-029_dbhc_append_1k_blocks")
sys.exit(0)
