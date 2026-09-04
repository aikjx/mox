import json, time, struct, urllib.request, urllib.parse

t = time.time()
with urllib.request.urlopen("http://127.0.0.1:30010/voice/health", timeout=120) as r:
    body = r.read().decode()
print("HC :30010 time=%.2fs body=%s" % (time.time() - t, body[:500]))

params = urllib.parse.urlencode({"text": "你好今天天气晴。", "speed": "1.0", "engine": "cosyvoice2"})
t = time.time()
with urllib.request.urlopen("http://127.0.0.1:30010/voice/tts/stream?" + params, timeout=600) as r:
    code = r.getcode()
    hdrs = dict(r.headers.items())
    data = r.read()
dt = time.time() - t
ct = hdrs.get("Content-Type", "")
eng = hdrs.get("X-TTS-Engine", "")
dsp = hdrs.get("X-TTS-DSP-Impl", "")
cl = hdrs.get("Content-Length", "")
print("TTS :30010 time=%.2fs code=%d len=%d CT=%s Engine=%s DSP=%s CL=%s" % (dt, code, len(data), ct, eng, dsp, cl))
