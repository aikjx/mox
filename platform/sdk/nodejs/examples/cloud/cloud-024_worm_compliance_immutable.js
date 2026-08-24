const { CloudClient } = require("../../xuanji-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("comp-bucket");
client.putObject("comp-bucket", "imm.txt", "immutable content");
client.wormRetention1y("comp-bucket", "imm.txt");
const result = client.wormComplianceImmutable("comp-bucket", "imm.txt");
if (!result.immutable || result.canDelete) process.exit(1);
console.log("XJ-OK: cloud-024_worm_compliance_immutable");
process.exit(0);
