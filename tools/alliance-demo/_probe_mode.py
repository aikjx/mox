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
