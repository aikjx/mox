const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projCommunityIn2("comm-002");
if (!result.ok || result.tags.length === 0) process.exit(1);
console.log("XJ-OK: graph-018_proj_community_in_2");
process.exit(0);
