# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs"
s = io.open(p, encoding="utf-8").read()

# 多行调用: dispatch_task(\n<indent>&state,  -> 插 "default",
n1 = s.count("dispatch_task(\n            &state,\n")
s = s.replace("dispatch_task(\n            &state,\n",
              "dispatch_task(\n            &state,\n            \"default\",\n")
# 单行调用: dispatch_task(&state, "consult", "test", None)
n2 = s.count('dispatch_task(&state, "consult", "test", None)')
s = s.replace('dispatch_task(&state, "consult", "test", None)',
              'dispatch_task(&state, "default", "consult", "test", None)')
io.open(p, "w", encoding="utf-8", newline="").write(s)
print("multi", n1, "single", n2)
