import importlib.util, sys, glob, os
spec = importlib.util.spec_from_file_location("cvh", "scripts/gate/check-view-hex.py")
m = importlib.util.module_from_spec(spec); sys.dont_write_bytecode = True
spec.loader.exec_module(m)
scanned = m.scan_sources()
print('== App.vue 是否还在账 ==')
for r in sorted(scanned):
    if r in ('src/App.vue',) or r.endswith('App.vue'):
        print('   %-40s %d 处' % (r, len(scanned[r])))
print('   App.vue in BASELINE:', [k for k in m.BASELINE if k.endswith('App.vue')],
      {k: len(m.BASELINE[k]) for k in m.BASELINE if k.endswith('App.vue')})
print('== 被排除层 src/modules 内的裸 hex 逐处 ==')
for ext in ('vue','js'):
    for p in glob.glob(os.path.join(m.SRC,'modules','**','*.'+ext), recursive=True):
        if p.endswith(('.test.js','.spec.js')): continue
        rel = p.replace(os.sep,'/').split('frontend-ui/')[-1]
        if not rel.startswith('src/'): rel = 'src/' + rel.split('src/',1)[1] if 'src/' in rel else rel
        sites = m.hex_sites(m.read(p))
        if sites:
            print('   %-58s %d 处' % (rel, len(sites)))
            for s in sites[:8]:
                ln = s[0] if isinstance(s,(tuple,list)) else s
                print('        L%-5s %s' % (getattr(m,'line_of',lambda x:'')(ln) if callable(getattr(m,'line_of',None)) else ln, m.read(p).split(chr(10))[int(ln)-1].strip()[:78] if isinstance(ln,int) else ''))
