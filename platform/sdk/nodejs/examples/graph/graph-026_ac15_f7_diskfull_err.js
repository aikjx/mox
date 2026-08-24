const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F7DiskfullErr();
if (!result.passed || !result.gracefulDegradation) process.exit(1);
console.log("XJ-OK: graph-026_ac15_f7_diskfull_err");
process.exit(0);
