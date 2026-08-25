const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("worm-bucket");
client.putObject("worm-bucket", "critical.txt", "important data");
const result = client.wormRetention1y("worm-bucket", "critical.txt");
if (!result.ok || result.retention.days !== 365) process.exit(1);
console.log("XJ-OK: cloud-022_worm_retention_1y");
process.exit(0);
