import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    bucket = "bigdata"
    key = "reports/2026/q2/large.csv"
    r0 = client.createMultipartUpload(bucket, key)
    if not r0["ok"]:
        raise RuntimeError("create failed")
    uid = r0["upload_id"]
    CHUNK = 64 * 1024
    parts = []
    total = 0
    for n in range(1, 11):
        data = bytearray([n & 0xFF] * CHUNK)
        for i in range(8):
            data[i] = (n + i) & 0xFF
        total += len(data)
        r = client.uploadPart(bucket, key, uid, n, bytes(data))
        if not r["ok"]:
            raise RuntimeError(f"part {n} upload failed")
        parts.append({"part_number": n, "etag": r["etag"]})
    if len(parts) != 10:
        raise RuntimeError(f"expected 10 parts got {len(parts)}")
    fin = client.completeMultipartUpload(bucket, key, uid, parts)
    if not fin["ok"]:
        raise RuntimeError("complete failed")
    obj = client.getObject(bucket, key)
    if not obj["found"]:
        raise RuntimeError("obj not found")
    data = obj["data"] if obj["data"] is not None else b""
    size = len(data)
    if size != total:
        raise RuntimeError(f"total size mismatch: expected={total} got={size}")
    if total != 10 * CHUNK:
        raise RuntimeError("CHUNK total mismatch")
    print(f"XJ-OK: t3_07_upload_10parts_big parts=10 total_bytes={total} etag={fin['etag']}")

if __name__ == "__main__":
    main()
