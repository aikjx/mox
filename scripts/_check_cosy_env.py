"""E-0 依赖+权重盘点（Windows/Py312/CosyVoice2 环境）。"""
import sys, os, importlib.util, pathlib

print("Python:", sys.version)
print("exec   =", sys.executable)
print("prefix =", sys.prefix)
print()

def has(mod: str) -> bool:
    return bool(importlib.util.find_spec(mod))

packages = ["torch", "torchaudio", "numpy", "librosa", "cosyvoice", "modelscope", "transformers", "sherpa_onnx"]
print("--- deps installed? ---")
for m in packages:
    print("  %-22s : %s" % (m, has(m)))

if has("torch"):
    import torch  # type: ignore
    print("  torch version=%s  cuda_available=%s  device_count=%s" % (
        torch.__version__, torch.cuda.is_available(), torch.cuda.device_count(),
    ))

ROOT = r"d:\a10\aikjx\gitcode\infotopograph"
cands = [
    os.path.join(os.path.expanduser("~"), ".mox", "models", "voice", "tts-cosyvoice2-0.5b"),
    os.path.abspath(os.path.join(ROOT, "projects", "xiaobai_voice", "models", "tts-cosyvoice2-0.5b")),
    os.path.join(ROOT, "models", "tts-cosyvoice2-0.5b"),
    os.path.expanduser(r"~\.cache\modelscope\hub\speech_tts\CosyVoice2-0_5b"),
    os.path.expanduser(r"~\.cache\huggingface\hub\models--FunAudioLLM--CosyVoice2-0.5B"),
]
print()
print("--- cosyvoice2 weights candidates ---")
for c in cands:
    p = os.path.expanduser(c)
    ok = os.path.isdir(p)
    mark = "OK" if ok else "--"
    print("  %s  %s" % (mark, p))
    if ok:
        files = list(pathlib.Path(p).iterdir())[:20]
        for f in files:
            t = "D" if f.is_dir() else "F"
            sz = f.stat().st_size if f.is_file() else 0
            print("        %s %12d  %s" % (t, sz, f.name))
