"""A-1 下载 tts-cosyvoice2-0.5b 权重（modelscope 国内源优先，带进度）。"""
import datetime, sys, os

sys.path.insert(0, r"d:\a10\aikjx\gitcode\infotopograph\projects\xiaobai_voice")

def cb(p):
    t = datetime.datetime.now().strftime("%H:%M:%S")
    model = p.get("model_id")
    state = p.get("state")
    pct = p.get("progress_pct", 0.0) or 0.0
    spd = p.get("speed_mbps", 0.0) or 0.0
    eta = p.get("eta_s", 0.0) or 0.0
    sys.stdout.write("[%s] model=%s state=%s pct=%.1f%% speed=%.2fMB/s eta=%.0fs\n" % (t, model, state, pct, spd, eta))
    sys.stdout.flush()

from xiaobai_voice.models.downloader import ModelRegistry, ModelDownloader

reg = ModelRegistry()
dl = ModelDownloader(reg)
print("download tts-cosyvoice2-0.5b ->", dl.preferred_root, flush=True)
out = dl.download("tts-cosyvoice2-0.5b", on_progress=cb)
print("EXTRACTED_DIR =", out, flush=True)
print("CONTENTS:")
for f in sorted(os.listdir(out)):
    fp = os.path.join(out, f)
    sz = os.path.getsize(fp) if os.path.isfile(fp) else 0
    kind = "F" if os.path.isfile(fp) else "D"
    print("  %s %12d  %s" % (kind, sz, f))
