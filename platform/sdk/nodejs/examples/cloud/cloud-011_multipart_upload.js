const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("mp-bucket");
const parts = [
  { partNumber: 1, data: "AAA" },
  { partNumber: 2, data: "BBB" },
  { partNumber: 3, data: "CCC" }
];
const result = client.multipartUpload("mp-bucket", "large.bin", parts);
if (!result.ok || result.parts !== 3) process.exit(1);
console.log("XJ-OK: cloud-011_multipart_upload");
process.exit(0);
