const { CloudClient } = require("../../mox-sdk-cloud");
const client = new CloudClient({ region: "cn-east-1" });
client.createBucket("lh-bucket");
client.putObject("lh-bucket", "hold.txt", "legal-hold-content");
client.wormLegalHoldOnOff("lh-bucket", "hold.txt", true);
const result = client.wormLegalHoldOnOff("lh-bucket", "hold.txt", false);
if (result.legalHold !== "OFF") process.exit(1);
console.log("XJ-OK: cloud-023_worm_legal_hold_on_off");
process.exit(0);
