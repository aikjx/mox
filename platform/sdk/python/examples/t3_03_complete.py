import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    bucket = "complete-bucket"
    key = "final/assembly.dat"
    r0 = client.createMultipartUpload(bucket, key)
    if not r0["ok"]:
        raise RuntimeError("create failed")
    uid = r0["upload_id"]
    parts = []
    total_len = 0
    for n in range(1, 4):
        s = f"PART-{n}-DATA-"
        total_len += len(s)
        r = client.uploadPart(bucket, key, uid, n, s)
        if not r["ok"]:
            raise RuntimeError(f"uploadPart {n} failed")
        parts.append({"part_number": n, "etag": r["etag"]})
    fin = client.completeMultipartUpload(bucket, key, uid, parts)
    if not fin["ok"]:
        raise RuntimeError("complete failed: " + str(fin))
    if not fin["etag"]:
        raise RuntimeError("final etag empty")
    obj = client.getObject(bucket, key)
    if not obj["found"]:
        raise RuntimeError("object not found after complete")
    data = obj["data"] if obj["data"] is not None else b""
    size = len(data)
    if size != total_len:
        raise RuntimeError(f"size mismatch expected={total_len} got={size}")
    print(f"XJ-OK: t3_03_complete etag={fin['etag']} size={size}")

if __name__ == "__main__":
    main()
