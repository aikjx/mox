# -*- coding: utf-8 -*-
import io
targets = {
 r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_session.rs":[184,411,452,617],
 r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_collaboration.rs":[1739,1790,1817,1843],
 r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs":[990,1008,1022,1034,1055,1091,1136,1152,1175,1193,1215,949],
 r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_graph.rs":[2243],
}
for p, lns in targets.items():
    L = io.open(p, encoding="utf-8").read().splitlines()
    print("###", p.split("\\")[-1])
    for n in lns:
        print(n, "|", L[n-1][:160])
