const { CloudClient } = require("../mox-sdk-cloud");

function main() {
  const client = new CloudClient();
  const names = ["alpha", "beta", "gamma", "delta"];
  names.forEach((n, i) => {
    const r0 = client.createMultipartUpload("lb", "file/" + n + ".bin");
    if (!r0.ok) throw new Error("create failed for " + n);
    const uid = r0.upload_id;
    if (i % 2 === 0) {
      client.uploadPart("lb", "file/" + n + ".bin", uid, 1, Buffer.alloc(64, 0));
    }
  });
  const list = client.listMultipartUploads();
  if (list.count !== 4) throw new Error("expected 4 uploads, got " + list.count);
  const withParts = list.uploads.filter(m => m.parts_count > 0).length;
  if (withParts !== 2) throw new Error("expected 2 uploads with parts, got " + withParts);
  console.log("XJ-OK: t3-05-list-uploads total=" + list.count + " with_parts=" + withParts);
}

if (require.main === module) main();
module.exports = main;
