const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projCommunityIn1("comm-001");
if (!result.ok || result.nodes < 100) process.exit(1);
console.log("XJ-OK: graph-017_proj_community_in_1");
process.exit(0);
