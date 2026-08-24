const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { CloudClient, crc64_ecma } = require("../xuanji-sdk-cloud");

const EXAMPLES_DIR = path.join(__dirname, "..", "examples");
const T3_IDS = [
  "t3-01-create-upload", "t3-02-upload-3parts", "t3-03-complete",
  "t3-04-abort", "t3-05-list-uploads", "t3-06-crc-check",
  "t3-07-upload-10parts-big", "t3-08-resume-partial"
];

describe("T3 Node Multipart SDK Example File Existence", function () {
  T3_IDS.forEach(function (id) {
    it("example file exists for " + id, function () {
      const fpath = path.join(EXAMPLES_DIR, id + ".js");
      assert.ok(fs.existsSync(fpath), "Missing example file: " + fpath);
    });
  });
});

describe("T3 CloudClient Multipart Lifecycle", function () {
  it("creates multipart upload and upload_id has mpu- prefix", function () {
    const c = new CloudClient();
    const r = c.createMultipartUpload("b", "k");
    assert.strictEqual(r.ok, true);
    assert.ok(r.upload_id.startsWith("mpu-"), "upload_id must start with mpu-");
  });

  it("uploadPart rejects empty part with ok:false", function () {
    const c = new CloudClient();
    const r0 = c.createMultipartUpload("b", "k");
    const r = c.uploadPart("b", "k", r0.upload_id, 1, Buffer.alloc(0));
    assert.strictEqual(r.ok, false);
    assert.strictEqual(r.error, "EmptyPart");
  });

  it("complete 3 contiguous parts writes assembled object with correct size", function () {
    const c = new CloudClient();
    const uid = c.createMultipartUpload("b", "k").upload_id;
    const parts = [];
    let total = 0;
    for (let n = 1; n <= 3; n++) {
      const data = "HELLO-" + n;
      total += data.length;
      const r = c.uploadPart("b", "k", uid, n, data);
      parts.push({ part_number: n, etag: r.etag });
    }
    const fin = c.completeMultipartUpload("b", "k", uid, parts);
    assert.strictEqual(fin.ok, true);
    assert.strictEqual(fin.parts, 3);
    const obj = c.getObject("b", "k");
    assert.strictEqual(obj.found, true);
    assert.strictEqual(obj.data.length, total);
  });

  it("abort removes upload and list returns 0 count", function () {
    const c = new CloudClient();
    const uid = c.createMultipartUpload("b", "k").upload_id;
    c.uploadPart("b", "k", uid, 1, "abc");
    assert.strictEqual(c.listMultipartUploads().count, 1);
    const a = c.abortMultipartUpload(uid);
    assert.strictEqual(a.ok, true);
    assert.strictEqual(a.aborted, true);
    assert.strictEqual(c.listMultipartUploads().count, 0);
  });

  it("abort on non-existent id returns ok:false aborted:false", function () {
    const c = new CloudClient();
    const a = c.abortMultipartUpload("no-such-id");
    assert.strictEqual(a.ok, false);
    assert.strictEqual(a.aborted, false);
  });
});

describe("T3 CRC64 ECMA-182 Verification", function () {
  it("known vector 123456789 matches 0x6C40DF5F0B497347", function () {
    const KNOWN = 0x6C40DF5F0B497347n;
    const got = crc64_ecma(0, "123456789");
    assert.strictEqual(got, KNOWN, "got=0x" + got.toString(16));
  });

  it("two-part incremental CRC matches direct combined CRC", function () {
    const p1 = Buffer.alloc(128, 0xAB);
    const p2 = Buffer.alloc(128, 0xCD);
    const combined = Buffer.concat([p1, p2]);
    const direct = crc64_ecma(0, combined);
    const step = crc64_ecma(crc64_ecma(0, p1), p2);
    assert.strictEqual(direct, step);
  });
});

