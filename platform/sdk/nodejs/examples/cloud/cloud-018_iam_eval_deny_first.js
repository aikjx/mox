const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const result = client.iamEvalDenyFirst(["s3:ListBucket", "s3:DeleteBucket"], "arn::bucket/*");
if (result.decision !== "DENY" || result.denied.length === 0) process.exit(1);
console.log("XJ-OK: cloud-018_iam_eval_deny_first");
process.exit(0);
