const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("dbhc-bucket");
const result = client.dbhcAppend1kBlocks("dbhc-bucket", "chain.log", 5);
if (!result.ok || result.blocksAppended !== 5 || result.totalSize !== 5 * 1024) process.exit(1);
console.log("XJ-OK: cloud-029_dbhc_append_1k_blocks");
process.exit(0);
