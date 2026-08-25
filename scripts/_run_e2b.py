import sys, os, wave, io, time
t0 = time.time()
sys.path.insert(0, r'D:\a10\aikjx\gitcode\infotopograph\projects\xiaobai_voice')
from xiaobai_voice.tts.cosyvoice2 import CosyVoice2Backend, rust_dsp_available
from xiaobai_voice.tts import TTSOptions

print('[0s] rust_dsp_available:', rust_dsp_available())
cfg = {'sample_rate': 22050, 'speed': 1.03}

class FakeReg: pass
print(f'[{time.time()-t0:.1f}s] Instantiating...')
tts = CosyVoice2Backend(cfg, FakeReg())
dsp_impl_before = getattr(tts, "_last_dsp_impl", None)
print(f'[{time.time()-t0:.1f}s] name:', tts.name, 'spk_id:', repr(getattr(tts,'_resolved_spk_id',None)))
fe = getattr(getattr(tts,'_model',None),'frontend',None)
if fe is not None:
    print(f'[{time.time()-t0:.1f}s] spk2info keys:', list(getattr(fe,'spk2info',{}).keys())[:10])

opts = TTSOptions(text='你好，璇玑系统。今天天气真不错，适合听一首动听的音乐。', voice='xiaobai', emotion='happy', speed=1.03, sample_rate=22050)
print(f'[{time.time()-t0:.1f}s] Synthesizing...')
wav_bytes = tts.synthesize_full(opts)
dsp_impl_after = getattr(tts, "_last_dsp_impl", None)
print(f'[{time.time()-t0:.1f}s] synth done: bytes={len(wav_bytes)}, dsp_impl_before={dsp_impl_before}, dsp_impl_after={dsp_impl_after}')
with io.BytesIO(wav_bytes) as f:
    with wave.open(f,'rb') as w:
        sr=w.getframerate(); ch=w.getnchannels(); sw=w.getsampwidth(); nf=w.getnframes()
        dur=nf/sr if sr else 0
print(f'WAV: sr={sr}Hz ch={ch} sw={sw}B dur={dur:.2f}s size={len(wav_bytes)}B')
ok = (
    rust_dsp_available()==True and tts.name=='cosyvoice2' and
    dsp_impl_after=='Rust' and sr==22050 and ch==1 and dur>0.5 and len(wav_bytes)>1000
)
print(f'[{time.time()-t0:.1f}s] E-2b PASS:', ok)
reports_dir = r'D:\a10\aikjx\gitcode\infotopograph\projects\xiaobai_voice\reports'
os.makedirs(reports_dir, exist_ok=True)
out_p = os.path.join(reports_dir,'e2b_synth_rust_dsp.wav')
with open(out_p,'wb') as fp: fp.write(wav_bytes)
print(f'Saved: {out_p}')
sys.exit(0 if ok else 2)
