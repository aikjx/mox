#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""给"清哪几处才能让裸 hex 棘轮回绿并出示 SHRANK"定价：只读反事实，磁盘一字不写。

为什么不手抄数字：站点表与计数一律由被测闸门 check-view-hex.py 自己的 hex_sites / scan_sources /
ratchet 给，本脚本不携带第二种口径，也不携带任何色值字面量。

反事实怎么做：monkey-patch 闸门的 read()，让"待清文件"在内存里返回把每个命中字面量换成占位令牌
的副本，然后重跑 scan_sources()/ratchet()。因此本脚本永远不会改 BASELINE（那是 main() 的
--baseline 分支，这里从不调用），也不写任何文件；前后各取一次闸门文件 sha256 做"未改写"见证。

三条判据：① 反事实总量必须等于"现量 − 待清站点数"，否则 INVALID（算术不自洽）；
② 若待清文件全清空后 grown/new 仍非空 ⇒ 报"仍不清绿"，说明绿不在这些文件上；
③ 只有当某文件的新量**低于**基线才会出现 SHRANK ⇒ 回到基线（消红）与出示 SHRANK（验收）是两件事。
"""
import hashlib
import importlib.util
import io
import os
import sys

if hasattr(sys.stdout, 'reconfigure'):
    # Windows 控制台默认 GBK：定价报告的站点表与判据说明是中文，不重配置就直接 UnicodeEncodeError 中断
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')

HERE = os.path.dirname(os.path.abspath(__file__))
GATE = os.path.join(HERE, 'check-view-hex.py')
PLACEHOLDER = 'var(--PLACEHOLDER)'
sys.dont_write_bytecode = True


def load():
    spec = importlib.util.spec_from_file_location('cvh_cf', GATE)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def sha(path):
    with open(path, 'rb') as f:
        return hashlib.sha256(f.read()).hexdigest()[:16]


def main():
    if not os.path.exists(GATE):
        print('CLEARANCE: INVALID 找不到被测闸门 %s' % GATE)
        return 2
    before = sha(GATE)
    m = load()

    hits = m.scan_sources()
    grown, brand_new, gone = m.ratchet(hits, m.BASELINE)
    now_total = sum(len(v) for v in hits.values())
    targets = sorted([rel for rel, _n, _b in grown] + [rel for rel, _n in brand_new])
    if len(sys.argv) > 1 and sys.argv[1] == '--all-red':
        targets = sorted(set(targets))
    picked = None
    for a in sys.argv[1:]:
        if a.startswith('--files='):
            picked = [r.strip() for r in a.split('=', 1)[1].split(',') if r.strip()]
    if picked is not None:
        targets = picked
    if not targets:
        print('现量 %d 处 / %d 文件；grown=0 new=0 ⇒ 此刻没有待清文件（要指定用 --files=rel1,rel2）'
              % (now_total, len(hits)))
        return 0

    print('现量 %d 处 / %d 个文件，基线 %d 处 / %d 个文件'
          % (now_total, len(hits), sum(m.BASELINE.values()), len(m.BASELINE)))
    print('待清候选 %d 个文件：%s' % (len(targets), ' '.join(targets)))

    site_map = {}
    for rel in targets:
        text = m.read(rel.replace('/', os.sep))
        sites = m.hex_sites(text)
        in_cmt = len(sites) - len(m.hex_sites(m.strip_comments(text)))
        site_map[rel] = sites
        lits = sorted({h[1] for h in sites})
        print('  %-52s %2d 处 / %2d 个不同字面量（其中 %d 处在注释里 ⇒ 清它只改账不减债）'
              % (rel, len(sites), len(lits), in_cmt))
        print('      行号：%s' % ' '.join('%s:%s' % (h[0], h[1]) for h in sites))
    planned = sum(len(v) for v in site_map.values())

    real_read = m.read

    def patched(path):
        rel = path.replace(os.sep, '/')
        for tgt, sites in site_map.items():
            if rel.endswith(tgt):
                t = real_read(path)
                for lit in sorted({h[1] for h in sites}, key=len, reverse=True):
                    t = t.replace(lit, PLACEHOLDER)
                return t
        return real_read(path)

    m.read = patched
    buf = io.StringIO()
    real_out = sys.stdout
    sys.stdout = buf
    try:
        cf_hits = m.scan_sources()
    finally:
        sys.stdout = real_out
        m.read = real_read

    cf_total = sum(len(v) for v in cf_hits.values())
    g2, n2, go2 = m.ratchet(cf_hits, m.BASELINE)
    ok_math = (cf_total == now_total - planned)
    print('反事实：现量 %d − 待清 %d = 应得 %d，实得 %d ⇒ %s'
          % (now_total, planned, now_total - planned, cf_total,
             '算术自洽' if ok_math else 'INVALID（两把口径不同源）'))
    print('反事实 grown %s / new %s / gone %s' % (g2, n2, go2))
    if g2 or n2:
        print('  仍不清绿：绿不在这些文件上（或有 grown/new 来自别处）')
    shrunk = [r for r, b, x in go2 if r in site_map]
    print('  其中由本次清账产生的 SHRANK：%s' % (shrunk or '无 ⇒ 只回到基线不会报 SHRANK，验收那半句要清到基线以下'))

    kept = {k: v for k, v in m.BASELINE.items() if k in cf_hits}
    g3, n3, go3 = m.ratchet(cf_hits, kept)
    print('删掉零头条目后（纪律：删行，不写 0）：grown %s / new %s / gone %s / 台账 phantom %s / 未记账 %s'
          % (g3, n3, go3,
             sorted(set(kept) - set(cf_hits)),
             sorted(set(cf_hits) - set(kept))))

    after = sha(GATE)
    print('闸门文件 sha256 前/后 = %s / %s %s' % (before, after, '（未改写）' if before == after else '（<== 被改写了！）'))
    if not ok_math or before != after:
        print('CLEARANCE: INVALID')
        return 1
    print('CLEARANCE: 定价完成（只读，未写任何文件）')
    return 0


if __name__ == '__main__':
    sys.exit(main())
