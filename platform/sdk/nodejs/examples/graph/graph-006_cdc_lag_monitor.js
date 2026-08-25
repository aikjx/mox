const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
client.cdcNew("consumer-006");
const result = client.cdcLagMonitor("consumer-006");
if (!result.ok || typeof result.lagMs !== "number") process.exit(1);
console.log("XJ-OK: graph-006_cdc_lag_monitor");
process.exit(0);
