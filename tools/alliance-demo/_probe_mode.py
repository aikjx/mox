import urllib.request as u, json

def get(url, token="dev-secret-token"):
    req = u.Request(url, headers={"Authorization": "Bearer " + token, "Accept": "application/json"})
    try:
        return u.urlopen(req, timeout=5).read().decode("utf-8", "replace")
    except Exception as e:  # noqa: BLE001
        return "ERR %s %s" % (type(e).__name__, str(e)[:120])

gw = "http://127.0.0.1:3080"
print("== gateway /api/alliance/tasks ==")
print(get(gw + "/api/alliance/tasks")[:600])
print()
print("== scheduler /tasks ==")
print(get("http://127.0.0.1:3100/tasks")[:600])
print()
print("== gateway /api/alliance/stats ==")
print(get(gw + "/api/alliance/stats")[:400])

# 复用演示脚本自身的模式判定，验证其结论与上面手工观察一致
import importlib.util

_spec = importlib.util.spec_from_file_location("alliance_demo", r"tools\alliance-demo\alliance_demo.py")
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)
print()
print("== detect_topology（演示脚本判定） ==")
print(json.dumps(_mod.detect_topology(_mod.GatewayClient(gw, 10, "dev-secret-token")),
                 ensure_ascii=False, indent=2))
