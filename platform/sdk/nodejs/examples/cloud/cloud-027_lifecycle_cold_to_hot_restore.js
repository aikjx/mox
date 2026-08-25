const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("cold-bucket");
client.putObject("cold-bucket", "archive.bin", "archived");
const result = client.lifecycleColdToHotRestore("cold-bucket", "archive.bin", 7);
if (!result.restored || result.restoreDays !== 7) process.exit(1);
console.log("XJ-OK: cloud-027_lifecycle_cold_to_hot_restore");
process.exit(0);
