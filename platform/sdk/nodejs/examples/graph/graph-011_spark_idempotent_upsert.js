const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const nodes = [
  { id: "nu1", label: "User" },
  { id: "nu2", label: "User" },
  { id: "nu1", label: "User" }
];
const result = client.sparkIdempotentUpsert(nodes);
if (!result.idempotent || result.updated < 1) process.exit(1);
console.log("XJ-OK: graph-011_spark_idempotent_upsert");
process.exit(0);
