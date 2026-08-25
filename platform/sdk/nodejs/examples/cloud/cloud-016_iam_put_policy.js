const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const doc = { Version: "2012", Statement: [{ Effect: "Allow", Action: "s3:*" }] };
const result = client.iamPutPolicy("AdminPolicy", doc);
if (!result.ok || !result.version) process.exit(1);
console.log("XJ-OK: cloud-016_iam_put_policy");
process.exit(0);
