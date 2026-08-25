const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F8CbPlusAudit();
if (!result.passed || !result.callbackInvoked) process.exit(1);
console.log("XJ-OK: graph-027_ac15_f8_cb_plus_audit");
process.exit(0);
