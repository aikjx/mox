import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_cloud import CloudClient, crc64_ecma

def main():
    KNOWN = 0x6C40DF5F0B497347
    computed = crc64_ecma(0, b"123456789")
    if computed != KNOWN:
        raise RuntimeError(f"known vec mismatch: got={hex(computed)} expected={hex(KNOWN)}")
    p1 = bytes([0xAA] * 512)
    p2 = bytes([0x55] * 512)
    combined = p1 + p2
    direct = crc64_ecma(0, combined)
    step = crc64_ecma(crc64_ecma(0, p1), p2)
    if direct != step:
        raise RuntimeError(f"incremental CRC mismatch: direct={direct} step={step}")
    client = CloudClient()
    bucket = "crcb"
    key = "crc/checked.bin"
    r0 = client.createMultipartUpload(bucket, key)
    uid = r0["upload_id"]
    r1 = client.uploadPart(bucket, key, uid, 1, p1)
    r2 = client.uploadPart(bucket, key, uid, 2, p2)
    parts = [
        {"part_number": 1, "etag": r1["etag"]},
        {"part_number": 2, "etag": r2["etag"]}
    ]
    client.completeMultipartUpload(bucket, key, uid, parts)
    obj = client.getObject(bucket, key)
    if not obj["found"]:
        raise RuntimeError("obj not found")
    obj_crc = crc64_ecma(0, obj["data"])
    if obj_crc != direct:
        raise RuntimeError("final object CRC mismatch")
    print(f"XJ-OK: t3_06_crc_check known_vec={hex(computed)} incremental={hex(step)} obj_crc={hex(obj_crc)}")

if __name__ == "__main__":
    main()
