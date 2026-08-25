const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.sparkStatsAccumulate();
if (!result.ok || !result.accumulated.totalTransactions) process.exit(1);
console.log("XJ-OK: graph-014_spark_stats_accumulate");
process.exit(0);
