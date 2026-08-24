const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projTypeOut2("node-xyz");
if (!result.ok || result.schemaVersion !== 2) process.exit(1);
console.log("XJ-OK: graph-016_proj_type_out_2");
process.exit(0);
