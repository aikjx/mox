const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F13LagSpike(15);
if (!result.passed || !result.recovered) process.exit(1);
console.log("XJ-OK: graph-029_ac15_f13_lag_spike");
process.exit(0);
