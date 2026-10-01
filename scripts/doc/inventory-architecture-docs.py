#!/usr/bin/env python3
"""全 docs 结构盘点：文件/摘要摘录/章节/图源/显式引用；不推断实现成熟度。"""
import argparse
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("doc_links", ROOT / "scripts/gate/check-doc-links.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
JSON_OUT = ROOT / "reports/data/docs-architecture-corpus.json"
MD_OUT = ROOT / "reports/markdown/docs-architecture-corpus.md"
TEXT_EXT = {".md", ".html", ".mmd", ".txt", ".rst", ".json", ".yaml", ".yml", ".sql", ".toml", ".csv", ".xml", ".ps1", ".sh", ".js", ".css", ".py"}
FENCE = re.compile(r"^[ \t]*(`{3,}|~{3,})mermaid[^\n]*\n(.*?)^[ \t]*\1[ \t]*$", re.M | re.S)


def inspect(path, root):
    data = path.read_bytes()
    rel = path.relative_to(root).as_posix()
    parts = path.relative_to(root / "docs").parts
    lifecycle = "archive" if "_archive" in parts else "evidence" if any(p in {"working-reports", "_verification", "_data", "_shots"} for p in parts) else "active-candidate"
    row = {"id": rel, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "lifecycle": lifecycle, "family": parts[0] if len(parts) > 1 else "(root)", "text": path.suffix.lower() in TEXT_EXT, "headings": [], "diagrams": [], "references": []}
    if not row["text"]:
        return row
    text = data.decode("utf-8-sig", errors="replace").replace("\r\n", "\n").replace("\r", "\n")
    row["decode_replacements"] = text.count("\ufffd")
    if path.suffix.lower() == ".html":
        title = re.search(r"<title[^>]*>(.*?)</title>", text, re.I | re.S)
        headings = [(m.start(), re.sub(r"<[^>]+>", "", m.group(2)).strip(), int(m.group(1))) for m in re.finditer(r"<h([1-6])\b[^>]*>(.*?)</h\1>", text, re.I | re.S)]
    else:
        title = re.search(r"^#\s+(.+)$", text, re.M)
        headings = [(m.start(), m.group(2), len(m.group(1))) for m in re.finditer(r"^(#{1,6})\s+(.+)$", gate.strip_fences(text), re.M)]
    row["title"] = title.group(1).strip() if title else path.stem
    row["headings"] = [{"line": text.count("\n", 0, pos) + 1, "title": label, "depth": depth} for pos, label, depth in headings]
    row["topics"] = [h["title"] for h in row["headings"] if h["depth"] <= 2][:16]
    # 摘录只保留原文句子，不把来源自称“完成”改写成验证结论。
    if path.suffix.lower() == ".md":
        body = re.sub(r"\A---\n.*?\n---\n", "", text, flags=re.S)
        paragraphs = re.split(r"\n\s*\n", gate.strip_fences(body))
        excerpt = next((p.strip().replace("\n", " ") for p in paragraphs if p.strip() and not p.lstrip().startswith(("#", "|", "---", "<!--", "```", "~~~"))), "")
        row["excerpt"] = excerpt[:320]
    for m in FENCE.finditer(text):
        line = text.count("\n", 0, m.start()) + 1
        row["diagrams"].append({"line": line, "section": next((h["title"] for h in reversed(row["headings"]) if h["line"] < line), row["title"]), "syntax": "mermaid", "source": m.group(2).strip()})
    if path.suffix.lower() == ".mmd":
        row["diagrams"].append({"line": 1, "section": path.stem, "syntax": "mermaid", "source": text.strip()})
    if path.suffix.lower() == ".html":
        for m in re.finditer(r'<(?:pre|div)\b[^>]*class=["\'][^"\']*\bmermaid\b[^"\']*["\'][^>]*>(.*?)</(?:pre|div)>', text, re.I | re.S):
            row["diagrams"].append({"line": text.count("\n", 0, m.start()) + 1, "section": "HTML Mermaid", "syntax": "mermaid", "source": m.group(1).strip()})
    ignored = gate.ignored_lines(text)
    for line, target, kind in gate.extract_targets(gate.strip_fences(text)):
        if line in ignored:
            continue
        row["references"].append({"line": line, "target": target, "kind": kind, "evidence_type": "EXTRACTED"})
    return row


def scan(root):
    files = sorted(p for p in (root / "docs").rglob("*") if p.is_file() and not set(p.parts) & {"node_modules", ".git", "target"})
    rows = [inspect(p, root) for p in files]
    ids = {row["id"] for row in rows}
    edges = []
    for row in rows:
        for ref in row["references"]:
            target, kind = gate.normalize_target(ref["target"])
            if target is None:
                continue
            if kind == "root-relative":
                target = root / target.relative_to(ROOT)
            elif kind == "relative":
                target = root / row["id"] / ".." / target
            target = gate.resolve(target)
            try:
                target_id = target.relative_to(root).as_posix()
            except ValueError:
                continue
            ref["resolved_id"] = target_id
            ref["exists"] = target.exists()
            if target_id in ids:
                edges.append({"source": row["id"], "target": target_id, "source_location": row["id"] + ":" + str(ref["line"]), "relation": "REFERENCES", "evidence_type": "EXTRACTED"})
    return {"schema": "docs-architecture-corpus/v1", "method": "Deterministic extraction; explicit references only; no semantic or readiness inference", "stats": {"files": len(rows), "text_files": sum(r["text"] for r in rows), "mermaid_sources": sum(len(r["diagrams"]) for r in rows), "references": sum(len(r["references"]) for r in rows), "internal_edges": len(edges), "by_lifecycle": dict(sorted(Counter(r["lifecycle"] for r in rows).items())), "by_family": dict(sorted(Counter(r["family"] for r in rows).items()))}, "documents": rows, "edges": edges}


def markdown(corpus):
    stats = corpus["stats"]
    lines = ["# docs 全量架构与流程资料盘点", "", "由 `python scripts/doc/inventory-architecture-docs.py` 生成；禁止手改。", "", "这是全量结构摘录与显式引用图，不是逐篇语义审查或生产认证。目标态、自称权威和归档文件均不自动升级为现状事实。JSON 保留文件 SHA-256、所有章节、Mermaid 图源和逐行引用，供反向检索。非文本文件仅登记元数据；HTML 中脚本动态绘图和普通文本框图未算入 Mermaid 分母。", "", f"文件 {stats['files']}；可读文本 {stats['text_files']}；Mermaid 图源 {stats['mermaid_sources']}；显式引用 {stats['references']}。", "", "| 分类 | 文件数 |", "|---|---:|"]
    lines.extend(f"| {key} | {value} |" for key, value in stats["by_family"].items())
    for family in stats["by_family"]:
        lines.extend(["", f"## {family}", "", "| 来源 | 生命周期 | 原文章节摘要 | 图源数 |", "|---|---|---|---:|"])
        for row in corpus["documents"]:
            if row["family"] != family:
                continue
            label = row.get("title", row["id"]).replace("|", "／").replace("\n", " ").replace("[", "（").replace("]", "）")
            topics = "；".join(row.get("topics", [])).replace("|", "／").replace("\n", " ")[:340] or "无 Markdown/HTML 章节；详见 JSON 元数据"
            # 报告位于 reports/markdown；此处相对链接不属于新增 docs 文档的引用规则。
            href = quote("../../" + row["id"], safe="/.-_")
            lines.append(f"| [{label}]({href}) | {row['lifecycle']} | {topics} | {len(row['diagrams'])} |")
    return "\n".join(lines) + "\n"


def selftest():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "docs/_archive").mkdir(parents=True)
        (root / "docs/a.md").write_text('# 主题\n\n原文摘要。\n\n## 流程\n```mermaid\nflowchart LR\n A-->B\n```\n\n[关联](b.md)\n\n```txt\n[示例](missing.md)\n```\n', encoding="utf-8")
        (root / "docs/_archive/b.md").write_text('# 历史\n\n[当前](../a.md)\n', encoding="utf-8-sig")
        (root / "docs/image.png").write_bytes(b"\x89PNG")
        corpus = scan(root)
        assert corpus["stats"]["files"] == 3
        assert corpus["stats"]["mermaid_sources"] == 1, corpus
        row = next(r for r in corpus["documents"] if r["id"] == "docs/a.md")
        assert row["diagrams"][0]["section"] == "流程"
        assert [r["target"] for r in row["references"]] == ["b.md"]
        assert corpus["stats"]["by_lifecycle"]["archive"] == 1
        assert row["excerpt"] == "原文摘要。"
        assert corpus["edges"][0]["target"] == "docs/a.md"
        assert row["references"][0]["exists"] is False
        assert scan(root) == corpus
        with (root / "docs/a.md").open("a", encoding="utf-8") as stream:
            stream.write("\n新增事实\n")
        assert scan(root) != corpus  # 来源变化必须使 --check 对账失败。
    print("inventory selftest PASS")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="只读核对已生成盘点是否与 docs 一致")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        selftest()
        return
    corpus = scan(ROOT)
    outputs = {JSON_OUT: json.dumps(corpus, ensure_ascii=False, indent=2) + "\n", MD_OUT: markdown(corpus)}
    if args.check:
        stale = [p.relative_to(ROOT).as_posix() for p, expected in outputs.items() if not p.exists() or p.read_text(encoding="utf-8") != expected]
        if stale:
            raise SystemExit("盘点漂移：" + ", ".join(stale))
    else:
        for path, content in outputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("w", encoding="utf-8", newline="\n") as stream:
                stream.write(content)
    print(json.dumps(corpus["stats"], ensure_ascii=False))


if __name__ == "__main__":
    main()
