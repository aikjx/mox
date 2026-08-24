const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("bucket-a");
client.createBucket("bucket-b");
const result = client.listBuckets();
if (!result.ok) process.exit(1);
console.log("XJ-OK: cloud-003_list_buckets");
process.exit(0);
