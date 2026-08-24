const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("obj-bucket");
client.putObject("obj-bucket", "data.txt", "my data content");
const result = client.getObject("obj-bucket", "data.txt");
if (!result.found || result.data !== "my data content") process.exit(1);
console.log("XJ-OK: cloud-007_get_object");
process.exit(0);
