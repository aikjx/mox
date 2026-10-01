import os, re, hashlib, shutil

FE = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui'
PRE = r'D:\tmp\upload-row-pre'
os.chdir(FE)
assert not os.path.exists(PRE), 'refusing to overwrite an earlier pre-image'
os.makedirs(PRE)

COLL = 'src/views/workspace/panels/CollaborationPanel.vue'
FILE = 'src/views/workspace/panels/FilePanel.vue'
KERNS = 'src/modules/_kernel/upload-row.js'
IMP = b"import { makeUploadRow } from '@/modules/_kernel/upload-row'"


def body(txt, nm):
    i = txt.index('function ' + nm)
    j = txt.index('\n}', i) + 2
    return txt[i:j]


def load(p):
    b = open(p, 'rb').read()
    crlf = b.count(b'\r\n')
    lone = b.count(b'\n') - crlf
    assert not (crlf and lone), p + ': mixed EOL'
    assert not crlf, p + ': CRLF -> driver only slices LF'
    return b, ('\r\n' if crlf else '\n')


bc, ecl = load(COLL)
# 驱动按 LF 行切片；CRLF 要另走字节级通道
bf, efl = load(FILE)
tc = bc.decode('utf-8')
tf = bf.decode('utf-8')

# --- 0) 两件面板里三个函数体逐字符相同（这是"副本"的判据，不成立就不许合并） ---
for nm in ['getFileType', 'formatFileSize']:
    assert body(tc, nm) == body(tf, nm), nm + ': bodies differ -> not a copy'
    assert tc.count('function ' + nm) == 1 and tf.count('function ' + nm) == 1, nm + ': not exactly 1 def'

ROW_RE = re.compile(
    r"  const type = getFileType\(file\.name\)\n  const newFile = \{\n(?:.*\n)*?  \}\n")
rc_list = ROW_RE.findall(tc)
rf_list = ROW_RE.findall(tf)
assert len(rc_list) == 1 and len(rf_list) == 1, 'row block anchor not unique'
assert rc_list[0] == rf_list[0], 'row blocks differ -> not a copy'
ROW = rc_list[0]

OBJ_START = ROW.index('{')
obj = ROW[OBJ_START + 1:ROW.rindex('}')].rstrip('\n')
assert 'type: type,' in obj, 'no type: type line'
obj = obj.replace('type: type,', 'type: getFileType(file.name),')
for probe in ['id:', 'name: file.name', 'size: formatFileSize(file.size)', 'uploader:', 'time:']:
    assert probe in obj, probe

kernel = (
    "// 上传后的乐观占位行 —— 单源。\n"
    "// 本轮之前这段在 views/workspace/panels 的两个面板里逐字符各有一份：\n"
    "// getFileType 574 B x2、formatFileSize 210 B x2、占位行块 328 B x2（三处函数体 sha 现场比对相同才动手）。\n"
    "// 键与文案逐字照搬原副本，本文件不改进任何行为；'我'/'刚刚' 是乐观本地行的占位口径，\n"
    "// 真实归属与时间要等后端回读后由列表刷新覆盖（这条边界见 FRONTEND-MODULE-GOVERNANCE §5.28）。\n"
    + body(tf, 'getFileType').replace('function', 'export function', 1) + "\n\n"
    + body(tf, 'formatFileSize').replace('function', 'export function', 1) + "\n\n"
    + "/** 文件对象 -> 列表行（与合并前两份副本的输出逐字段一致） */\n"
    + "export function makeUploadRow(file) {\n  return {" + obj + "\n  }\n}\n"
)

NEWCALL = "  const newFile = makeUploadRow(file)\n"
for p, txt in [(COLL, tc), (FILE, tf)]:
    head = txt[:txt.index('\nimport ') + 1]
    assert IMP.decode() not in txt, p + ': import already present'

rows = []


def commit():
    for p, eol, txt in [(COLL, ecl, tc), (FILE, efl, tf)]:
        b = txt.encode('utf-8')
        shutil.copyfile(p, os.path.join(PRE, os.path.basename(p) + '.before'))
        before = open(p, 'rb').read()
        out = b.replace(ROW.encode('utf-8'), NEWCALL.encode('utf-8'), 1)
        for nm in ['getFileType', 'formatFileSize']:
            out = out.replace(body(txt, nm).encode('utf-8'), b'', 1)
        idx = out.index(b'\nimport ') + 1
        out = out[:idx] + IMP + b'\n' + out[idx:]
        open(p, 'wb').write(out)
        chk = open(p, 'rb').read()
        assert chk.count(b'function getFileType') == 0 and chk.count(b'function formatFileSize') == 0
        assert chk.count(IMP) == 1 and chk.count(b'const newFile = makeUploadRow(file)') == 1
        assert b'getFileType(' not in chk and b'formatFileSize(' not in chk, p+': stale helper call left'
        assert b'\r\n' in chk if eol == '\r\n' else b'\r\n' not in chk
        rows.append((p, len(before), len(chk), hashlib.sha256(before).hexdigest()[:12],
                     hashlib.sha256(chk).hexdigest()[:12]))
    kb = kernel.encode('utf-8')
    open(KERNS, 'wb').write(kb)
    assert open(KERNS, 'rb').read() == kb
    rows.append((KERNS, 0, len(kb), '-', hashlib.sha256(kb).hexdigest()[:12]))
    with open(os.path.join(PRE, 'manifest.txt'), 'w', encoding='utf-8', newline='\n') as f:
        for r in rows:
            f.write('%s\t%d\t%d\t%s\t%s\n' % r)


# --- 1) 校验全部通过后才写盘（两趟） ---
print('副本判据成立：三处函数体两文件逐字符相同')
print('row block %d B, obj lines=%d' % (len(ROW), obj.count('\n') + 1))
print('kernel %d B' % len(kernel))
commit()
for p, a, b_, s1, s2 in rows:
    print('wrote %-58s %6d->%6d B  %s->%s' % (p, a, b_, s1, s2))
print('rc=0')
