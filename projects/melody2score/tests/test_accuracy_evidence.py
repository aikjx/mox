"""Accuracy regressions: acoustic evidence and strict note evaluation."""
import sys
from pathlib import Path
from types import SimpleNamespace
import numpy as np

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))


def test_denoise_does_not_treat_first_note_as_noise():
    from core.preprocess import preprocess
    sr=16000
    t=np.arange(16003)/sr
    y=np.sin(2*np.pi*440*t).astype(np.float32)
    result=preprocess(y,sr,True)
    assert len(result)==len(y)
    assert np.sqrt(np.mean(result**2)) > .6
    assert np.argmax(abs(np.fft.rfft(result))) in (440,441)


def test_denoise_quiet_tail_preserves_length_and_note():
    from core.preprocess import preprocess
    sr=16000
    y=np.concatenate([np.sin(2*np.pi*440*np.arange(8000)/sr),np.zeros(4003)]).astype(np.float32)
    result=preprocess(y,sr,True)
    assert len(result)==len(y)
    assert np.sqrt(np.mean(result[:7000]**2)) > .6


def test_pyin_centered_frames_have_no_extra_fft_shift(monkeypatch):
    import librosa
    from core.pitch import PitchDetector
    def predict(*args,**kwargs):
        return np.array([440.,440.,440.]),np.ones(3,dtype=bool),np.ones(3)
    monkeypatch.setattr(librosa,'pyin',predict)
    points=PitchDetector(hop=10)._detect_pyin(np.zeros(1600),16000)
    assert np.allclose([p['t'] for p in points],[0,.02,.04])


def test_confidence_is_acoustic_not_note_length():
    from core.pipeline import _conf,_quality
    assert _conf([{'conf':.3},{'conf':.5}]) == .4
    assert _conf([])==0
    notes=[{'midi':60,'start':0.,'end':2.}]
    det=SimpleNamespace(used_backend='pyin',preferred_backend='torchcrepe')
    quality=_quality(notes,[],det,{'strategy':'hpss'})
    assert quality['accuracy'] is None
    assert quality['status']=='needs_review'
    assert quality['review_note_indices']==[0]
    assert len(quality['warnings'])==3


def test_cache_results_are_not_mutable_across_requests():
    from core.pipeline import _LRUCache
    cache=_LRUCache()
    source={'notes':[{'midi':60}]}
    cache.put('key',source)
    source['notes'][0]['midi']=20
    first=cache.get('key')
    assert first['notes'][0]['midi']==60
    first['notes'].clear()
    assert cache.get('key')['notes']==[{'midi':60}]


def test_known_instrument_sample_does_not_use_vocal_separator():
    from core.config import Config,config_for_sample
    base=Config()
    cfg=config_for_sample(base,'instrument')
    assert not cfg.vocal_mode and not cfg.enable_vad and not cfg.enable_separation
    assert base.vocal_mode
    assert config_for_sample(base,'user') is base


def test_metrics_count_octave_errors_and_duplicates():
    from scripts.benchmark_accuracy import note_metrics
    expected=[{'midi':60,'start':0.,'end':.5}]
    wrong=[{'midi':72,'start':0.,'end':.5}]
    assert note_metrics(expected,wrong)['f1']==0
    assert note_metrics(expected,expected*2)['precision']==.5
    late=[{'midi':60,'start':.08,'end':.58}]
    assert note_metrics(expected,late,.05)['recall']==0
    assert note_metrics(expected,late,.15)['recall']==1


def test_ai_review_marks_disagreements_without_changing_notes():
    from core.ai_review import compare_notes
    notes=[{'midi':60,'start':0.,'end':.4},{'midi':64,'start':.5,'end':.9}]
    points=[{'t':t,'freq':440*2**((62-69)/12),'conf':.9} for t in np.arange(.02,.39,.01)]
    result=compare_notes(notes,points)
    assert result['details'][0]['status']=='disagree'
    assert result['details'][0]['peer_midi']==62
    assert result['details'][1]['status']=='insufficient_evidence'
    assert notes[0]['midi']==60
    assert result['accuracy'] is None


def test_ai_review_fallback_is_not_independent_evidence(monkeypatch):
    import core.ai_review as review
    from core.config import Config
    class FakeDetector:
        def __init__(self,**kwargs):
            self.used_backend='pyin'
            self.failures={'torchcrepe':'missing'}
        def detect(self,*args): return []
    monkeypatch.setattr(review,'PitchDetector',FakeDetector)
    result=review.review_audio(np.zeros(100),16000,[],'pyin',Config())
    assert result['status']=='unavailable'
    assert 'details' not in result


