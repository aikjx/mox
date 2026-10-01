# -*- coding: utf-8 -*-
"""Batch grep: run many patterns, dump all hits to one file for review."""
import os, re, io, sys

ROOTS = [
    r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance",
    r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src",
]

PATTERNS = [
    ("env_port_3100", r"3100"),
    ("env_port_3200", r"3200"),
    ("env_port_3400", r"3400"),
    ("env_port_3080", r"3080"),
    ("MOX_", r"MOX_[A-Z_]+"),
    ("WebSocketUpgrade", r"WebSocketUpgrade"),
    ("WAL", r"WAL|journal_mode|wal_mode|busy_timeout|busy_timeout"),
    ("leadership_traits", r"LeaseStore|LeaderElector|SqliteLeaseStore"),
    ("aggregation", r"aggregated.heartbeat|aggregation\.rs|node.*rack.*cell|10:1:1"),
    ("SM4", r"sm4|SM4|gzip|GzDecoder|GzEncoder"),
    ("STORAGE_MODE", r"MOX_ALLIANCE_STORAGE_MODE|storage_mode|StorageMode"),
    ("REMOTE_MODE", r"MOX_ALLIANCE_REMOTE_MODE|remote_mode|RemoteMode"),
    ("HA_MODE", r"MOX_ALLIANCE_HA_MODE|ha_mode|HaMode|HA_MODE"),
    ("fencing", r"fencing|Fencing|epoch|term|lease"),
    ("unwrap_in_gateway", r"\.unwrap\(\)|\.expect\("),
    ("fs_write", r"fs::write|write_all|File::create"),
    ("audit", r"audit|Audit"),
    ("booking", r"booking|Booking|reservation"),
    ("health_probe", r"health_probe|HealthProbe|health_check"),
    ("config_sync", r"config_sync|ConfigSync"),
    ("llm_router", r"llm_router|LlmRouter|circuit|CircuitBreaker|breaker"),
    ("dag_retry", r"retry|timeout|parallel|max_parallel|concurrency"),
    ("migration", r"migrat|schema_version|user_version|PRAGMA user_version"),
    ("metrics", r"/metrics|prometheus|Prometheus|Counter|Gauge"),
    ("auth_middleware", r"middleware|auth|Auth|Authorization|Bearer|jwt|JWT"),
    ("graph_table", r"graph_nodes|graph_edges|graph_meta|CREATE TABLE"),
    ("session_table", r"sessions|session_messages|CREATE TABLE"),
    ("experts_table", r"CREATE TABLE|experts\b"),
]

def iter_files():
    for root in ROOTS:
        for dp, dn, fn in os.walk(root):
            dn[:] = [d for d in dn if d not in ("target", ".git", "node_modules", "_verification")]
            for f in fn:
                if f.endswith((".rs", ".toml")):
                    yield os.path.join(dp, f)

def short(p):
    return p.replace("D:\\a10\\aikjx\\gitcode\\infotopograph\\platform\\", "")

out = []
for label, pat in PATTERNS:
    rx = re.compile(pat)
    hits = []
    for path in iter_files():
        try:
            with io.open(path, "r", encoding="utf-8") as f:
                for i, line in enumerate(f, 1):
                    if rx.search(line):
                        hits.append((path, i, line.rstrip()))
        except Exception:
            pass
    out.append("\n\n========== [%s] pattern=%s  hits=%d ==========" % (label, pat, len(hits)))
    # cap hits per pattern to keep file manageable
    for p, i, l in hits[:120]:
        out.append("%s:%d: %s" % (short(p), i, l.strip()[:180]))
    if len(hits) > 120:
        out.append("... (%d more hits truncated)" % (len(hits)-120))

with io.open(os.path.join(os.path.dirname(__file__), "batch_grep.txt"), "w", encoding="utf-8") as f:
    f.write("\n".join(out))
print("done")
