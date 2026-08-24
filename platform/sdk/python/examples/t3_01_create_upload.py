import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient

def main():
    client = CloudClient()
    r = client.createMultipartUpload("my-bucket", "data/large-file.bin")
    if not r["ok"]:
        raise RuntimeError("create failed: " + str(r))
    uid = r["upload_id"]
    if not uid or len(uid) == 0:
        raise RuntimeError("upload_id empty")
    if not uid.startswith("mpu-"):
        raise RuntimeError("upload_id must start with mpu-: " + uid)
    print("XJ-OK: t3_01_create_upload id=" + uid)

if __name__ == "__main__":
    main()
