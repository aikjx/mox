const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projTypeOut1("node-abc");
if (!result.ok || result.projectType !== "GRAPH") process.exit(1);
console.log("XJ-OK: graph-015_proj_type_out_1");
process.exit(0);
