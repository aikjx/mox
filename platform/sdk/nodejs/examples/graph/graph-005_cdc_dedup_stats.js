const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
client.cdcNew("consumer-005");
const result = client.cdcDedupStats("consumer-005");
if (!result.ok || result.totalEvents <= 0) process.exit(1);
console.log("XJ-OK: graph-005_cdc_dedup_stats");
process.exit(0);
