import importlib.util, sys
spec = importlib.util.spec_from_file_location("cvh", "scripts/gate/check-view-hex.py")
m = importlib.util.module_from_spec(spec); sys.dont_write_bytecode = True
spec.loader.exec_module(m)
scanned = m.scan_sources()
comp = [r for r in scanned if r.startswith('src/components/')]
comp_sites = sum(len(scanned[r]) for r in comp)
outside = {r: len(scanned[r]) for r in scanned
           if not r.startswith(('src/views/', 'src/constants/', 'src/components/'))}
print('TOTAL files=%d sites=%d' % (len(scanned), sum(len(v) for v in scanned.values())))
print('components files=%d sites=%d  (下限 20 / 500)' % (len(comp), comp_sites))
print('outside files=%d sites=%d  (下限 13 / 81)' % (len(outside), sum(outside.values())))
for r in sorted(outside, key=lambda k: -outside[k]):
    print('   %-52s %d' % (r, outside[r]))
only_comp = {r: len(scanned[r]) for r in comp}
for r in sorted(only_comp, key=lambda k: -only_comp[k])[:8]:
    print('   comp %-52s %d' % (r, only_comp[r]))
print('BASELINE files=%d sites=%d' % (len(m.BASELINE), sum(len(v) if hasattr(v,'__len__') else v for v in m.BASELINE.values())))
print('in BASELINE only:', sorted(set(m.BASELINE) - set(scanned)))
print('scanned not in BASELINE:', sorted(set(scanned) - set(m.BASELINE)))
