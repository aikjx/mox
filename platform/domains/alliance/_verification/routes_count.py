# -*- coding: utf-8 -*-
"""Extract actuator.rs ROUTES and count /api/experts and /api/alliance paths."""
import io, re, os

ACTUATOR = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\actuator.rs"
COMMON = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_common.rs"
HA = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\svc\mox-alliance-scheduler-svc\src\ha.rs"
DISPATCHER = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs"
ORCH = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_orchestration.rs"
GRAPH = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_graph.rs"
METRICS = r"D:\a10\aikjx\gitcode\infotopograph\platform\domains\alliance\core\mox-alliance-scheduler-core\src\metrics.rs"

def read(p):
    with io.open(p, "r", encoding="utf-8") as f:
        return f.readlines()

out = []

# 1. Extract all r("...", "METHOD", "/path", ...) lines from actuator.rs
lines = read(ACTUATOR)
exp_paths = set()
all_paths = set()
exp_rows = []
all_rows = []
route_re = re.compile(r'r\("([^"]+)",\s*"([A-Z]+|ANY)",\s*"([^"]+)"')
for i, l in enumerate(lines, 1):
    m = route_re.search(l)
    if m:
        rid, method, path = m.group(1), m.group(2), m.group(3)
        if path.startswith("/api/experts"):
            exp_paths.add(path)
            exp_rows.append((i, method, path, rid))
        elif path.startswith("/api/alliance"):
            all_paths.add(path)
            all_rows.append((i, method, path, rid))

out.append("===== /api/experts/* unique paths: %d =====" % len(exp_paths))
for p in sorted(exp_paths):
    out.append("  " + p)
out.append("")
out.append("===== /api/alliance/* unique paths: %d =====" % len(all_paths))
for p in sorted(all_paths):
    out.append("  " + p)
out.append("")
out.append("===== /api/experts/* rows (method-level): %d =====" % len(exp_rows))
for i, m, p, rid in exp_rows:
    out.append("  actuator.rs:%d  %-5s %s  [%s]" % (i, m, p, rid))
out.append("")
out.append("===== /api/alliance/* rows (method-level): %d =====" % len(all_rows))
for i, m, p, rid in all_rows:
    out.append("  actuator.rs:%d  %-5s %s  [%s]" % (i, m, p, rid))

# 2. experts_common.rs state fields around 456-475
out.append("\n\n===== experts_common.rs 450-480 (shared state fields) =====")
cl = read(COMMON)
for i in range(449, min(480, len(cl))):
    out.append("%d: %s" % (i+1, cl[i].rstrip()))

# 3. favorites / plans / orchestration_history grep in common
out.append("\n===== experts_common.rs favorites/plans/orchestration_history =====")
for i, l in enumerate(cl, 1):
    if any(k in l for k in ["favorites", "plans", "orchestration_history", "HashSet", "HashMap<String, CollaborationPlan>", "Vec<OrchestrationRecord>"]):
        out.append("%d: %s" % (i, l.rstrip()))

# 4. ha.rs reconcile / orphan takeover
out.append("\n===== ha.rs reconcile/orphan/leader loop =====")
hl = read(HA)
for i, l in enumerate(hl, 1):
    if any(k in l for k in ["reconcile", "orphan", "stall", "leader", "accepts", "takeover", "对账", "孤儿", "接管"]):
        out.append("%d: %s" % (i, l.rstrip()))

# 5. dispatcher unwrap/expect panic points
out.append("\n===== dispatcher.rs unwrap/expect (non-test) =====")
dl = read(DISPATCHER)
in_test = False
for i, l in enumerate(dl, 1):
    if "#[cfg(test)]" in l or "#[test]" in l:
        in_test = True
    if re.search(r"\.unwrap\(\)|\.expect\(", l) and not in_test:
        out.append("%d: %s" % (i, l.rstrip()))

# 6. orchestration.rs route registrations
out.append("\n===== orchestration.rs .route( calls =====")
ol = read(ORCH)
for i, l in enumerate(ol, 1):
    if ".route(" in l:
        out.append("%d: %s" % (i, l.rstrip()))

# 7. graph.rs route registrations + CRUD
out.append("\n===== graph.rs .route( calls =====")
gl = read(GRAPH)
for i, l in enumerate(gl, 1):
    if ".route(" in l:
        out.append("%d: %s" % (i, l.rstrip()))

# 8. metrics.rs content
out.append("\n===== scheduler-core/metrics.rs (first 80 lines) =====")
ml = read(METRICS)
for i in range(min(80, len(ml))):
    out.append("%d: %s" % (i+1, ml[i].rstrip()))

with io.open(os.path.join(os.path.dirname(__file__), "routes_count.txt"), "w", encoding="utf-8") as f:
    f.write("\n".join(out))
print("done")
