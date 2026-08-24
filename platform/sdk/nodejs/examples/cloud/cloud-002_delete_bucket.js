const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("temp-bucket");
client.deleteBucket("temp-bucket");
console.log("XJ-OK: cloud-002_delete_bucket");
process.exit(0);
