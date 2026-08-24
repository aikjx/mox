const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.sparkRoundtrip5k8k();
if (!result.ok || result.nodesWritten !== 5120) process.exit(1);
console.log("XJ-OK: graph-013_spark_roundtrip_5k_8k");
process.exit(0);
