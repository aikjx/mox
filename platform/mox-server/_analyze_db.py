import sqlite3, json
conn = sqlite3.connect('data/mox_data.db')
cur = conn.cursor()
cur.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
tables = [r[0] for r in cur.fetchall()]
print('=== 所有表 (%d) ===' % len(tables))
for t in tables:
    cur.execute('SELECT COUNT(*) FROM "%s"' % t)
    cnt = cur.fetchone()[0]
    cur.execute('PRAGMA table_info("%s")' % t)
    cols = [r[1]+'('+r[2]+')' for r in cur.fetchall()]
    print('  %-25s %4d行  字段: %s' % (t, cnt, ', '.join(cols)))
conn.close()
