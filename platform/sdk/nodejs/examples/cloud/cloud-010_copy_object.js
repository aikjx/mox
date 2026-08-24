const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("src-bucket");
client.createBucket("dst-bucket");
client.putObject("src-bucket", "src.txt", "copy-me");
const result = client.copyObject("src-bucket", "src.txt", "dst-bucket", "dst.txt");
if (!result.ok) process.exit(1);
console.log("XJ-OK: cloud-010_copy_object");
process.exit(0);
