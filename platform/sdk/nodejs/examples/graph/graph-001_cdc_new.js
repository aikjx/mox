const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.cdcNew("consumer-001", { offset: 0 });
if (!result.ok || result.consumerId !== "consumer-001") process.exit(1);
console.log("XJ-OK: graph-001_cdc_new");
process.exit(0);
