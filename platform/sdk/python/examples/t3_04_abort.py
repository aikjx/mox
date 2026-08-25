import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from mox_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    bucket = "abkt"
    key = "abort/file.tmp"
    r0 = client.createMultipartUpload(bucket, key)
    if not r0["ok"]:
        raise RuntimeError("create failed")
    uid = r0["upload_id"]
    client.uploadPart(bucket, key, uid, 1, "partial")
    before = client.listMultipartUploads()
    if before["count"] != 1:
        raise RuntimeError(f"expected 1 before abort got {before['count']}")
    a = client.abortMultipartUpload(uid)
    if not a["ok"] or not a["aborted"]:
        raise RuntimeError("abort should be ok+aborted")
    after = client.listMultipartUploads()
    if after["count"] != 0:
        raise RuntimeError(f"expected 0 after abort got {after['count']}")
    print(f"XJ-OK: t3_04_abort removed={uid} remaining={after['count']}")

if __name__ == "__main__":
    main()
