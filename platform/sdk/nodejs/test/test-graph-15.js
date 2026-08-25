const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { GraphClient } = require("../mox-sdk-graph");

const EXAMPLES_DIR = path.join(__dirname, "..", "examples", "graph");

const GRAPH_IDS = [
  "graph-001_cdc_new", "graph-002_cdc_next_blocking", "graph-003_cdc_resume_offset",
  "graph-004_cdc_100k_via_writer", "graph-005_cdc_dedup_stats", "graph-006_cdc_lag_monitor",
  "graph-007_cdc_consumer_id_rotate", "graph-008_spark_reader_paged_nodes",
  "graph-009_spark_reader_paged_edges", "graph-010_spark_writer_bulk",
  "graph-011_spark_idempotent_upsert", "graph-012_spark_roundtrip_2k_3k",
  "graph-013_spark_roundtrip_5k_8k", "graph-014_spark_stats_accumulate",
  "graph-015_proj_type_out_1", "graph-016_proj_type_out_2",
  "graph-017_proj_community_in_1", "graph-018_proj_community_in_2",
  "graph-019_proj_attr_out", "graph-020_proj_attr_in",
  "graph-021_proj_degree_out_2", "graph-022_proj_label_in_1",
  "graph-023_ac15_f1_double_idempotent", "graph-024_ac15_f3_lost_zero",
  "graph-025_ac15_f6_partial", "graph-026_ac15_f7_diskfull_err",
  "graph-027_ac15_f8_cb_plus_audit", "graph-028_ac15_f12_timeout_dedup",
  "graph-029_ac15_f13_lag_spike", "graph-030_ac15_f14_audit_cb"
];

describe("Graph SDK Example ID Existence", function () {
  GRAPH_IDS.forEach(function (id) {
    it(`example file exists for ${id}`, function () {
      const files = fs.readdirSync(EXAMPLES_DIR);
      const found = files.find(function (f) {
        if (!f.endsWith(".js")) return false;
        const base = f.slice(0, -3);
        return base === id || base.startsWith(id + "_");
      });
      assert.ok(found, `Example file not found for ID: ${id}`);
    });
  });
});

describe("GraphClient Core Methods", function () {
  it("cdcNew creates consumer with offset", function () {
    const client = new GraphClient();
    const r = client.cdcNew("c1", { offset: 10 });
    assert.strictEqual(r.ok, true);
    assert.strictEqual(r.consumerId, "c1");
  });

  it("cdcNextBlocking returns events array", function () {
    const client = new GraphClient();
    client.cdcNew("c2");
    const r = client.cdcNextBlocking("c2");
    assert.strictEqual(r.ok, true);
    assert.ok(Array.isArray(r.events));
    assert.ok(r.events.length > 0);
  });

  it("cdcResumeOffset sets offset correctly", function () {
    const client = new GraphClient();
    client.cdcNew("c3");
    const r = client.cdcResumeOffset("c3", 999);
    assert.strictEqual(r.resumedOffset, 999);
  });

  it("sparkReaderPagedNodes returns requested page size", function () {
    const client = new GraphClient();
    const r = client.sparkReaderPagedNodes(20, "0");
    assert.strictEqual(r.nodes.length, 20);
    assert.ok(r.nextPageToken);
  });

  it("sparkWriterBulk returns correct counts", function () {
    const client = new GraphClient();
    const nodes = [{ id: "a" }, { id: "b" }, { id: "c" }];
    const edges = [{ id: "e1", src: "a", dst: "b" }];
    const r = client.sparkWriterBulk(nodes, edges);
    assert.strictEqual(r.nodesWritten, 3);
    assert.strictEqual(r.edgesWritten, 1);
  });

  it("sparkIdempotentUpsert counts inserts vs updates", function () {
    const client = new GraphClient();
    const r = client.sparkIdempotentUpsert([{ id: "n1" }, { id: "n1" }]);
    assert.strictEqual(r.idempotent, true);
    assert.ok(r.total === 2);
  });

  it("ac15F3LostZero reports zero loss rate", function () {
    const client = new GraphClient();
    const r = client.ac15F3LostZero();
    assert.strictEqual(r.passed, true);
    assert.strictEqual(r.lossRate, 0);
  });

  it("ac15F14AuditCb logs audit and fires callback", function () {
    const client = new GraphClient();
    const r = client.ac15F14AuditCb("evt-1");
    assert.strictEqual(r.auditLogged, true);
    assert.strictEqual(r.callbackFired, true);
  });
});
