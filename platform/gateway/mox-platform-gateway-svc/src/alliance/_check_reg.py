# -*- coding: utf-8 -*-
import io, sys
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_registry.rs"
s = io.open(p, "r", encoding="utf-8").read()
print("TenantId(default) occurrences:", s.count('TenantId("default".into())'))
print("graph HashMap new:", s.count("graph: Arc::new(Mutex::new(HashMap::new()))"))
print("ExpertGraph::default remaining:", s.count("ExpertGraph::default"))
print("old list_experts call remains:", s.count("list_experts(State(state), Query(params))"))
