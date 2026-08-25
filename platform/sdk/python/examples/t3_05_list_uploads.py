import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from mox_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    names = ["alpha", "beta", "gamma", "delta"]
    for i, n in enumerate(names):
        r0 = client.createMultipartUpload("lb", f"file/{n}.bin")
        if not r0["ok"]:
            raise RuntimeError(f"create failed for {n}")
        uid = r0["upload_id"]
        if i % 2 == 0:
            client.uploadPart("lb", f"file/{n}.bin", uid, 1, bytes(64))
    lst = client.listMultipartUploads()
    if lst["count"] != 4:
        raise RuntimeError(f"expected 4 uploads got {lst['count']}")
    with_parts = sum(1 for m in lst["uploads"] if m["parts_count"] > 0)
    if with_parts != 2:
        raise RuntimeError(f"expected 2 with parts got {with_parts}")
    print(f"XJ-OK: t3_05_list_uploads total={lst['count']} with_parts={with_parts}")

if __name__ == "__main__":
    main()
