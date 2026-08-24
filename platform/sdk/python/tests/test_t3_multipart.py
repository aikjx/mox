import sys, os, pytest
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient, crc64_ecma

EXAMPLES_DIR = os.path.join(os.path.dirname(__file__), "..", "examples")
T3_IDS = [
    "t3_01_create_upload", "t3_02_upload_3parts", "t3_03_complete",
    "t3_04_abort", "t3_05_list_uploads", "t3_06_crc_check",
    "t3_07_upload_10parts_big", "t3_08_resume_partial"
]


@pytest.mark.parametrize("ex_id", T3_IDS)
def test_t3_example_file_exists(ex_id):
    fpath = os.path.join(EXAMPLES_DIR, ex_id + ".py")
    assert os.path.exists(fpath), f"Missing example file: {fpath}"


def test_all_8_t3_examples_present():
    files = os.listdir(EXAMPLES_DIR)
    found = set()
    for f in files:
        if f.startswith("t3_") and f.endswith(".py"):
            found.add(f[:-3])
    for ex_id in T3_IDS:
        assert ex_id in found, f"Missing t3 example: {ex_id}"
    assert len(found) >= 8


def test_create_multipart_returns_mpu_prefix():
    c = CloudClient()
    r = c.createMultipartUpload("b", "k")
    assert r["ok"] is True
    assert r["upload_id"].startswith("mpu-")


def test_upload_part_empty_returns_ok_false():
    c = CloudClient()
    r0 = c.createMultipartUpload("b", "k")
    uid = r0["upload_id"]
    r = c.uploadPart("b", "k", uid, 1, b"")
    assert r["ok"] is False
    assert r["error"] == "EmptyPart"


def test_complete_3_parts_writes_object_of_correct_size():
    c = CloudClient()
    uid = c.createMultipartUpload("b", "k")["upload_id"]
    parts = []
    total = 0
    for n in range(1, 4):
        data = f"HELLO-{n}"
        total += len(data)
        r = c.uploadPart("b", "k", uid, n, data)
        parts.append({"part_number": n, "etag": r["etag"]})
    fin = c.completeMultipartUpload("b", "k", uid, parts)
    assert fin["ok"] is True
    assert fin["parts"] == 3
    obj = c.getObject("b", "k")
    assert obj["found"] is True
    assert len(obj["data"]) == total


def test_abort_removes_upload_then_list_count_0():
    c = CloudClient()
    uid = c.createMultipartUpload("b", "k")["upload_id"]
    c.uploadPart("b", "k", uid, 1, b"abc")
    assert c.listMultipartUploads()["count"] == 1
    a = c.abortMultipartUpload(uid)
    assert a["ok"] is True
    assert a["aborted"] is True
    assert c.listMultipartUploads()["count"] == 0


def test_abort_nonexistent_id_returns_aborted_false():
    c = CloudClient()
    a = c.abortMultipartUpload("no-such-id")
    assert a["ok"] is False
    assert a["aborted"] is False


def test_crc64_known_vector_123456789():
    KNOWN = 0x6C40DF5F0B497347
    got = crc64_ecma(0, b"123456789")
    assert got == KNOWN, f"got={hex(got)}"


def test_crc64_incremental_two_parts_matches_combined():
    p1 = bytes([0xAB] * 128)
    p2 = bytes([0xCD] * 128)
    combined = p1 + p2
    direct = crc64_ecma(0, combined)
    step = crc64_ecma(crc64_ecma(0, p1), p2)
    assert direct == step
