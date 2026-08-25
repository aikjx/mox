const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("acl-bucket");
const result = client.setBucketAcl("acl-bucket", "public-read");
if (!result.ok || result.acl !== "public-read") process.exit(1);
console.log("XJ-OK: cloud-005_set_bucket_acl");
process.exit(0);
