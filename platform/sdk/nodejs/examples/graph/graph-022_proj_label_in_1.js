const { GraphClient } = require("../../mox-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.projLabelIn1("node-multi", ["Employee", "Manager"]);
if (!result.ok || result.labelsApplied.length !== 2) process.exit(1);
console.log("XJ-OK: graph-022_proj_label_in_1");
process.exit(0);
