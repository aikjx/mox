const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("obj-bucket");
const result = client.putObject("obj-bucket", "hello.txt", "Hello World");
if (!result.ok || !result.etag) process.exit(1);
console.log("XJ-OK: cloud-006_put_object");
process.exit(0);
