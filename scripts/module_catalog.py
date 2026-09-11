"""Generate a deterministic code inventory; existence does not imply production readiness."""
import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "docs/modules/CODE-CATALOG.md"


def location(path):
    parts = Path(path).parts
    if len(parts) >= 4 and parts[:2] == ("platform", "domains"):
        return parts[2], parts[3]
    if len(parts) >= 2 and parts[0] == "platform":
        return "platform", parts[1]
    return "external-layout", parts[0]


def link(path, label=None):
    return "[{}](<../../{}>)".format(label or path, path)


def collect():
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=str(ROOT), check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    meta = json.loads(result.stdout)
    members = set(meta["workspace_members"])
    packages = sorted((p for p in meta["packages"] if p["id"] in members), key=lambda p: p["name"])
    names = {p["name"] for p in packages}
    rows = []
    for package in packages:
        path = Path(package["manifest_path"]).relative_to(ROOT).as_posix()
        domain, layer = location(path)
        dependencies = {}
        for dep in package["dependencies"]:
            if dep["name"] in names:
                kind = dep.get("kind") or "runtime"
                dependencies.setdefault(kind, []).append(dep["name"])
        rows.append(dict(name=package["name"], path=path, domain=domain, layer=layer,
                         dependencies={k: sorted(set(v)) for k, v in sorted(dependencies.items())},
                         bins=sorted(t["name"] for t in package["targets"] if "bin" in t["kind"])))
    return rows


def render(rows):
    lines = ["# 全仓代码模块目录", "", "由 `python tools/module_catalog.py` 从 Cargo 元数据与目录生成；请勿手改。",
             "", "本目录记录代码归属和入口，不代表功能验收、生产可用性或部署就绪。运行时依赖包含可选依赖；开发和构建依赖分别列出。",
             "", "返回 [模块导航](README.md) · [API 注册表](../API-REGISTRY.md) · [端口注册表](../api/PORT-REGISTRY.md)",
             "", "## 后端模块", "", "当前 workspace：**{}** 个 crate。".format(len(rows)), ""]
    for domain in sorted({r["domain"] for r in rows}):
        group = [r for r in rows if r["domain"] == domain]
        lines += ["### {}（{}）".format(domain, len(group)), "", "| 模块 | 层/目录 | 可执行入口 | workspace 依赖 |", "|---|---|---|---|"]
        for row in group:
            deps = "<br>".join("{}: {}".format(k, ", ".join(v)) for k, v in row["dependencies"].items()) or "—"
            lines.append("| {} | {} | {} | {} |".format(link(row["path"], row["name"]), row["layer"], ", ".join(row["bins"]) or "—", deps))
        lines.append("")
    lines += ["## 前端入口与功能文件", "", "页面文件不等于已注册路由；实际挂载关系以路由源码为准。", ""]
    for folder, patterns in [("router", ("*.js", "*.ts")), ("views", ("*.vue",)), ("api", ("*.js", "*.ts")), ("stores", ("*.js", "*.ts")), ("composables", ("*.js", "*.ts"))]:
        base = ROOT / "frontend-ui/src" / folder
        paths = sorted({p for pattern in patterns for p in base.rglob(pattern) if p.is_file()})
        lines += ["### {}（{}）".format(folder, len(paths)), ""]
        lines += ["- " + link(p.relative_to(ROOT).as_posix()) for p in paths]
        lines.append("")
    lines += ["## projects 子目录", "", "包含产品、示例及验收产物；目录存在不表示它是独立服务。", "", "| 目录 | 顶层说明/构建入口 |", "|---|---|"]
    for folder in sorted((ROOT / "projects").iterdir()):
        if not folder.is_dir() or folder.name.startswith("."):
            continue
        markers = [p for p in sorted(folder.iterdir()) if p.is_file() and (p.name.lower().startswith("readme") or p.name in ("Cargo.toml", "package.json", "pyproject.toml", "Dockerfile"))]
        lines.append("| {} | {} |".format(link(folder.relative_to(ROOT).as_posix(), folder.name), " · ".join(link(p.relative_to(ROOT).as_posix(), p.name) for p in markers) or "无顶层入口标记，需人工确认归属"))
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail when generated inventory has drifted")
    args = parser.parse_args()
    text = render(collect())
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text(encoding="utf-8") != text:
            raise SystemExit("Module catalog is stale; run python tools/module_catalog.py")
        print("Module catalog matches workspace and frontend sources")
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(text, encoding="utf-8")
        print(OUTPUT.relative_to(ROOT))


if __name__ == "__main__":
    main()
