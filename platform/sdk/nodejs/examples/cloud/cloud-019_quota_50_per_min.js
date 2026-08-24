const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const result = client.quota50PerMin(30);
if (!result.withinLimit || result.remaining !== 20) process.exit(1);
console.log("XJ-OK: cloud-019_quota_50_per_min");
process.exit(0);
