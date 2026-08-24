const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
client.cdcNew("consumer-old-7");
const result = client.cdcConsumerIdRotate("consumer-old-7", "consumer-new-7");
if (!result.rotated) process.exit(1);
console.log("XJ-OK: graph-007_cdc_consumer_id_rotate");
process.exit(0);
