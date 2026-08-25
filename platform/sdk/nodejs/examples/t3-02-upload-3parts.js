const { CloudClient } = require("../mox-sdk-cloud");

function main() {
  const client = new CloudClient();
  const bucket = "bk";
  const key = "obj/3part.zip";
  const r0 = client.createMultipartUpload(bucket, key);
  if (!r0.ok) throw new Error("create failed");
  const uid = r0.upload_id;
  const parts = [];
  for (let n = 1; n <= 3; n++) {
    const size = n * 256;
    const chunk = Buffer.alloc(size, n & 0xFF);
    const r = client.uploadPart(bucket, key, uid, n, chunk);
    if (!r.ok) throw new Error("uploadPart " + n + " failed");
    if (r.etag.length !== 16) throw new Error("etag must be 16 chars, got " + r.etag.length);
    parts.push({ part_number: n, etag: r.etag });
  }
  if (parts.length !== 3) throw new Error("expected 3 parts");
  const etagsOnly = parts.map(p => p.etag);
  console.log("XJ-OK: t3-02-upload-3parts uid=" + uid + " etags=[" + etagsOnly.join(",") + "]");
}

if (require.main === module) main();
module.exports = main;
