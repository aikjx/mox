const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("dbhc-bucket");
client.dbhcAppend1kBlocks("dbhc-bucket", "chain.log", 3);
const result = client.dbhcVerifyCliOk("dbhc-bucket", "chain.log");
if (!result.verified) process.exit(1);
console.log("XJ-OK: cloud-030_dbhc_verify_cli_ok");
process.exit(0);
