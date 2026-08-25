const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const nodes = [{ id: "n1", label: "User" }, { id: "n2", label: "User" }];
const edges = [{ id: "e1", src: "n1", dst: "n2", label: "KNOWS" }];
const result = client.sparkWriterBulk(nodes, edges);
if (!result.ok || result.nodesWritten !== 2 || result.edgesWritten !== 1) process.exit(1);
console.log("XJ-OK: graph-010_spark_writer_bulk");
process.exit(0);
