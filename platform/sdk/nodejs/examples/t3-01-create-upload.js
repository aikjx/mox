const { CloudClient } = require("../xuanji-sdk-cloud");

function main() {
  const client = new CloudClient();
  const r = client.createMultipartUpload("my-bucket", "data/large-file.bin");
  if (!r.ok) throw new Error("create failed: " + r.message);
  const uid = r.upload_id;
  if (!uid || uid.length === 0) throw new Error("upload_id empty");
  if (!uid.startsWith("mpu-")) throw new Error("upload_id must start with mpu-: " + uid);
  console.log("XJ-OK: t3-01-create-upload id=" + uid);
}

if (require.main === module) main();
module.exports = main;
