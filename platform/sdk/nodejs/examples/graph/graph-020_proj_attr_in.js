const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projAttrIn("node-imp-1", { score: 99, level: "gold" });
if (!result.imported || result.imported < 1) process.exit(1);
console.log("XJ-OK: graph-020_proj_attr_in");
process.exit(0);
