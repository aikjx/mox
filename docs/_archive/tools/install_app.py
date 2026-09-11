#!/usr/bin/env python3
"""
MOX 一键安装/卸载/更新工具
用法:
  python tools/install_app.py --app-key my-crm           # 从商店安装
  python tools/install_app.py --file ./my-crm.mxap       # 从本地包安装
  python tools/install_app.py --app-key my-crm --uninstall
  python tools/install_app.py --app-key my-crm --update
  python tools/install_app.py --list                       # 列出已安装
"""
import os, sys, json, sqlite3, zipfile, shutil, argparse, urllib.request
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
META_DB = os.path.join(ROOT, "platform", "mox-server", "mox_meta.db")
STORE_DIR = os.path.join(ROOT, "platform", "mox-store", "store_data")
INSTALLED_DIR = os.path.join(STORE_DIR, "installed")


def now_iso():
    return datetime.now(CST).isoformat()


def get_db():
    conn = sqlite3.connect(META_DB)
    conn.row_factory = sqlite3.Row
    return conn


def init_db():
    conn = get_db()
    conn.executescript("""
    CREATE TABLE IF NOT EXISTS store_installs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        app_key TEXT UNIQUE NOT NULL,
        app_name TEXT NOT NULL,
        version TEXT DEFAULT '1.0.0',
        install_path TEXT DEFAULT '',
        status TEXT DEFAULT 'running',
        config TEXT DEFAULT '{}',
        installed_at TEXT, updated_at TEXT
    );
    """)
    conn.commit()
    conn.close()


def install_from_store(app_key, store_url="http://localhost:8601"):
    init_db()
    # 从商店获取应用信息
    try:
        resp = urllib.request.urlopen(f"{store_url}/api/store/apps/{app_key}", timeout=10)
        app = json.loads(resp.read().decode())["data"]
    except Exception as e:
        print(f"ERROR: 无法从商店获取应用: {e}")
        return False

    # 下载MXAP包
    mxap_url = app.get("mxap_path", "")
    if not mxap_url or not os.path.exists(mxap_url):
        print(f"ERROR: MXAP包不存在: {mxap_url}")
        return False

    return install_from_file(mxap_url, app_key, app.get("app_name", app_key), app.get("version", "1.0.0"))


def install_from_file(file_path, app_key=None, app_name=None, version=None):
    init_db()
    if not os.path.exists(file_path):
        print(f"ERROR: 文件不存在: {file_path}")
        return False

    # 读取manifest
    with zipfile.ZipFile(file_path, 'r') as zf:
        manifest = json.loads(zf.read("manifest.json").decode())

    ak = app_key or manifest.get("app_key", "")
    an = app_name or manifest.get("app_name", ak)
    ver = version or manifest.get("version", "1.0.0")

    if not ak:
        print("ERROR: 缺少app_key")
        return False

    # 检查是否已安装
    conn = get_db()
    existing = conn.execute("SELECT id FROM store_installs WHERE app_key=?", (ak,)).fetchone()
    if existing:
        print(f"WARN: 应用 {ak} 已安装，使用 --update 更新")
        conn.close()
        return False

    # 解压
    install_path = os.path.join(INSTALLED_DIR, ak)
    os.makedirs(install_path, exist_ok=True)
    with zipfile.ZipFile(file_path, 'r') as zf:
        zf.extractall(install_path)

    ts = now_iso()
    conn.execute("""INSERT INTO store_installs (app_key, app_name, version, install_path, status, config, installed_at, updated_at)
        VALUES (?,?,?,?,?,?,?,?)""", (ak, an, ver, install_path, "running", "{}", ts, ts))
    conn.commit()
    conn.close()

    print(f"✓ 安装成功: {an} ({ak}) v{ver}")
    print(f"  路径: {install_path}")
    print(f"  入口: {install_path}/frontend/index.html")
    return True


