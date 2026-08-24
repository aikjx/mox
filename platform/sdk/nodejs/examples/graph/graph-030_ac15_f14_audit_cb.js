const { GraphClient } = require("../../xuanji-sdk-graph");
const client = new GraphClient({ endpoint: "graph.local" });
const result = client.ac15F14AuditCb("evt-audit-999");
if (!result.passed || !result.auditLogged || !result.callbackFired) process.exit(1);
console.log("XJ-OK: graph-030_ac15_f14_audit_cb");
process.exit(0);
