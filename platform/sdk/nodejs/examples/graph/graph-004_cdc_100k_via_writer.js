const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
client.cdcNew("consumer-004");
const result = client.cdc100kViaWriter("consumer-004", 5000);
if (!result.ok || result.total !== 100000) process.exit(1);
console.log("XJ-OK: graph-004_cdc_100k_via_writer");
process.exit(0);
