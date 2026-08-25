# -*- coding: utf-8 -*-
import os, sys, subprocess
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, ROOT)
from core.pipeline import Melody2Score
from core.config import Config

wav = os.path.join(HERE, "twinkle.wav")
if not os.path.exists(wav):
    subprocess.run([sys.executable, os.path.join(HERE, "gen_test_audio.py")], check=True)

# 使用 v1 时代的精确配置：vocal_mode=True + 关分离 + 关降噪 + tiny + conf=0.3
cfg = Config(vocal_mode=True, enable_separation=False, enable_postprocess=True,
             enable_denoise=False, model_size="tiny", conf_thresh=0.30,
             robust=True)
res = Melody2Score(cfg).run(audio_path=wav)
print('Got:  ', [n['midi'] for n in res['notes']])
print('Exp:  ', [60, 60, 67, 67, 69, 69, 67, 65, 65, 64, 64, 62, 62, 60])
print('简谱: ', res['jianpu'])
print('Exp:  ', '1 1 5 5 6 6 5- 4 4 3 3 2 2 1-')
print('BPM:  ', res['bpm'])
print('Backend:', res['backend'], 'Kept=', res['robust_kept'])
print('Separation:', res.get('separation'), 'Post:', res.get('postprocess'))
