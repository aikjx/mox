# -*- coding: utf-8 -*-
import io
files = [
    r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_session.rs",
    r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs",
    r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_graph.rs",
]
old = "graph: Arc::new(Mutex::new(ExpertGraph::default())),"
new = "graph: Arc::new(Mutex::new(HashMap::new())),"
for p in files:
    s = io.open(p, encoding="utf-8").read()
    n = s.count(old)
    s = s.replace(old, new)
    io.open(p, "w", encoding="utf-8", newline="").write(s)
    print(p.split("\\")[-1], "replaced", n)
