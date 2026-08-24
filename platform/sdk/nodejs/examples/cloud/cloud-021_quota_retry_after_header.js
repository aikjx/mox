const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const result = client.quotaRetryAfterHeader(150);
if (!result.throttled || result.retryAfterSeconds <= 0) process.exit(1);
console.log("XJ-OK: cloud-021_quota_retry_after_header");
process.exit(0);
