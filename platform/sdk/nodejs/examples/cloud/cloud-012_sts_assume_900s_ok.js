const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const result = client.stsAssume("arn:mox:iam::role/readonly", 900);
if (!result.ok || !result.credentials) process.exit(1);
console.log("XJ-OK: cloud-012_sts_assume_900s_ok");
process.exit(0);
