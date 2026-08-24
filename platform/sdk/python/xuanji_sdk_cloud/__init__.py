class CloudClient:
    def __init__(self, options=None):
        self.options = options or {}
        self._buckets = {}
        self._objects = {}
        self._policies = {}

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
        self._objects[full_key] = {
            "bucket": bucket, "key": key, "data": data,
            "size": len(data) if data else 0, **opts
        }
        return {"ok": True, "bucket": bucket, "key": key, "etag": f"fake-etag-{full_key}"}

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

    def multipartUpload(self, bucket, key, parts=None):
        parts = parts or []
        full_key = f"{bucket}/{key}"
        all_data = "".join(p.get("data", "") for p in parts)
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
        for _ in range(blockCount):
            total_data += "A" * 1024
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
