const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F12TimeoutDedup(3000);
if (!result.passed || result.duplicatesHandled < 1) process.exit(1);
console.log("XJ-OK: graph-028_ac15_f12_timeout_dedup");
process.exit(0);
