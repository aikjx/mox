const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
const token = "mytoken123";
const signature = "sig-" + token;
const result = client.stsTokenSignatureVerify(token, signature);
if (!result.valid) process.exit(1);
console.log("XJ-OK: cloud-014_sts_token_signature_verify");
process.exit(0);
