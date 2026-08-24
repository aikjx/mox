const { CloudClient, crc64_ecma } = require("../xuanji-sdk-cloud");

function main() {
  const KNOWN = 0x6C40DF5F0B497347n;
  const computed = crc64_ecma(0, "123456789");
  if (computed !== KNOWN) {
    throw new Error("known vector mismatch: computed=" + computed.toString(16) + " expected=" + KNOWN.toString(16));
  }
  const p1 = Buffer.alloc(512, 0xAA);
  const p2 = Buffer.alloc(512, 0x55);
  const combined = Buffer.concat([p1, p2]);
  const direct = crc64_ecma(0, combined);
  const step = crc64_ecma(crc64_ecma(0, p1), p2);
  if (direct !== step) throw new Error("incremental CRC mismatch: direct=" + direct + " step=" + step);

  const client = new CloudClient();
  const bucket = "crcb", key = "crc/checked.bin";
  const r0 = client.createMultipartUpload(bucket, key);
  const uid = r0.upload_id;
  const r1 = client.uploadPart(bucket, key, uid, 1, p1);
  const r2 = client.uploadPart(bucket, key, uid, 2, p2);
  const parts = [
    { part_number: 1, etag: r1.etag },
    { part_number: 2, etag: r2.etag }
  ];
  client.completeMultipartUpload(bucket, key, uid, parts);
  const obj = client.getObject(bucket, key);
  if (!obj.found) throw new Error("obj not found");
  const objCrc = crc64_ecma(0, obj.data);
  if (objCrc !== direct) throw new Error("final object CRC mismatch");
  console.log("XJ-OK: t3-06-crc-check known_vec=0x" + computed.toString(16) + " incremental=0x" + step.toString(16) + " obj_crc=0x" + objCrc.toString(16));
}

if (require.main === module) main();
module.exports = main;

