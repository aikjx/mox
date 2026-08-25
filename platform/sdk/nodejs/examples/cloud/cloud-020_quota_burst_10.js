const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const result = client.quotaBurst10(15);
if (!result.throttled) process.exit(1);
console.log("XJ-OK: cloud-020_quota_burst_10");
process.exit(0);
