const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F6Partial(0.03);
if (!result.passed || !result.handled) process.exit(1);
console.log("XJ-OK: graph-025_ac15_f6_partial");
process.exit(0);
