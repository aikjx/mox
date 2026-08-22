#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""旋律转谱图谱一键导出 + 可选 info-graph 规范校验。

用法:
    python graph/export_melody_graph.py                 # 生成 JSON + Mermaid + Cytoscape.html
    python graph/export_melody_graph.py --validate      # 额外用 tools/info-graph 校验 GR-STD-V1.0
    python graph/export_melody_graph.py --html-only      # 仅重导出交互式 HTML

导出采用「最好的开源工具架构」:
    - 规范 JSON      -> tools/info-graph 同构 (GR-STD-V1.0, nodes/edges schema 一致)
    - Mermaid        -> 文档/画布级静态图谱 (info-graph export --format mermaid 同构)
    - Cytoscape.js   -> 交互式网络图 (网络图领域事实标准, CDN 零依赖单 HTML)
"""
import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)

from build_melody_graph import main as _build_main  # noqa: E402
from build_melody_graph import Graph  # noqa: E402


def _validate_with_info_graph(json_path: str) -> bool:
    """用 tools/info-graph 校验产物是否符合 GR-STD-V1.0。"""
    info_graph = os.path.join(REPO_ROOT, "tools", "info-graph")
    if not os.path.isdir(info_graph):
        print("[warn] 未找到 tools/info-graph，跳过规范校验。")
        return True
    # 期望 info-graph 提供 validate 子命令；若二进制未编译则提示。
    bin_path = os.path.join(info_graph, "target", "release", "info-graph")
    if not os.path.isfile(bin_path):
        print("[warn] info-graph 未编译 (cargo build --release)，跳过规范校验。"
              " 可运行: cd tools/info-graph && cargo build --release")
        return True
    try:
        r = subprocess.run([bin_path, "validate", "--path", json_path],
                           capture_output=True, text=True)
        print(r.stdout)
        if r.returncode != 0:
            print(r.stderr)
            return False
        return True
    except Exception as e:  # pragma: no cover
        print(f"[warn] info-graph 校验调用失败: {e}")
        return True


def main():
    ap = argparse.ArgumentParser(description="旋律图谱导出 (JSON/Mermaid/Cytoscape)")
    ap.add_argument("--validate", action="store_true", help="用 tools/info-graph 校验 GR-STD-V1.0")
    ap.add_argument("--html-only", action="store_true", help="仅重导出交互式 HTML")
    args = ap.parse_args()

    if args.html_only:
        # 直接复用已生成的 JSON 重新产出 HTML
        import json
        json_path = os.path.join(HERE, "melody_infograph.json")
        if not os.path.isfile(json_path):
            print("尚未生成 melody_infograph.json，先完整构建一次。")
            _build_main()
        else:
            with open(json_path, encoding="utf-8") as f:
                data = json.load(f)
            g = Graph.__new__(Graph)
            g.nodes = {}
            g.edges = {}
            for n in data["nodes"]:
                g.nodes[n["id"]] = n
            for e in data["edges"]:
                g.edges[e["id"]] = e
            out_html = os.path.join(HERE, "melody_infograph.html")
            with open(out_html, "w", encoding="utf-8") as f:
                f.write(g.to_cytoscape_html("旋律转谱领域信息关联图谱"))
            print(f"已重导出交互式 HTML: {out_html}")
    else:
        _build_main()

    if args.validate:
        ok = _validate_with_info_graph(os.path.join(HERE, "melody_infograph.json"))
        sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
