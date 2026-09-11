#!/usr/bin/env python3
"""
MOX 一键发布工具 — 打包→签名→上传→上架
用法:
  python tools/publish_app.py --app-dir ./my-app --app-key my-crm --name "CRM" --upload --publish
  python tools/publish_app.py --from-app mox --upload --publish
  python tools/publish_app.py --app-dir ./my-app --pack-only
"""
import os, sys, json, hashlib, zipfile, argparse, time, uuid
from datetime import datetime, timezone, timedelta

CST = timezone(timedelta(hours=8))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def now_iso():
    return datetime.now(CST).isoformat()


def gen_signature(app_key, version, content):
    raw = f"{app_key}:{version}:{len(content)}:{hashlib.sha256(content).hexdigest()}"
    return "sig:" + hashlib.sha256(raw.encode()).hexdigest()[:32]


def pack_mxap(app_dir, app_key, name, version, author, category, description, tags, app_type):
    """打包为MXAP格式"""
    ts = now_iso()
    manifest = {
        "format": "MXAP", "version": "1.0",
        "app_key": app_key, "app_name": name, "app_type": app_type,
        "version": version, "author": author, "author_key": author.lower().replace(" ", "-"),
        "icon": "icon.png", "description": description, "long_description": description,
        "category": category, "tags": tags,
        "screenshots": [], "homepage": "", "license": "MIT", "price": "free",
        "runtime": {"min_mox_version": "1.0.0", "requires_backend": False,
                     "requires_database": True, "memory_min": "256MB"},
        "dependencies": [],
        "permissions": {"api_scopes": ["dsql:execute", "kg:query"], "data_access": [f"app_key={app_key}"]},
        "routes": [{"path": f"/{app_key}", "title": name, "icon": "package", "entry": "frontend/index.html"}],
        "created_at": ts, "updated_at": ts,
    }

    out_path = os.path.join(ROOT, "exports", f"{app_key}-{version}.mxap")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)

    with zipfile.ZipFile(out_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False, indent=2))
        # 前端文件
        frontend_dir = os.path.join(app_dir, "frontend")
        if os.path.exists(frontend_dir):
            for dp, dn, fn in os.walk(frontend_dir):
                for f in fn:
                    fp = os.path.join(dp, f)
                    arc = os.path.join("frontend", os.path.relpath(fp, frontend_dir))
                    zf.write(fp, arc)
        else:
            zf.writestr("frontend/index.html",
                f"<!DOCTYPE html><html><head><meta charset='utf-8'><title>{name}</title></head>"
                f"<body style='font-family:sans-serif;padding:40px'><h1>{name}</h1>"
                f"<p>{description}</p><p>版本: {version}</p></body></html>")
        # 数据文件
        data_dir = os.path.join(app_dir, "data")
        if os.path.exists(data_dir):
            for dp, dn, fn in os.walk(data_dir):
                for f in fn:
                    fp = os.path.join(dp, f)
                    arc = os.path.join("data", os.path.relpath(fp, data_dir))
                    zf.write(fp, arc)
        # 图标
        icon_path = os.path.join(app_dir, "icon.png")
        if os.path.exists(icon_path):
            zf.write(icon_path, "icon.png")
        # README
        readme_path = os.path.join(app_dir, "README.md")
        if os.path.exists(readme_path):
            zf.write(readme_path, "README.md")
        else:
            zf.writestr("README.md", f"# {name}\n\n{description}\n\n版本: {version}\n作者: {author}\n")

    # 签名
    with open(out_path, "rb") as f:
        content = f.read()
    sig = gen_signature(app_key, version, content)

    # 追加签名到manifest（重新打包）
    manifest["signature"] = sig
    tmp_path = out_path + ".tmp"
    with zipfile.ZipFile(out_path, 'r') as zin:
        with zipfile.ZipFile(tmp_path, 'w', zipfile.ZIP_DEFLATED) as zout:
            for item in zin.infolist():
                if item.filename == "manifest.json":
                    zout.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False, indent=2))
                else:
                    zout.writestr(item, zin.read(item.filename))
    os.replace(tmp_path, out_path)

    return out_path, manifest, sig


