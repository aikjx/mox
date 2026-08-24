const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.sparkReaderPagedNodes(50, "1000");
if (!result.ok || result.nodes.length !== 50) process.exit(1);
console.log("XJ-OK: graph-008_spark_reader_paged_nodes");
process.exit(0);