def test_robust_thresholds_reuse_inference_without_stale_audio(monkeypatch):
    from core.pitch import PitchDetector
    detector=PitchDetector(backend='pyin')
    calls=[]
    monkeypatch.setattr(detector,'_is_backend_ok',lambda name: True)
    def detect(y,sr,conf_thresh=None):
        calls.append(conf_thresh)
        return [{'t':0.,'freq':440.,'conf':.4},{'t':.02,'freq':440.,'conf':.8}]
    monkeypatch.setattr(detector,'_detect_pyin',detect)
    y=np.zeros(160)
    first=detector.detect(y,16000,conf_thresh=.3)
    first[0]['freq']=99
    second=detector.detect(y,16000,conf_thresh=.7)
    assert len(second)==1 and second[0]['freq']==440
    assert len(calls)==1
    y[0]=1
    detector.detect(y,16000)
    assert len(calls)==2


def test_auto_separator_does_not_claim_hpss_is_vocals(monkeypatch):
    from core import separator
    monkeypatch.setattr(separator,'_has_demucs',lambda: False)
    y=np.linspace(-1,1,100,dtype=np.float32)
    result=separator.separate_melody(y,16000,'auto')
    assert result['strategy']=='passthrough'
    assert result['warning']
    assert np.array_equal(result['vocals'],y)


def test_separation_metadata_remains_json_serializable():
    import json
    from core.config import Config
    from core.pipeline import Melody2Score
    t=np.arange(16000)/16000
    cfg=Config(preferred_backend='pyin',robust=False,enable_separation=True,
               separation_strategy='none',vocal_mode=False,enable_vad=False,enable_denoise=False)
    result=Melody2Score(cfg).recognize({'kind':'array','sr':16000,'y':np.sin(2*np.pi*440*t).astype(np.float32)})
    assert 'other' not in result['separation']
    json.dumps(result)


def test_postprocess_preserves_legitimate_pitch_changes_and_long_notes():
    from core.postprocess import postprocess_notes
    for pitches in ([60,61,60], [60,72,60]):
        notes = [{'midi': m, 'start': i*.4, 'end': i*.4+.3, 'sep_prev': True}
                 for i,m in enumerate(pitches)]
        assert [n['midi'] for n in postprocess_notes(notes)['notes']] == pitches
    original = {'midi':60, 'start':0., 'end':8., 'dur':999.}
    result = postprocess_notes([original])['notes'][0]
    assert result['end'] == result['dur'] == 8.
    assert original['dur'] == 999.


def test_review_sparse_frames_cannot_certify_long_note():
    from core.ai_review import compare_notes
    points = [{'t':t, 'freq':261.625565, 'conf':.99} for t in (.1,.11)]
    result = compare_notes([{'midi':60,'start':0.,'end':5.}], points)
    assert result['details'][0]['status'] == 'insufficient_evidence'
    assert result['details'][0]['coverage'] < .01
    assert result['review_note_indices'] == [0]


def test_review_detects_missing_regions_without_inventing_notes():
    from core.ai_review import compare_notes
    points = [{'t':float(t), 'freq':440., 'conf':.9} for t in np.arange(.1,.5,.01)]
    result = compare_notes([], points)
    assert len(result['review_intervals']) == 1
    assert result['review_intervals'][0]['reason'] == 'possible_missing_note'
    assert compare_notes([], points[:2])['review_intervals'] == []
    covered = compare_notes([{'midi':69,'start':0.,'end':.6}], points)
    assert covered['review_intervals'] == []
    assert compare_notes([], [])['review_intervals'] == []


def test_enterprise_review_config_and_dotted_quarter():
    from enterprise_api import _build_config, _notes_to_vexflow, app
    assert _build_config(ai_review=True).ai_review
    result = _notes_to_vexflow([{'midi':60,'dur':.75}], {'tonic':'C','mode':'major'},120)
    assert result['notes'][0]['duration'] == 'qd'
    assert result['total_beats'] == 1.5
    schema = app.openapi()
    for name in ('recognize','recognize-sample','recognize-record'):
        body = schema['paths']['/api/melody2score/'+name]['post']['requestBody']
        ref = next(iter(body['content'].values()))['schema']['$ref'].split('/')[-1]
        assert 'ai_review' in schema['components']['schemas'][ref]['properties']


def test_missing_note_review_reaches_quality_warnings(monkeypatch):
    from core.config import Config
    from core.pipeline import Melody2Score
    import core.ai_review as review
    interval = {'start': .2, 'end': .4, 'reason': 'possible_missing_note'}
    monkeypatch.setattr(review, 'review_audio', lambda *args: {
        'status': 'reviewed', 'details': [], 'review_note_indices': [],
        'review_intervals': [interval], 'accuracy': None})
    cfg = Config(preferred_backend='pyin', ai_review=True, robust=False,
                 enable_separation=False, enable_denoise=False, vocal_mode=False,
                 enable_vad=False)
    y = (.3*np.sin(2*np.pi*440*np.arange(16000)/16000)).astype(np.float32)
    result = Melody2Score(cfg).recognize({'kind':'array','y':y,'sr':16000})
    assert result['quality']['status'] == 'needs_review'
    assert result['quality']['review_intervals'] == [interval]
    assert any('0.20' in w and '0.40' in w for w in result['quality']['warnings'])
