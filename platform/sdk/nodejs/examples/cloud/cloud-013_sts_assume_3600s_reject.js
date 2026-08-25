const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const result = client.stsAssume("arn:mox:iam::role/long", 3600);
if (result.ok) process.exit(1);
console.log("XJ-OK: cloud-013_sts_assume_3600s_reject");
process.exit(0);
