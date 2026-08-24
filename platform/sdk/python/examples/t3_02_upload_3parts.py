import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    bucket = "bk"
    key = "obj/3part.zip"
    r0 = client.createMultipartUpload(bucket, key)
    if not r0["ok"]:
        raise RuntimeError("create failed")
    uid = r0["upload_id"]
    parts = []
    for n in range(1, 4):
        size = n * 256
        chunk = bytes([n & 0xFF] * size)
        r = client.uploadPart(bucket, key, uid, n, chunk)
        if not r["ok"]:
            raise RuntimeError(f"uploadPart {n} failed")
        if len(r["etag"]) != 16:
            raise RuntimeError(f"etag len must be 16 got {len(r['etag'])}")
        parts.append({"part_number": n, "etag": r["etag"]})
    if len(parts) != 3:
        raise RuntimeError("expected 3 parts")
    etags = [p["etag"] for p in parts]
    print("XJ-OK: t3_02_upload_3parts uid=" + uid + " etags=[" + ",".join(etags) + "]")

if __name__ == "__main__":
    main()
