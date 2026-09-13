"""Independent acoustic review; disagreements are surfaced, never auto-rewritten."""
from collections import defaultdict
import numpy as np
from core.pitch import PitchDetector


def compare_notes(notes, peer_points, hop_seconds=None):
    # A frame supports at most one frame interval, never an arbitrary silent gap.
    points = sorted({float(p['t']): p for p in peer_points
                     if np.isfinite(p['t'])}.values(), key=lambda p: p['t'])
    diffs = np.diff([p['t'] for p in points])
    hop = float(hop_seconds) if hop_seconds else (
        min(.02, float(np.median(diffs[diffs > 0]))) if np.any(diffs > 0) else .01)
    hop = max(.001, min(.1, hop))
    voiced = [p for p in points if np.isfinite(p['freq']) and p['freq'] > 0
              and np.isfinite(p.get('conf', 0)) and p.get('conf', 0) >= .3]
    details = []
    for index, note in enumerate(notes):
        margin = min(.02, max(0., note['end']-note['start'])/4)
        lo, hi = note['start']+margin, note['end']-margin
        support = [p for p in voiced if lo <= p['t'] <= hi]
        votes = defaultdict(float)
        covered = 0.
        cursor = lo
        for point in support:
            midi = int(round(69+12*np.log2(point['freq']/440)))
            left, right = max(lo, point['t']-hop/2), min(hi, point['t']+hop/2)
            weight = max(0., right-max(left, cursor))
            cursor = max(cursor, right)
            covered += weight
            votes[midi] += point['conf']*weight
        total = sum(votes.values())
        peer = max(votes, key=votes.get) if total else None
        dominance = votes[peer]/total if total else 0.
        coverage = min(1., covered/max(hi-lo, 1e-9))
        if len(support) < 2 or coverage < .7:
            status = 'insufficient_evidence'
        elif dominance < .7:
            status = 'ambiguous'
        else:
            status = 'agree' if peer == note['midi'] else 'disagree'
        details.append({'index': index, 'primary_midi': note['midi'], 'peer_midi': peer,
                        'status': status, 'dominance': round(dominance, 3),
                        'coverage': round(coverage, 3)})
    # Review unrepresented time spans too, including an empty primary result.
    uncovered = [p for p in voiced if not any(
        n['start']-hop <= p['t'] <= n['end']+hop for n in notes)]
    groups = []
    for point in uncovered:
        if not groups or point['t']-groups[-1][-1]['t'] > hop*1.5:
            groups.append([])
        groups[-1].append(point)
    intervals = [{'start': round(max(0., g[0]['t']-hop/2), 4),
                  'end': round(g[-1]['t']+hop/2, 4),
                  'reason': 'possible_missing_note'} for g in groups
                 if len(g) >= 3 and g[-1]['t']-g[0]['t']+hop >= .08]
    return {'status': 'reviewed', 'accuracy': None, 'details': details,
            'review_intervals': intervals,
            'review_note_indices': [r['index'] for r in details if r['status'] != 'agree']}


def review_audio(y,sr,notes,primary_backend,cfg):
    peer='pyin' if primary_backend in ('torchcrepe','crepe_onnx') else 'torchcrepe'
    detector=PitchDetector(backend=peer,model_size='tiny',hop=cfg.hop,
                           fmin=cfg.fmin,fmax=cfg.fmax,conf_thresh=cfg.conf_thresh,
                           intra_op_threads=2,inference_timeout=cfg.inference_timeout)
    try:
        points=detector.detect(y,sr)
        if detector.used_backend!=peer:
            return {'status':'unavailable','requested_backend':peer,
                    'actual_backend':detector.used_backend,'errors':detector.failures,
                    'accuracy':None}
        result=compare_notes(notes, points, hop_seconds=max(10, int(cfg.hop*2))/1000. if peer == "pyin" else cfg.hop/1000.)
        result['backend']=peer
        result['model']='tiny' if peer=='torchcrepe' else None
        return result
    except Exception as exc:
        return {'status':'unavailable','requested_backend':peer,'error':str(exc),'accuracy':None}
