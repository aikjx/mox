const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F3LostZero();
if (!result.passed || result.lossRate !== 0) process.exit(1);
console.log("XJ-OK: graph-024_ac15_f3_lost_zero");
process.exit(0);
