const { CloudClient } = require("../xuanji-sdk-cloud");

function main() {
  const client = new CloudClient();
  const bucket = "resume";
  const key = "resume/final.dat";

  // First attempt: upload 2 parts, abort (simulated failure)
  const r1 = client.createMultipartUpload(bucket, key);
  const uid1 = r1.upload_id;
  client.uploadPart(bucket, key, uid1, 1, Buffer.alloc(200, 1));
  client.uploadPart(bucket, key, uid1, 2, Buffer.alloc(200, 2));
  client.abortMultipartUpload(uid1);
  if (client.listMultipartUploads().count !== 0) throw new Error("aborted upload should be removed");

  // Second attempt: resume by creating new upload, upload all 4 parts
  const r2 = client.createMultipartUpload(bucket, key);
  const uid2 = r2.upload_id;
  const sizes = [200, 200, 300, 300];
  const parts = [];
  let total = 0;
  for (let i = 0; i < sizes.length; i++) {
    const n = i + 1;
    const data = Buffer.alloc(sizes[i], n & 0xFF);
    total += sizes[i];
    const r = client.uploadPart(bucket, key, uid2, n, data);
    if (!r.ok) throw new Error("part " + n + " failed");
    parts.push({ part_number: n, etag: r.etag });
  }
  const fin = client.completeMultipartUpload(bucket, key, uid2, parts);
  if (!fin.ok) throw new Error("complete failed");
  const obj = client.getObject(bucket, key);
  if (!obj.found) throw new Error("final obj missing");
  const size = (obj.data && obj.data.length) || 0;
  if (size !== total) throw new Error("size mismatch");
  if (client.listMultipartUploads().count !== 0) throw new Error("no uploads should remain after complete");
  console.log("XJ-OK: t3-08-resume-partial first_uid=" + uid1 + " second_uid=" + uid2 + " total_bytes=" + total + " etag=" + fin.etag);
}

if (require.main === module) main();
module.exports = main;
