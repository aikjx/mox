const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("head-test");
const result = client.headBucket("head-test");
if (!result.exists) process.exit(1);
console.log("XJ-OK: cloud-004_head_bucket");
process.exit(0);
