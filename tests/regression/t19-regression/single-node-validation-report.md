# Mox v2.0 单二进制部署验证报告

- 验证时间: 2026-08-24T15:08:14.428Z
- 目标端点: http://127.0.0.1:18080
- 启动方式: mox-server.exe server --single-node --public-port 18080 --ctrl-port 19080 --data-port 19081
- 总体通过率: **40/40** (100.0%)

## 逐项结果

| # | 项 | 结果 | 详情 |
|---|---|---|---|
| 1 | SV2.1 /health returns 200 | ✅ PASS | 200 |
| 2 | SV2.2 health ok=true | ✅ PASS | {"audit_chain_len":2,"graph":{"archived_edges":0,"edges":0,"objects":0,"soft_deleted":0,"tags":0},"metrics":true,"mpu_uploads_active":0,"objects_stored":0,"ok":true,"uptime_ms":10161} |
| 3 | SV2.3 audit_chain_len >= 1 (seeded) | ✅ PASS | 2 |
| 4 | SV2.4 /metrics returns 200 | ✅ PASS | 200 body_len=3970 |
| 5 | SV2.5 Content-Type Prometheus text | ✅ PASS | text/plain; version=0.0.4; charset=utf-8 |
| 6 | SV3.1 PUT /s3 returns 200 | ✅ PASS | 200 {"bucket":"demo","crc64_ecma":"c3f0c58038a5be9f","etag":"c3f0c58038a5be9f-62283f39b7f03ab4","fusion_status":true,"graph_wrote_edges":4,"key":"hello/alpha.bin","miji_level":1,"ok":true,"ref":"s3://demo/hello/alpha.bin","size":343,"tags_count":3} |
| 7 | SV3.2 PUT returns non-empty ETag | ✅ PASS | c3f0c58038a5be9f-62283f39b7f03ab4 |
| 8 | SV3.3 PUT returns non-empty CRC64/ECMA | ✅ PASS | c3f0c58038a5be9f |
| 9 | SV3.4 PUT CRC matches client-computed CRC64/ECMA | ✅ PASS | server=c3f0c58038a5be9f client=c3f0c58038a5be9f |
| 10 | SV3.5 PUT ETag matches deterministic client formula | ✅ PASS | server=c3f0c58038a5be9f-62283f39b7f03ab4 client=c3f0c58038a5be9f-62283f39b7f03ab4 |
| 11 | SV3.6 PUT fusion_status = true | ✅ PASS | true |
| 12 | SV3.7 PUT tags_count = 3 | ✅ PASS | tags_count=3 |
| 13 | SV3.8 GET returns 200 | ✅ PASS | 200 body_len=343 |
| 14 | SV3.9 GET body equals original payload | ✅ PASS | orig_len=343 got_len=343 |
| 15 | SV3.10 GET x-amz-meta-crc64-ecma header matches PUT CRC | ✅ PASS | put=c3f0c58038a5be9f get_hdr=c3f0c58038a5be9f |
| 16 | SV3.11 GET ETag header matches PUT ETag | ✅ PASS | put=c3f0c58038a5be9f-62283f39b7f03ab4 get_hdr="c3f0c58038a5be9f-62283f39b7f03ab4" |
| 17 | SV4.1 /graph/stats returns 200 | ✅ PASS | 200 |
| 18 | SV4.2 graph.objects >= 1 (from fusion PUT) | ✅ PASS | 1 |
| 19 | SV4.3 graph.tags >= 3 (project/owner/dataset + defaults?) | ✅ PASS | 4 |
| 20 | SV4.4 graph.edges >= objCount (obj+tags HAS_TAG) | ✅ PASS | edges=4 objs=1 |
| 21 | SV4.5 /graph/query_by_tag returns 200 | ✅ PASS | 200 |
| 22 | SV4.6 query_by_tag count >= 1 | ✅ PASS | count=1 |
| 23 | SV4.7 query result ref == s3 ref | ✅ PASS | ref=s3://demo/hello/alpha.bin |
| 24 | SV4.8 query result ETag matches PUT | ✅ PASS | put=c3f0c58038a5be9f-62283f39b7f03ab4 graph=c3f0c58038a5be9f-62283f39b7f03ab4 |
| 25 | SV4.9 query result CRC hex matches PUT | ✅ PASS | put=c3f0c58038a5be9f graph=c3f0c58038a5be9f |
| 26 | SV4.10 /audit/chain returns 200 | ✅ PASS | 200 |
| 27 | SV4.11 audit verified=true (WORM integrity) | ✅ PASS | verified=true len=3 last_block=2 |
| 28 | SV4.12 audit len >= 2 (genesis + PUT + seed?) | ✅ PASS | 3 |
| 29 | SV5.1 /metrics exposes all 10 Mox base metrics | ✅ PASS | 10/10; missing= |
| 30 | SV6.1 MPU create returns UploadId | ✅ PASS | {"bucket":"demo","key":"mpu/big.bin","ok":true,"owner":"api-user","upload_id":"e6d0c40b-94a5-4070-a411-12d96455cbaa-9090123b4533"} |
| 31 | SV6.2 part 1 upload | ✅ PASS | part=1 status=200 etag=cb93bc2daed8e588 crc=cb93bc2daed8e588 |
| 32 | SV6.2 part 2 upload | ✅ PASS | part=2 status=200 etag=9cce797ef8944842 crc=9cce797ef8944842 |
| 33 | SV6.2 part 3 upload | ✅ PASS | part=3 status=200 etag=e6941fde6d8ae72f crc=e6941fde6d8ae72f |
| 34 | SV6.2 part 4 upload | ✅ PASS | part=4 status=200 etag=d48282f008f26a93 crc=d48282f008f26a93 |
| 35 | SV6.2 part 5 upload | ✅ PASS | part=5 status=200 etag=879d0f6d7d454a77 crc=879d0f6d7d454a77 |
| 36 | SV6.3 MPU complete returns 200 + n_parts=5 | ✅ PASS | status=200 n_parts=5 agg_crc=f6c02f1404721516 |
| 37 | SV6.4 MPU aggregate CRC matches client concatenation | ✅ PASS | server=f6c02f1404721516 client=f6c02f1404721516 size=350 |
| 38 | SV6.5 GET completed MPU returns 200 | ✅ PASS | status=200 len=350 expected=350 |
| 39 | SV6.6 GET MPU body length matches aggregate bytes | ✅ PASS | got_len=350 want_len=350 |
| 40 | SV6.7 GET MPU x-amz-meta-crc64-ecma matches complete agg CRC | ✅ PASS | complete=f6c02f1404721516 get=f6c02f1404721516 |
