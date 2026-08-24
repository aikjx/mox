const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
client.cdcNew("consumer-002");
const result = client.cdcNextBlocking("consumer-002");
if (!result.ok || result.events.length === 0) process.exit(1);
console.log("XJ-OK: graph-002_cdc_next_blocking");
process.exit(0);
