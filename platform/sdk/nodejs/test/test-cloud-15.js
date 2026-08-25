const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { CloudClient } = require("../mox-sdk-cloud");

const EXAMPLES_DIR = path.join(__dirname, "..", "examples", "cloud");

const CLOUD_IDS = [
  "cloud-001_create_bucket", "cloud-002_delete_bucket", "cloud-003_list_buckets",
  "cloud-004_head_bucket", "cloud-005_set_bucket_acl", "cloud-006_put_object",
  "cloud-007_get_object", "cloud-008_delete_object", "cloud-009_list_prefix",
  "cloud-010_copy_object", "cloud-011_multipart_upload", "cloud-012_sts_assume_900s_ok",
  "cloud-013_sts_assume_3600s_reject", "cloud-014_sts_token_signature_verify",
  "cloud-015_sts_assume_chain", "cloud-016_iam_put_policy", "cloud-017_iam_get_policy",
  "cloud-018_iam_eval_deny_first", "cloud-019_quota_50_per_min",
  "cloud-020_quota_burst_10", "cloud-021_quota_retry_after_header",
  "cloud-022_worm_retention_1y", "cloud-023_worm_legal_hold_on_off",
  "cloud-024_worm_compliance_immutable", "cloud-025_lifecycle_hot_to_warm_30d",
  "cloud-026_lifecycle_warm_to_cold_180d", "cloud-027_lifecycle_cold_to_hot_restore",
  "cloud-028_lifecycle_bucket_stats", "cloud-029_dbhc_append_1k_blocks",
  "cloud-030_dbhc_verify_cli_ok"
];

describe("Cloud SDK Example ID Existence", function () {
  CLOUD_IDS.forEach(function (id) {
    it(`example file exists for ${id}`, function () {
      const files = fs.readdirSync(EXAMPLES_DIR);
      const found = files.find(function (f) {
        if (!f.endsWith(".js")) return false;
        const base = f.slice(0, -3);
        return base === id || base.startsWith(id + "_");
      });
      assert.ok(found, `Example file not found for ID: ${id}`);
    });
  });
});

describe("CloudClient Core Methods", function () {
  it("creates a bucket and returns ok:true", function () {
    const client = new CloudClient();
    const r = client.createBucket("test-bucket");
    assert.strictEqual(r.ok, true);
    assert.strictEqual(r.bucket, "test-bucket");
  });

  it("headBucket reports exists:true for created bucket", function () {
    const client = new CloudClient();
    client.createBucket("head-bkt");
    const r = client.headBucket("head-bkt");
    assert.strictEqual(r.exists, true);
  });

  it("putObject then getObject returns the same data", function () {
    const client = new CloudClient();
    client.createBucket("data");
    client.putObject("data", "k", "hello world");
    const r = client.getObject("data", "k");
    assert.strictEqual(r.found, true);
    assert.strictEqual(r.data, "hello world");
  });

  it("stsAssume rejects duration > 900 with ok:false", function () {
    const client = new CloudClient();
    const r = client.stsAssume("arn:role/x", 1800);
    assert.strictEqual(r.ok, false);
  });

  it("stsAssume returns credentials for 900s", function () {
    const client = new CloudClient();
    const r = client.stsAssume("arn:role/y", 900);
    assert.strictEqual(r.ok, true);
    assert.ok(r.credentials.accessKeyId);
  });

  it("iamEvalDenyFirst denies s3:DeleteBucket", function () {
    const client = new CloudClient();
    const r = client.iamEvalDenyFirst(["s3:DeleteBucket"], "*");
    assert.strictEqual(r.decision, "DENY");
  });

  it("quota50PerMin within limit ok", function () {
    const client = new CloudClient();
    const r = client.quota50PerMin(10);
    assert.strictEqual(r.withinLimit, true);
    assert.strictEqual(r.remaining, 40);
  });

  it("dbhcAppend1kBlocks appends correct size", function () {
    const client = new CloudClient();
    client.createBucket("dbhc");
    const r = client.dbhcAppend1kBlocks("dbhc", "log", 4);
    assert.strictEqual(r.blocksAppended, 4);
    assert.strictEqual(r.totalSize, 4 * 1024);
  });
});
