const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("life-bucket");
const result = client.lifecycleWarmToCold180d("life-bucket");
if (!result.ok || result.rule.transition.days !== 180) process.exit(1);
console.log("XJ-OK: cloud-026_lifecycle_warm_to_cold_180d");
process.exit(0);
