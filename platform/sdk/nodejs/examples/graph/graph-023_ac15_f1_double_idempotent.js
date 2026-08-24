const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F1DoubleIdempotent("op-idempotent-1");
if (!result.passed || !result.sameResult) process.exit(1);
console.log("XJ-OK: graph-023_ac15_f1_double_idempotent");
process.exit(0);