def uninstall(app_key):
    init_db()
    conn = get_db()
    inst = conn.execute("SELECT * FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    if not inst:
        print(f"ERROR: 应用未安装: {app_key}")
        conn.close()
        return False
    inst = dict(inst)
    # 删除目录
    if inst.get("install_path") and os.path.exists(inst["install_path"]):
        shutil.rmtree(inst["install_path"], ignore_errors=True)
    conn.execute("DELETE FROM store_installs WHERE app_key=?", (app_key,))
    conn.commit()
    conn.close()
    print(f"✓ 卸载成功: {inst['app_name']} ({app_key})")
    return True


def update_app(app_key, file_path=None, store_url="http://localhost:8601"):
    init_db()
    conn = get_db()
    inst = conn.execute("SELECT * FROM store_installs WHERE app_key=?", (app_key,)).fetchone()
    if not inst:
        print(f"ERROR: 应用未安装: {app_key}")
        conn.close()
        return False
    inst = dict(inst)
    conn.close()

    # 获取新版本
    if file_path:
        mxap_path = file_path
    else:
        try:
            resp = urllib.request.urlopen(f"{store_url}/api/store/apps/{app_key}", timeout=10)
            app = json.loads(resp.read().decode())["data"]
            mxap_path = app.get("mxap_path", "")
        except Exception as e:
            print(f"ERROR: 获取应用信息失败: {e}")
            return False

    if not mxap_path or not os.path.exists(mxap_path):
        print("ERROR: MXAP包不存在")
        return False

    # 读取新版本
    with zipfile.ZipFile(mxap_path, 'r') as zf:
        manifest = json.loads(zf.read("manifest.json").decode())
    new_ver = manifest.get("version", "1.0.0")

    # 重新解压
    shutil.rmtree(inst["install_path"], ignore_errors=True)
    os.makedirs(inst["install_path"], exist_ok=True)
    with zipfile.ZipFile(mxap_path, 'r') as zf:
        zf.extractall(inst["install_path"])

    ts = now_iso()
    conn = get_db()
    conn.execute("UPDATE store_installs SET version=?, updated_at=? WHERE app_key=?", (new_ver, ts, app_key))
    conn.commit()
    conn.close()

    print(f"✓ 更新成功: {inst['app_name']} ({app_key}) {inst['version']} → {new_ver}")
    return True


def list_installed():
    init_db()
    conn = get_db()
    rows = conn.execute("SELECT * FROM store_installs ORDER BY installed_at DESC").fetchall()
    conn.close()
    if not rows:
        print("暂无已安装应用")
        return
    print(f"\n{'='*70}")
    print(f"  {'应用名称':20s} {'标识':15s} {'版本':10s} {'状态':8s} 安装时间")
    print(f"{'='*70}")
    for r in rows:
        print(f"  {r['app_name']:20s} {r['app_key']:15s} {r['version']:10s} {r['status']:8s} {r['installed_at'][:19]}")
    print(f"{'='*70}")
    print(f"  共 {len(rows)} 个应用\n")


def main():
    p = argparse.ArgumentParser(description="MOX 一键安装/卸载/更新工具")
    p.add_argument("--app-key", default=None, help="应用标识")
    p.add_argument("--file", default=None, help="本地MXAP包路径")
    p.add_argument("--store-url", default="http://localhost:8601", help="应用商店地址")
    p.add_argument("--uninstall", action="store_true", help="卸载应用")
    p.add_argument("--update", action="store_true", help="更新应用")
    p.add_argument("--list", action="store_true", help="列出已安装应用")
    args = p.parse_args()

    if args.list:
        list_installed()
        return
    if not args.app_key and not args.file:
        print("ERROR: 请指定 --app-key 或 --file")
        return

    if args.uninstall:
        uninstall(args.app_key)
    elif args.update:
        update_app(args.app_key, args.file, args.store_url)
    elif args.file:
        install_from_file(args.file)
    else:
        install_from_store(args.app_key, args.store_url)


if __name__ == "__main__":
    main()
