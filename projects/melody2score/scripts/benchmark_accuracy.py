"""Evaluate production recognition against timestamped corpus annotations."""
import argparse
import hashlib
import json
import sys
import time
from pathlib import Path
import numpy as np
from scipy.optimize import linear_sum_assignment

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))


def reference_notes(seq, beat, gap):
    notes, start = [], 0.
    for midi, beats in seq:
        if midi > 0:
            notes.append({"midi": midi, "start": start, "end": start + beats * beat})
        start += beats * beat + gap
    return notes


def note_metrics(expected, actual, tolerance=.05):
    n, m = len(expected), len(actual)
    matches = []
    if n and m:
        costs = np.full((n, m), 1e6)
        for i, e in enumerate(expected):
            for j, a in enumerate(actual):
                delta = abs(e["start"] - a["start"])
                if e["midi"] == a["midi"] and delta <= tolerance:
                    costs[i, j] = delta
        rows, cols = linear_sum_assignment(costs)
        matches = [(i, j) for i, j in zip(rows, cols) if costs[i, j] < 1e6]
    tp = len(matches)
    precision, recall = tp / max(1, m), tp / max(1, n)
    return {"tp": tp, "expected": n, "predicted": m,
            "precision": precision, "recall": recall,
            "f1": 2 * tp / max(1, n + m),
            "onset_mae_ms": float(np.mean([abs(expected[i]["start"]-actual[j]["start"]) for i,j in matches])*1000) if matches else None,
            "offset_mae_ms": float(np.mean([abs(expected[i]["end"]-actual[j]["end"]) for i,j in matches])*1000) if matches else None}


def summarize(items):
    out = {"count": len(items), "failed": sum("error" in i for i in items)}
    for key in ("strict", "loose"):
        tp = sum(i.get(key, {}).get("tp", 0) for i in items)
        expected = sum(i.get(key, {}).get("expected", i.get("n_expected", 0)) for i in items)
        predicted = sum(i.get(key, {}).get("predicted", 0) for i in items)
        out[key] = {"precision": tp/max(1,predicted), "recall": tp/max(1,expected),
                    "f1": 2*tp/max(1,expected+predicted), "matched": tp,
                    "expected": expected, "predicted": predicted}
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", default="pyin")
    ap.add_argument("--model", default="full")
    ap.add_argument("--melodies", default="0,2")
    ap.add_argument("--timbres", default="")
    ap.add_argument("--code-root")
    ap.add_argument("--no-denoise", action="store_true")
    ap.add_argument("--output", required=True)
    args = ap.parse_args()
    if args.code_root: sys.path.insert(0, str(Path(args.code_root).resolve()))
    from core.config import Config
    from core.pipeline import Melody2Score
    from core.capture import load_audio
    import classic_corpus as corpus
    manifest = json.loads((ROOT/'audio/manifest.json').read_text(encoding='utf-8'))
    selected = set(map(int,args.melodies.split(','))) if args.melodies != 'all' else None
    timbres = set(args.timbres.split(',')) if args.timbres else None
    items = []
    output = Path(args.output); output.parent.mkdir(parents=True,exist_ok=True)
    for item in manifest:
        if selected is not None and item['melody_index'] not in selected: continue
        if timbres and item['timbre'] not in timbres: continue
        expected = reference_notes(corpus.MELODIES[item['melody_index']][2],corpus.BEAT,corpus.GAP)
        row = {'id':item['id'], 'timbre':item['timbre'], 'category':item['category'], 'n_expected':len(expected)}
        started = time.perf_counter()
        try:
            cfg = Config(preferred_backend=args.backend, model_size=args.model, robust=False,
                         vocal_mode=item['category']=='voice', enable_separation=False,
                         enable_denoise=not args.no_denoise, intra_op_threads=2)
            path = ROOT/item['file']
            row['audio_sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
            y = load_audio(str(path),cfg.sr)
            result = Melody2Score(cfg).recognize({'kind':'array','y':y,'sr':cfg.sr})
            row.update(backend=result['backend'], confidence=result['confidence'], notes=result['notes'], bpm=result['bpm'], key=result['key'])
            row['strict']=note_metrics(expected,result['notes'],.05)
            row['loose']=note_metrics(expected,result['notes'],.15)
        except Exception as exc:
            row['error']=str(exc)
        row['seconds']=round(time.perf_counter()-started,3)
        items.append(row)
        summary = summarize(items)
        by_timbre={t:summarize([r for r in items if r['timbre']==t]) for t in sorted({r['timbre'] for r in items})}
        output.write_text(json.dumps({'arguments':vars(args),'summary':summary,'by_timbre':by_timbre,'items':items},ensure_ascii=False,indent=2),encoding='utf-8')
        print(row['id'],row.get('backend'),row.get('strict',{}).get('f1'),row.get('loose',{}).get('f1'),row.get('error',''),flush=True)
    print(json.dumps(summary),flush=True)

if __name__ == '__main__': main()
