const { CloudClient } = require("../mox-sdk-cloud");

function main() {
  const client = new CloudClient();
  const bucket = "bigdata";
  const key = "reports/2026/q2/large.csv";
  const r0 = client.createMultipartUpload(bucket, key);
  if (!r0.ok) throw new Error("create failed");
  const uid = r0.upload_id;
  const CHUNK = 64 * 1024;
  const parts = [];
  let total = 0;
  for (let n = 1; n <= 10; n++) {
    const data = Buffer.alloc(CHUNK, n & 0xFF);
    for (let i = 0; i < 8; i++) data[i] = (n + i) & 0xFF;
    total += data.length;
    const r = client.uploadPart(bucket, key, uid, n, data);
    if (!r.ok) throw new Error("part " + n + " upload failed");
    parts.push({ part_number: n, etag: r.etag });
  }
  if (parts.length !== 10) throw new Error("expected 10 parts, got " + parts.length);
  const fin = client.completeMultipartUpload(bucket, key, uid, parts);
  if (!fin.ok) throw new Error("complete failed");
  const obj = client.getObject(bucket, key);
  if (!obj.found) throw new Error("obj not found");
  const size = (obj.data && obj.data.length) || 0;
  if (size !== total) throw new Error("total size mismatch: expected=" + total + " got=" + size);
  if (total !== 10 * CHUNK) throw new Error("CHUNK total mismatch");
  console.log("XJ-OK: t3-07-upload-10parts-big parts=10 total_bytes=" + total + " etag=" + fin.etag);
}

if (require.main === module) main();
module.exports = main;
