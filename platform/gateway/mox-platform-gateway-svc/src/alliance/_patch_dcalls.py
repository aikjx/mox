# -*- coding: utf-8 -*-
import io
p = r"D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\alliance\experts_dispatcher.rs"
s = io.open(p, encoding="utf-8").read()
T = 'TenantId("default".into()), '
reps = [
    ('update_config(State(state.clone()), admin_user(),',
     'update_config(State(state.clone()), ' + T + 'admin_user(),'),
    ('reset_expert(State(state.clone()), admin_user(),',
     'reset_expert(State(state.clone()), ' + T + 'admin_user(),'),
    ('dispatcher_status(State(state.clone()))',
     'dispatcher_status(State(state.clone()), ' + T.rstrip(", " + ")") + ')'),
    ('consult(State(state.clone()), OptionalAuthUser(None),',
     'consult(State(state.clone()), ' + T + 'OptionalAuthUser(None),'),
    ('reset_all(State(state.clone()), admin_user())',
     'reset_all(State(state.clone()), ' + T + 'admin_user())'),
]
for old, new in reps:
    n = s.count(old)
    s = s.replace(old, new)
    print(n, old[:50])
io.open(p, "w", encoding="utf-8", newline="").write(s)
