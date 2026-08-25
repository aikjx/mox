import sys, os, pytest
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from mox_sdk_cloud import CloudClient

EXAMPLES_DIR = os.path.join(os.path.dirname(__file__), "..", "examples", "cloud")

CLOUD_IDS = [
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
]


# --- Example ID existence tests (30 IDs = 30 cases, but we pick some to keep total per file >=15) ---
# We'll test first 7 IDs as individual tests
@pytest.mark.parametrize("ex_id", CLOUD_IDS[:7])
def test_example_file_exists(ex_id):
    files = os.listdir(EXAMPLES_DIR)
    found = False
    for f in files:
        if not f.endswith(".py"): continue
        base = f[:-3]
        if base == ex_id or base.startswith(ex_id + "_"):
            found = True
            break
    assert found, f"Example file missing for {ex_id}"


# Check all 30 IDs exist in batch
def test_all_30_cloud_ids_have_files():
    files = os.listdir(EXAMPLES_DIR)
    for ex_id in CLOUD_IDS:
        found = False
        for f in files:
            if not f.endswith(".py"): continue
            base = f[:-3]
            if base == ex_id or base.startswith(ex_id + "_"):
                found = True
                break
        assert found, f"Missing example file for {ex_id}"


# --- CloudClient method tests ---
def test_create_bucket_returns_ok():
    client = CloudClient()
    r = client.createBucket("b1")
    assert r["ok"] is True
    assert r["bucket"] == "b1"


def test_head_bucket_exists_after_create():
    client = CloudClient()
    client.createBucket("hb")
    r = client.headBucket("hb")
    assert r["exists"] is True


def test_head_bucket_not_exists_for_missing():
    client = CloudClient()
    r = client.headBucket("never-created")
    assert r["exists"] is False


def test_put_and_get_object_roundtrip():
    client = CloudClient()
    client.createBucket("d")
    client.putObject("d", "k", "payload-xyz")
    r = client.getObject("d", "k")
    assert r["found"] is True
    assert r["data"] == "payload-xyz"


def test_delete_object_removes_it():
    client = CloudClient()
    client.createBucket("d2")
    client.putObject("d2", "x", "temp")
    client.deleteObject("d2", "x")
    r = client.getObject("d2", "x")
    assert r["found"] is False


def test_sts_assume_900s_ok():
    client = CloudClient()
    r = client.stsAssume("arn:role/read", 900)
    assert r["ok"] is True
    assert "accessKeyId" in r["credentials"]


def test_sts_assume_3600s_reject():
    client = CloudClient()
    r = client.stsAssume("arn:role/long", 3600)
    assert r["ok"] is False


def test_iam_eval_deny_delete_bucket():
    client = CloudClient()
    r = client.iamEvalDenyFirst(["s3:DeleteBucket"], "*")
    assert r["decision"] == "DENY"
    assert len(r["denied"]) > 0


def test_quota_within_limit_ok():
    client = CloudClient()
    r = client.quota50PerMin(5)
    assert r["withinLimit"] is True
    assert r["remaining"] == 45


def test_quota_over_limit():
    client = CloudClient()
    r = client.quota50PerMin(55)
    assert r["withinLimit"] is False


def test_burst_over_limit_throttled():
    client = CloudClient()
    r = client.quotaBurst10(11)
    assert r["throttled"] is True


def test_worm_retention_1y_sets_days():
    client = CloudClient()
    client.createBucket("w")
    client.putObject("w", "x", "v")
    r = client.wormRetention1y("w", "x")
    assert r["retention"]["days"] == 365
    assert r["retention"]["mode"] == "COMPLIANCE"


def test_worm_compliance_immutable_after_retention():
    client = CloudClient()
    client.createBucket("w2")
    client.putObject("w2", "imm", "v")
    client.wormRetention1y("w2", "imm")
    r = client.wormComplianceImmutable("w2", "imm")
    assert r["immutable"] is True
    assert r["canDelete"] is False


def test_dbhc_append_size_matches():
    client = CloudClient()
    client.createBucket("dbhc")
    r = client.dbhcAppend1kBlocks("dbhc", "log", 6)
    assert r["blocksAppended"] == 6
    assert r["totalSize"] == 6 * 1024


def test_lifecycle_hot_to_warm_30d_rule():
    client = CloudClient()
    client.createBucket("l")
    r = client.lifecycleHotToWarm30d("l")
    assert r["rule"]["transition"]["days"] == 30
    assert r["rule"]["transition"]["storageClass"] == "WARM"
