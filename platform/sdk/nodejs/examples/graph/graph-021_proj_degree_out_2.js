const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projDegreeOut2("hub-node");
if (!result.ok || result.outDegree !== 2) process.exit(1);
console.log("XJ-OK: graph-021_proj_degree_out_2");
process.exit(0);
