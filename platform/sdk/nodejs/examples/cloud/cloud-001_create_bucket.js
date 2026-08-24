const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("my-bucket-001");
console.log("XJ-OK: cloud-001_create_bucket");
process.exit(0);
