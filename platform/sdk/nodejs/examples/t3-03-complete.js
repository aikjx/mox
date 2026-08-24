const { CloudClient } = require("../xuanji-sdk-cloud");

function main() {
  const client = new CloudClient();
  const bucket = "complete-bucket";
  const key = "final/assembly.dat";
  const r0 = client.createMultipartUpload(bucket, key);
  if (!r0.ok) throw new Error("create failed");
  const uid = r0.upload_id;
  const parts = [];
  let totalLen = 0;
  for (let n = 1; n <= 3; n++) {
    const s = "PART-" + n + "-DATA-";
    totalLen += s.length;
    const r = client.uploadPart(bucket, key, uid, n, s);
    if (!r.ok) throw new Error("uploadPart " + n + " failed");
    parts.push({ part_number: n, etag: r.etag });
  }
  const fin = client.completeMultipartUpload(bucket, key, uid, parts);
  if (!fin.ok) throw new Error("complete failed: " + fin.message);
  if (!fin.etag) throw new Error("final etag empty");
  const obj = client.getObject(bucket, key);
  if (!obj.found) throw new Error("object not found after complete");
  const size = (obj.data && obj.data.length) || 0;
  if (size !== totalLen) throw new Error("size mismatch expected=" + totalLen + " got=" + size);
  console.log("XJ-OK: t3-03-complete etag=" + fin.etag + " size=" + size);
}

if (require.main === module) main();
module.exports = main;
