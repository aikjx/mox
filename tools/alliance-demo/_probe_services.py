import urllib.request as u

def probe(label, base, path):
    url = base + path
    try:
        r = u.urlopen(url, timeout=4)
        body = r.read(160).decode("utf-8", "replace")
        print("%s %s%s -> %d %s" % (label, base, path, r.status, body.replace("\n", " ")[:160]))
    except Exception as e:  # noqa: BLE001
        print("%s %s%s -> ERR %s %s" % (label, base, path, type(e).__name__, str(e)[:90]))

SCHED = "http://127.0.0.1:3100"
EXEC = "http://127.0.0.1:3200"
probe("sched", SCHED, "/tasks")
probe("sched", SCHED, "/")
probe("sched", SCHED, "/health")
probe("sched", SCHED, "/experts/search")
probe("exec", EXEC, "/tasks/x/status")
probe("exec", EXEC, "/")
probe("exec", EXEC, "/health")
probe("exec", EXEC, "/result")
