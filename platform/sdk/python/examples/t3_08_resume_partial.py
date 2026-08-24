import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    bucket = "resume"
    key = "resume/final.dat"

    r1 = client.createMultipartUpload(bucket, key)
    uid1 = r1["upload_id"]
    client.uploadPart(bucket, key, uid1, 1, bytes([1] * 200))
    client.uploadPart(bucket, key, uid1, 2, bytes([2] * 200))
    client.abortMultipartUpload(uid1)
    if client.listMultipartUploads()["count"] != 0:
        raise RuntimeError("aborted upload should be removed")

    r2 = client.createMultipartUpload(bucket, key)
    uid2 = r2["upload_id"]
    sizes = [200, 200, 300, 300]
    parts = []
    total = 0
    for i, sz in enumerate(sizes):
        n = i + 1
        data = bytes([n & 0xFF] * sz)
        total += sz
        r = client.uploadPart(bucket, key, uid2, n, data)
        if not r["ok"]:
            raise RuntimeError(f"part {n} failed")
        parts.append({"part_number": n, "etag": r["etag"]})
    fin = client.completeMultipartUpload(bucket, key, uid2, parts)
    if not fin["ok"]:
        raise RuntimeError("complete failed")
    obj = client.getObject(bucket, key)
    if not obj["found"]:
        raise RuntimeError("final obj missing")
    data = obj["data"] if obj["data"] is not None else b""
    size = len(data)
    if size != total:
        raise RuntimeError("size mismatch")
    if client.listMultipartUploads()["count"] != 0:
        raise RuntimeError("no uploads should remain after complete")
    print(f"XJ-OK: t3_08_resume_partial first_uid={uid1} second_uid={uid2} total_bytes={total} etag={fin['etag']}")

if __name__ == "__main__":
    main()
