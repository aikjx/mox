const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("obj-bucket");
client.putObject("obj-bucket", "todelete.txt", "temp");
client.deleteObject("obj-bucket", "todelete.txt");
const result = client.getObject("obj-bucket", "todelete.txt");
if (result.found) process.exit(1);
console.log("XJ-OK: cloud-008_delete_object");
process.exit(0);
