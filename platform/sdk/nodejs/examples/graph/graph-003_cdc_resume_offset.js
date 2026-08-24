const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
client.cdcNew("consumer-003");
const result = client.cdcResumeOffset("consumer-003", 42);
if (!result.ok || result.resumedOffset !== 42) process.exit(1);
console.log("XJ-OK: graph-003_cdc_resume_offset");
process.exit(0);
