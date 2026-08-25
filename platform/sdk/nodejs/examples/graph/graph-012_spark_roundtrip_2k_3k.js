const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.sparkRoundtrip2k3k();
if (!result.ok || result.consistency !== "verified") process.exit(1);
console.log("XJ-OK: graph-012_spark_roundtrip_2k_3k");
process.exit(0);