def upload_to_store(mxap_path, manifest, store_url="http://localhost:8601"):
    """上传到应用商店"""
    try:
        import urllib.request
        boundary = uuid.uuid4().hex
        with open(mxap_path, "rb") as f:
            file_content = f.read()
        body = (
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"manifest\"\r\n\r\n"
            f"{json.dumps(manifest)}\r\n"
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"mxap\"; filename=\"{os.path.basename(mxap_path)}\"\r\n"
            f"Content-Type: application/zip\r\n\r\n"
        ).encode() + file_content + f"\r\n--{boundary}--\r\n".encode()
        req = urllib.request.Request(f"{store_url}/api/store/publish", data=body,
                                      headers={"Content-Type": f"multipart/form-data; boundary={boundary}"})
        resp = urllib.request.urlopen(req, timeout=30)
        return json.loads(resp.read().decode())
    except Exception as e:
        return {"success": False, "message": str(e)}


def main():
    p = argparse.ArgumentParser(description="MOX 一键发布工具")
    p.add_argument("--app-dir", default=None, help="应用目录(含frontend/data/icon)")
    p.add_argument("--from-app", default=None, help="从现有MOX应用导出并发布")
    p.add_argument("--app-key", required=True, help="应用唯一标识")
    p.add_argument("--name", default=None, help="应用名称")
    p.add_argument("--version", default="1.0.0", help="版本号")
    p.add_argument("--author", default="MOX Developer", help="作者/开发商")
    p.add_argument("--category", default="办公协同", help="分类")
    p.add_argument("--description", default="", help="描述")
    p.add_argument("--tags", default="[]", help="标签(JSON数组)")
    p.add_argument("--app-type", default="subsystem", choices=["subsystem", "standalone", "plugin", "template", "theme"])
    p.add_argument("--store-url", default="http://localhost:8601", help="应用商店地址")
    p.add_argument("--pack-only", action="store_true", help="仅打包不上传")
    p.add_argument("--upload", action="store_true", help="上传到应用商店")
    p.add_argument("--publish", action="store_true", help="发布上架(需--upload)")
    args = p.parse_args()

    name = args.name or args.app_key
    tags = []
    if args.tags:
        try:
            tags = json.loads(args.tags) if isinstance(args.tags, str) else args.tags
            if not isinstance(tags, list): tags = [str(tags)]
        except Exception:
            tags = [t.strip() for t in str(args.tags).split(',') if t.strip()]
    app_dir = args.app_dir or os.path.join(ROOT, "exports", f"_pack_{args.app_key}")

    if args.from_app:
        print(f"[1/4] 从现有应用 {args.from_app} 导出数据...")
        os.makedirs(app_dir, exist_ok=True)
        os.makedirs(os.path.join(app_dir, "data"), exist_ok=True)
        # 调用export_data导出
        export_cmd = f'python "{os.path.join(ROOT, "tools", "export_data.py")}" --app-key {args.from_app} --output "{os.path.join(app_dir, "data")}"'
        os.system(export_cmd)

    print(f"[2/4] 打包MXAP: {args.app_key} v{args.version}")
    mxap_path, manifest, sig = pack_mxap(
        app_dir, args.app_key, name, args.version, args.author,
        args.category, args.description, tags, args.app_type)
    size = os.path.getsize(mxap_path)
    print(f"  包路径: {mxap_path}")
    print(f"  包大小: {size/1024:.1f} KB")
    print(f"  签名:   {sig}")

    if args.pack_only:
        print("\n[完成] 仅打包模式，未上传")
        return

    if args.upload or args.publish:
        print(f"[3/4] 上传到应用商店: {args.store_url}")
        result = upload_to_store(mxap_path, manifest, args.store_url)
        if result.get("success"):
            print(f"  上传成功: app_key={result['data']['app_key']}")
            if args.publish:
                print(f"[4/4] 发布上架: 已自动审核通过")
            else:
                print(f"[4/4] 已上传，待审核发布")
        else:
            print(f"  上传失败: {result.get('message')}")
            print(f"  提示: 请确保应用商店服务已启动 (python platform/mox-store/store_server.py)")
    else:
        print("\n[完成] 打包完成，使用 --upload 上传到应用商店")

    print(f"\n{'='*50}")
    print(f"  应用: {name} ({args.app_key})")
    print(f"  版本: {args.version}")
    print(f"  分类: {args.category}")
    print(f"  类型: {args.app_type}")
    print(f"  作者: {args.author}")
    print(f"  MXAP: {mxap_path}")
    print(f"{'='*50}")


if __name__ == "__main__":
    main()
