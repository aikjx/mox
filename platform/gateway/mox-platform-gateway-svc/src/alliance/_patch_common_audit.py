# -*- coding: utf-8 -*-
"""给 common.rs 第二个 emit_audit 测试调用补 tenant 参数。"""
import io

p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_common.rs"
with io.open(p, "r", encoding="utf-8") as f:
    s = f.read()

old = """        emit_audit(
            &state,
            &AuditActor::system(),
            AuditAction::ExpertDispatch,
            "dispatch",
            "disp-test",
            AuditOutcome::Success,
            Some("smoke"),
        );"""
new = """        emit_audit(
            &state,
            &AuditActor::system(),
            DEFAULT_TENANT,
            AuditAction::ExpertDispatch,
            "dispatch",
            "disp-test",
            AuditOutcome::Success,
            Some("smoke"),
        );"""
n = s.count(old)
s = s.replace(old, new)
print("replaced", n)
with io.open(p, "w", encoding="utf-8", newline="") as f:
    f.write(s)
print("OK")
