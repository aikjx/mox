# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs"
s = io.open(p, encoding="utf-8").read()
reps = [
 ('if let Some(e) = registry.get_mut("exp-ai-001") {',
  'if let Some(e) = registry.get_mut("default").unwrap().get_mut("exp-ai-001") {'),
 ('assert_eq!(registry.get("exp-ai-001").unwrap().availability.current_load, 0);',
  'assert_eq!(registry.get("default").unwrap().get("exp-ai-001").unwrap().availability.current_load, 0);'),
 ('for e in registry.values_mut() {',
  'for e in registry.get_mut("default").unwrap().values_mut() {'),
 ('for e in registry.values() {',
  'for e in registry.get("default").unwrap().values() {'),
]
for old, new in reps:
    n = s.count(old); s = s.replace(old, new); print(n, old[:45])
io.open(p, "w", encoding="utf-8", newline="").write(s)
