const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("stats-bucket");
const result = client.lifecycleBucketStats("stats-bucket");
if (!result.stats.totalObjects || result.stats.totalBytes <= 0) process.exit(1);
console.log("XJ-OK: cloud-028_lifecycle_bucket_stats");
process.exit(0);
