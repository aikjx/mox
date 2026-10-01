# -*- coding: utf-8 -*-
"""Final targeted checks."""
import io, os, re

GATEWAY_ALLIANCE = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance"
DISPATCHER = os.path.join(GATEWAY_ALLIANCE, "experts_dispatcher.rs")
REGISTRY = os.path.join(GATEWAY_ALLIANCE, "experts_registry.rs")
COMMON = os.path.join(GATEWAY_ALLIANCE, "experts_common.rs")
API_TOML = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\api\Cargo.toml"
GRAPH = os.path.join(GATEWAY_ALLIANCE, "experts_graph.rs")
EXECUTOR_BRIDGE = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\core\mox-alliance-scheduler-core\src\executor_bridge.rs"

out = []

# 1. List actual files in gateway/src/alliance
out.append("===== Files in gateway/src/alliance/ =====")
for f in sorted(os.listdir(GATEWAY_ALLIANCE)):
    p = os.path.join(GATEWAY_ALLIANCE, f)
    if os.path.isfile(p):
        sz = os.path.getsize(p)
        out.append("  %s  (%d bytes)" % (f, sz))

# 2. api Cargo.toml name
out.append("\n===== api/Cargo.toml =====")
with io.open(API_TOML, "r", encoding="utf-8") as f:
    out.append(f.read())

# 3. dispatcher.rs: all unwrap/expect (no test filter) with context
out.append("\n===== dispatcher.rs unwrap/expect ALL occurrences =====")
with io.open(DISPATCHER, "r", encoding="utf-8") as f:
    dl = f.readlines()
for i, l in enumerate(dl, 1):
    if re.search(r"\.unwrap\(\)|\.expect\(", l):
        out.append("  %d: %s" % (i, l.rstrip()))

# 4. dispatcher.rs circuit_breakers / engine_status
out.append("\n===== dispatcher.rs circuit_breakers / engine_status / running literal =====")
for i, l in enumerate(dl, 1):
    if any(k in l for k in ["circuit_breakers", "engine_status", "running", "fc_guard", "favorable"]):
        out.append("  %d: %s" % (i, l.rstrip()))

# 5. registry.rs availability.status / health
out.append("\n===== experts_registry.rs availability.status / health =====")
with io.open(REGISTRY, "r", encoding="utf-8") as f:
    rl = f.readlines()
for i, l in enumerate(rl, 1):
    if any(k in l for k in ["availability", "health", "status", "online", "busy", "offline", "away"]):
        if i < 200:  # just first 200 lines
            out.append("  %d: %s" % (i, l.rstrip()))

# 6. favorites: any save/load/persist?
out.append("\n===== favorites persistence across gateway alliance =====")
for fn in os.listdir(GATEWAY_ALLIANCE):
    if not fn.endswith(".rs"): continue
    p = os.path.join(GATEWAY_ALLIANCE, fn)
    with io.open(p, "r", encoding="utf-8") as f:
        for i, l in enumerate(f, 1):
            if "favorite" in l.lower() and any(k in l for k in ["save", "load", "db", "sqlite", "persist", "fs::"]):
                out.append("  %s:%d: %s" % (fn, i, l.rstrip()))

# 7. graph.rs: any write/CRUD handlers (post/put/delete besides rebuild)
out.append("\n===== graph.rs write operations (insert/update/delete node) =====")
with io.open(GRAPH, "r", encoding="utf-8") as f:
    gl = f.readlines()
for i, l in enumerate(gl, 1):
    if any(k in l for k in ["INSERT", "UPDATE graph", "DELETE graph", "add_node", "add_edge", "remove_node", "create_node", "update_node"]):
        out.append("  %d: %s" % (i, l.rstrip()))

# 8. executor_bridge.rs SM4 / crypto
out.append("\n===== executor_bridge.rs crypto/SM4 lines =====")
with io.open(EXECUTOR_BRIDGE, "r", encoding="utf-8") as f:
    bl = f.readlines()
for i, l in enumerate(bl, 1):
    if any(k in l for k in ["crypto", "sm4", "SM4", "seal", "open_response", "outbound_headers", "MOX_API_CRYPTO"]):
        out.append("  %d: %s" % (i, l.rstrip()))

# 9. Count total .rs files in gateway/src/alliance with line counts
out.append("\n===== gateway/src/alliance line counts =====")
total = 0
for fn in sorted(os.listdir(GATEWAY_ALLIANCE)):
    if fn.endswith(".rs"):
        p = os.path.join(GATEWAY_ALLIANCE, fn)
        with io.open(p, "r", encoding="utf-8") as f:
            n = sum(1 for _ in f)
        total += n
        out.append("  %-30s %d lines" % (fn, n))
out.append("  TOTAL: %d lines" % total)

# 10. Check for auth middleware on /api/experts and /api/alliance
out.append("\n===== auth public_paths / which paths skip auth =====")
AUTH = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\config.rs"
with io.open(AUTH, "r", encoding="utf-8") as f:
    acl = f.readlines()
for i, l in enumerate(acl, 1):
    if any(k in l for k in ["public_path", "auth", "jwt", "dev_mode", "enabled"]):
        out.append("  config.rs:%d: %s" % (i, l.rstrip()))

with io.open(os.path.join(os.path.dirname(__file__), "final_checks.txt"), "w", encoding="utf-8") as f:
    f.write("\n".join(out))
print("done")
