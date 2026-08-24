import sys, os, pytest
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from xuanji_sdk_graph import GraphClient

EXAMPLES_DIR = os.path.join(os.path.dirname(__file__), "..", "examples", "graph")

GRAPH_IDS = [
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
]


# Parametrize first 7 IDs
@pytest.mark.parametrize("ex_id", GRAPH_IDS[:7])
def test_graph_example_file_exists(ex_id):
    files = os.listdir(EXAMPLES_DIR)
    found = False
    for f in files:
        if not f.endswith(".py"): continue
        base = f[:-3]
        if base == ex_id or base.startswith(ex_id + "_"):
            found = True
            break
    assert found, f"Example file missing for {ex_id}"


def test_all_30_graph_ids_have_files():
    files = os.listdir(EXAMPLES_DIR)
    for ex_id in GRAPH_IDS:
        found = False
        for f in files:
            if not f.endswith(".py"): continue
            base = f[:-3]
            if base == ex_id or base.startswith(ex_id + "_"):
                found = True
                break
        assert found, f"Missing example file for {ex_id}"


# --- GraphClient method tests ---
def test_cdc_new_creates_consumer():
    client = GraphClient()
    r = client.cdcNew("c1", {"offset": 5})
    assert r["ok"] is True
    assert r["consumerId"] == "c1"
    assert r["offset"] == 5


def test_cdc_next_blocking_returns_events():
    client = GraphClient()
    client.cdcNew("c2")
    r = client.cdcNextBlocking("c2")
    assert r["ok"] is True
    assert isinstance(r["events"], list)
    assert len(r["events"]) > 0


def test_cdc_resume_offset_sets_offset():
    client = GraphClient()
    client.cdcNew("c3")
    r = client.cdcResumeOffset("c3", 1234)
    assert r["resumedOffset"] == 1234


def test_cdc_100k_writes_total():
    client = GraphClient()
    client.cdcNew("c4")
    r = client.cdc100kViaWriter("c4", 1000)
    assert r["total"] == 100000
    assert r["written"] == 100000


def test_spark_reader_nodes_page_size():
    client = GraphClient()
    r = client.sparkReaderPagedNodes(33, "0")
    assert len(r["nodes"]) == 33
    assert r["nextPageToken"] == "33"


def test_spark_reader_edges_page_size():
    client = GraphClient()
    r = client.sparkReaderPagedEdges(5, "10")
    assert len(r["edges"]) == 5


def test_spark_writer_bulk_counts():
    client = GraphClient()
    nodes = [{"id": "a"}, {"id": "b"}, {"id": "c"}, {"id": "d"}]
    edges = [{"id": "e1", "src": "a", "dst": "b"}, {"id": "e2", "src": "c", "dst": "d"}]
    r = client.sparkWriterBulk(nodes, edges)
    assert r["nodesWritten"] == 4
    assert r["edgesWritten"] == 2
    assert r["total"] == 6


def test_spark_idempotent_upsert_counts():
    client = GraphClient()
    nodes = [{"id": "n1"}, {"id": "n2"}, {"id": "n1"}]
    r = client.sparkIdempotentUpsert(nodes)
    assert r["idempotent"] is True
    assert r["total"] == 3
    assert r["inserted"] + r["updated"] == 3


def test_spark_roundtrip_2k3k_consistency():
    client = GraphClient()
    r = client.sparkRoundtrip2k3k()
    assert r["consistency"] == "verified"
    assert r["nodesWritten"] == 2048


def test_proj_type_out_1_graph():
    client = GraphClient()
    r = client.projTypeOut1("n1")
    assert r["projectType"] == "GRAPH"
    assert r["schemaVersion"] == 1


def test_proj_attr_in_imported_count():
    client = GraphClient()
    r = client.projAttrIn("n1", {"a": 1, "b": 2, "c": 3})
    assert r["imported"] == 3
    assert r["attributes"]["_imported"] is True


def test_proj_degree_out_2_value():
    client = GraphClient()
    r = client.projDegreeOut2("n1")
    assert r["outDegree"] == 2
    assert len(r["neighbors"]) == 2


def test_ac15_f3_zero_loss():
    client = GraphClient()
    r = client.ac15F3LostZero()
    assert r["passed"] is True
    assert r["lossRate"] == 0
    assert r["eventsIn"] == r["eventsOut"]


def test_ac15_f7_diskfull_graceful():
    client = GraphClient()
    r = client.ac15F7DiskfullErr()
    assert r["passed"] is True
    assert r["gracefulDegradation"] is True
    assert r["errorInjected"] == "DISK_FULL"


def test_ac15_f14_audit_cb_logged():
    client = GraphClient()
    r = client.ac15F14AuditCb("evt-100")
    assert r["passed"] is True
    assert r["auditLogged"] is True
    assert r["callbackFired"] is True
    assert r["auditEntry"]["id"] == "evt-100"
