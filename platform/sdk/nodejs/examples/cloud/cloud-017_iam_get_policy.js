const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.iamPutPolicy("ReadOnly", { Statement: [] });
const result = client.iamGetPolicy("ReadOnly");
if (!result.found || !result.policy) process.exit(1);
console.log("XJ-OK: cloud-017_iam_get_policy");
process.exit(0);
