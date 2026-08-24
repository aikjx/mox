const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const roles = ["arn:role/A", "arn:role/B", "arn:role/C"];
const result = client.stsAssumeChain(roles);
if (!result.ok || result.length !== 3) process.exit(1);
console.log("XJ-OK: cloud-015_sts_assume_chain");
process.exit(0);
