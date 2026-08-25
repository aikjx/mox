const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.sparkReaderPagedEdges(25, "0");
if (!result.ok || result.edges.length !== 25) process.exit(1);
console.log("XJ-OK: graph-009_spark_reader_paged_edges");
process.exit(0);
