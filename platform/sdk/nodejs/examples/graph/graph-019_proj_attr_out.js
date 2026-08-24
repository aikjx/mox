const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projAttrOut("node-person-1");
if (!result.exported || !result.attributes.name) process.exit(1);
console.log("XJ-OK: graph-019_proj_attr_out");
process.exit(0);
