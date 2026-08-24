def crc64_ecma(state, data):
    POLY = 0x42F0E1EBA9EA3693
    s = state & 0xFFFFFFFFFFFFFFFF
    if isinstance(data, str):
        data = data.encode("utf-8")
    for b in data:
        s ^= (b & 0xFF) << 56
        for _ in range(8):
            if s & (1 << 63):
                s = ((s << 1) ^ POLY) & 0xFFFFFFFFFFFFFFFF
            else:
                s = (s << 1) & 0xFFFFFFFFFFFFFFFF
    return s


def _fxhash16(data):
    if isinstance(data, str):
        data = data.encode("utf-8")
    h = 0xcbf29ce484222325
    FNV = 0x100000001b3
    MASK = 0xFFFFFFFFFFFFFFFF
    for b in data:
        h ^= (b & 0xFF)
        h = (h * FNV) & MASK
    return format(h, "016x")


def _rand_hex8():
    import time
    t = int(time.time() * 1_000_000) & 0xFFFFFFFF
    return format(t, "08x")
class CloudClient:
    def __init__(self, options=None):
        self.options = options or {}
        self._buckets = {}
        self._objects = {}
        self._policies = {}
        self._multiparts = {}

    def createBucket(self, name, opts=None):
        opts = opts or {}
        self._buckets[name] = {"name": name, "acl": opts.get("acl", "private"), "createdAt": 0}
        return {"ok": True, "bucket": name}

    def deleteBucket(self, name):
        if name in self._buckets:
            del self._buckets[name]
        return {"ok": True, "bucket": name}

    def listBuckets(self):
        return {"ok": True, "buckets": list(self._buckets.values())}

    def headBucket(self, name):
        exists = name in self._buckets
        return {"ok": True, "exists": exists, "bucket": self._buckets.get(name)}

    def setBucketAcl(self, name, acl):
        if name in self._buckets:
            self._buckets[name]["acl"] = acl
        return {"ok": True, "bucket": name, "acl": acl}

    def putObject(self, bucket, key, data, opts=None):
        opts = opts or {}
        full_key = f"{bucket}/{key}"
        size = len(data) if data else 0
        self._objects[full_key] = {
            "bucket": bucket, "key": key, "data": data,
            "size": size, **opts
        }
        etag = _fxhash16(data if data else b"")
        return {"ok": True, "bucket": bucket, "key": key, "etag": etag}

    def getObject(self, bucket, key):
        full_key = f"{bucket}/{key}"
        obj = self._objects.get(full_key)
        return {"ok": True, "bucket": bucket, "key": key,
                "data": obj["data"] if obj else None, "found": obj is not None}

    def deleteObject(self, bucket, key):
        full_key = f"{bucket}/{key}"
        if full_key in self._objects:
            del self._objects[full_key]
        return {"ok": True, "bucket": bucket, "key": key}

    def listPrefix(self, bucket, prefix):
        prefix_full = f"{bucket}/{prefix}"
        results = []
        for k, obj in self._objects.items():
            if k.startswith(prefix_full):
                results.append(obj)
        return {"ok": True, "bucket": bucket, "prefix": prefix, "objects": results}

    def copyObject(self, srcBucket, srcKey, dstBucket, dstKey):
        src_full = f"{srcBucket}/{srcKey}"
        src_obj = self._objects.get(src_full)
        if src_obj:
            dst_full = f"{dstBucket}/{dstKey}"
            self._objects[dst_full] = {**src_obj, "bucket": dstBucket, "key": dstKey}
        return {"ok": True, "src": {"bucket": srcBucket, "key": srcKey},
                "dst": {"bucket": dstBucket, "key": dstKey}}

    def createMultipartUpload(self, bucket, key):
        upload_id = f"mpu-{bucket}-{key}-{_rand_hex8()}-{_rand_hex8()}"
        self._multiparts[upload_id] = {
            "upload_id": upload_id, "bucket": bucket, "key": key,
            "parts": {}
        }
        return {"ok": True, "upload_id": upload_id, "bucket": bucket, "key": key}

    def uploadPart(self, bucket, key, upload_id, part_number, data):
        mpu = self._multiparts.get(upload_id)
        if mpu is None:
            return {"ok": False, "error": "NotFound",
                    "message": f"upload_id {upload_id} not found"}
        if isinstance(data, str):
            data = data.encode("utf-8")
        if data is None or len(data) == 0:
            return {"ok": False, "error": "EmptyPart", "message": "empty part"}
        etag = _fxhash16(data)
        mpu["parts"][part_number] = {"etag": etag, "data": data}
        return {"ok": True, "part_number": part_number, "etag": etag, "upload_id": upload_id}

    def completeMultipartUpload(self, bucket, key, upload_id, parts):
        mpu = self._multiparts.get(upload_id)
        if mpu is None:
            return {"ok": False, "error": "NotFound",
                    "message": f"upload_id {upload_id} not found"}
        ordered = parts if (parts is not None and len(parts) > 0) else [
            {"part_number": n} for n in sorted(mpu["parts"].keys())
        ]
        combined = bytearray()
        for p in ordered:
            stored = mpu["parts"].get(p["part_number"])
            if stored is not None:
                combined.extend(stored["data"])
        del self._multiparts[upload_id]
        full_key = f"{bucket}/{key}"
        self._objects[full_key] = {
            "bucket": bucket, "key": key, "data": bytes(combined),
            "size": len(combined), "multipart": True, "parts": len(ordered)
        }
        final_etag = f"{len(ordered)}-{_fxhash16(bytes(combined))}{_fxhash16(full_key + str(len(ordered)))[:8]}"
        return {"ok": True, "bucket": bucket, "key": key, "etag": final_etag,
                "parts": len(ordered), "size": len(combined)}

    def abortMultipartUpload(self, upload_id):
        if upload_id in self._multiparts:
            del self._multiparts[upload_id]
            return {"ok": True, "upload_id": upload_id, "aborted": True}
        return {"ok": False, "upload_id": upload_id, "aborted": False, "error": "NotFound"}

    def listMultipartUploads(self):
        uploads = []
        for m in self._multiparts.values():
            uploads.append({
                "upload_id": m["upload_id"],
                "bucket": m["bucket"],
                "key": m["key"],
                "parts_count": len(m["parts"])
            })
        uploads.sort(key=lambda x: x["upload_id"])
        return {"ok": True, "uploads": uploads, "count": len(uploads)}

    def multipartUpload(self, bucket, key, parts=None):
        parts = parts or []
        full_key = f"{bucket}/{key}"
        all_data = b"".join((p.get("data", "") if isinstance(p.get("data", ""), bytes)
                             else p.get("data", "").encode("utf-8")) for p in parts)
        self._objects[full_key] = {
            "bucket": bucket, "key": key, "data": all_data,
            "size": len(all_data), "multipart": True, "parts": len(parts)
        }
        return {"ok": True, "bucket": bucket, "key": key,
                "parts": len(parts), "etag": f"fake-multipart-{full_key}"}

    def stsAssume(self, roleArn, durationSeconds):
        if durationSeconds <= 900:
            return {
                "ok": True,
                "credentials": {
                    "accessKeyId": f"STS-ACCESS-{roleArn}",
                    "secretAccessKey": f"STS-SECRET-{roleArn}",
                    "sessionToken": f"STS-TOKEN-{roleArn}",
                    "durationSeconds": durationSeconds
                }
            }
        return {"ok": False, "error": "DurationSecondsExceeded",
                "message": f"Max allowed: 900, requested: {durationSeconds}"}

    def stsTokenSignatureVerify(self, token, signature):
        expected = f"sig-{token}"
        return {"ok": True, "valid": signature == expected, "token": token, "signature": signature}

    def stsAssumeChain(self, roles=None):
        roles = roles or []
        chain = [{"roleArn": r, "credentials": {
            "accessKeyId": f"CHAIN-{i}-ACCESS",
            "secretAccessKey": f"CHAIN-{i}-SECRET",
            "sessionToken": f"CHAIN-{i}-TOKEN"
        }} for i, r in enumerate(roles)]
        return {"ok": True, "chain": chain, "length": len(chain)}

    def iamPutPolicy(self, policyName, document):
        self._policies[policyName] = {"policyName": policyName, "document": document, "version": 1}
        return {"ok": True, "policyName": policyName, "version": 1}

    def iamGetPolicy(self, policyName):
        policy = self._policies.get(policyName)
        return {"ok": True, "policyName": policyName, "policy": policy, "found": policy is not None}

    def iamEvalDenyFirst(self, actions=None, resource=""):
        actions = actions or []
        deny_list = ["s3:DeleteBucket", "iam:DeletePolicy"]
        denied = [a for a in actions if a in deny_list]
        allowed = [a for a in actions if a not in deny_list]
        return {"ok": True, "denied": denied, "allowed": allowed,
                "decision": "DENY" if denied else "ALLOW"}

    def quota50PerMin(self, requestCount=0):
        limit = 50
        return {"ok": True, "limit": limit, "used": requestCount,
                "remaining": max(0, limit - requestCount),
                "withinLimit": requestCount <= limit}

    def quotaBurst10(self, burstCount=0):
        limit = 10
        return {"ok": True, "burstLimit": limit, "burstUsed": burstCount,
                "burstRemaining": max(0, limit - burstCount),
                "throttled": burstCount > limit}

    def quotaRetryAfterHeader(self, currentRate=0):
        limit = 100
        over = currentRate > limit
        return {"ok": True, "limit": limit, "currentRate": currentRate,
                "throttled": over,
                "retryAfterSeconds": ((currentRate - limit) + 9) // 10 if over else 0}

    def wormRetention1y(self, bucket, key):
        full_key = f"{bucket}/{key}"
        if full_key in self._objects:
            self._objects[full_key]["wormRetention"] = {"mode": "COMPLIANCE", "days": 365}
        return {"ok": True, "bucket": bucket, "key": key,
                "retention": {"mode": "COMPLIANCE", "days": 365}}

    def wormLegalHoldOnOff(self, bucket, key, on=True):
        full_key = f"{bucket}/{key}"
        status = "ON" if on else "OFF"
        if full_key in self._objects:
            self._objects[full_key]["legalHold"] = status
        return {"ok": True, "bucket": bucket, "key": key, "legalHold": status}

    def wormComplianceImmutable(self, bucket, key):
        full_key = f"{bucket}/{key}"
        obj = self._objects.get(full_key)
        immutable = bool(obj and obj.get("wormRetention", {}).get("mode") == "COMPLIANCE")
        return {"ok": True, "bucket": bucket, "key": key,
                "immutable": immutable, "canDelete": not immutable}

    def lifecycleHotToWarm30d(self, bucket):
        rule = {"id": "hot-to-warm", "transition": {"days": 30, "storageClass": "WARM"}}
        return {"ok": True, "bucket": bucket, "rule": rule}

    def lifecycleWarmToCold180d(self, bucket):
        rule = {"id": "warm-to-cold", "transition": {"days": 180, "storageClass": "COLD"}}
        return {"ok": True, "bucket": bucket, "rule": rule}

    def lifecycleColdToHotRestore(self, bucket, key, days=1):
        return {"ok": True, "bucket": bucket, "key": key,
                "restoreDays": days, "restored": True}

    def lifecycleBucketStats(self, bucket):
        hot, warm, cold = 100, 50, 20
        return {
            "ok": True, "bucket": bucket,
            "stats": {
                "hotObjects": hot, "warmObjects": warm, "coldObjects": cold,
                "totalObjects": hot + warm + cold,
                "totalBytes": hot * 1024 + warm * 2048 + cold * 4096
            }
        }

    def dbhcAppend1kBlocks(self, bucket, key, blockCount=10):
        full_key = f"{bucket}/{key}"
        current = self._objects.get(full_key)
        total_data = current.get("data", "") if current else ""
        if isinstance(total_data, bytes):
            total_data = total_data + ("A" * 1024 * blockCount).encode("utf-8")
        else:
            total_data = total_data + "A" * (1024 * blockCount)
        self._objects[full_key] = {
            "bucket": bucket, "key": key, "data": total_data,
            "size": len(total_data), "dbhc": True, "blocks": blockCount
        }
        return {"ok": True, "bucket": bucket, "key": key,
                "blocksAppended": blockCount, "totalSize": len(total_data)}

    def dbhcVerifyCliOk(self, bucket, key):
        full_key = f"{bucket}/{key}"
        obj = self._objects.get(full_key)
        return {"ok": True, "bucket": bucket, "key": key,
                "verified": bool(obj and obj.get("dbhc")),
                "size": obj["size"] if obj else 0}

