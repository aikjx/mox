const { CloudClient } = require("../xuanji-sdk-cloud");

function main() {
  const client = new CloudClient();
  const bucket = "abkt";
  const key = "abort/file.tmp";
  const r0 = client.createMultipartUpload(bucket, key);
  if (!r0.ok) throw new Error("create failed");
  const uid = r0.upload_id;
  client.uploadPart(bucket, key, uid, 1, "partial");
  const before = client.listMultipartUploads();
  if (before.count !== 1) throw new Error("expected 1 before abort, got " + before.count);
  const a = client.abortMultipartUpload(uid);
  if (!a.ok || !a.aborted) throw new Error("abort should return ok:true aborted:true");
  const after = client.listMultipartUploads();
  if (after.count !== 0) throw new Error("expected 0 after abort, got " + after.count);
  console.log("XJ-OK: t3-04-abort removed=" + uid + " remaining=" + after.count);
}

if (require.main === module) main();
module.exports = main;
