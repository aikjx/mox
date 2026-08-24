const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("life-bucket");
const result = client.lifecycleHotToWarm30d("life-bucket");
if (!result.ok || result.rule.transition.days !== 30) process.exit(1);
console.log("XJ-OK: cloud-025_lifecycle_hot_to_warm_30d");
process.exit(0);
