# -*- coding: utf-8 -*-
"""构建「哼唱旋律转歌谱」领域信息关联关系子图。

严格遵循关图规范 GR-STD-V1.0 节点/边 schema（与 tools/info-graph 同构），
可直接被 `info-graph validate / export / query` 加载，并可与 graph.enterprise.json 合并。

节点：
  - Requirement:D13                        哼唱旋律转歌谱应用（需求根）
  - Business:application/...              应用能力域
  - Business:category/<乐器|人声|纯音乐>   三大内容分类
  - Business:timbre/<音色>                 各音色（钢琴/吉他/弦乐/.../人声/纯音）
  - CodeFile:melody2score/...              core 各层 + demo 入口
  - Script:melody2score/board_run.sh      开发板运行脚本
  - Data:melody/<id>                       每条被识别的旋律样本（含 ground truth）
  - Data:melody_result/<id>                识别结果（含音高类准确率等）
  - Dependency:<pitch 后端>                实际使用的音高检测库

边：
  - Bind          D13 -> 各代码/脚本节点
  - Reference     分类/音色 与 旋律样本、模块间依赖
  - ReadWrite     流水线读取旋律音频、写出识别结果
  - Dependency     音高检测层依赖实际后端库

数据源：audio/manifest.json（合成样本元数据） + results/classic_results.json（真实识别结果）。
"""
import json
import os
import sys
from typing import Dict, List

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)  # melody2score/

# ---------------- 低层图容器（与 tools/info-graph JSON 同构） ----------------
class Graph:
    def __init__(self):
        self.nodes = {}   # id -> dict
        self.edges = {}   # "from|kind|to" -> dict

    def node(self, kind, key, name, path, summary, external=False):
        nid = f"{kind}:{key}"
        if nid not in self.nodes:
            self.nodes[nid] = {
                "id": nid, "kind": kind, "name": name,
                "path": path, "summary": summary, "external": external,
            }
        else:
            # 补全信息
            self.nodes[nid].update({"name": name, "path": path, "summary": summary})
        return nid

    def edge(self, frm, to, kind, label, evidence, external=False):
        eid = f"{frm}|{kind}|{to}"
        if eid not in self.edges:
            self.edges[eid] = {
                "id": eid, "from": frm, "to": to, "kind": kind,
                "label": label, "evidence": evidence, "external": external,
            }
        return eid

    def to_json(self):
        out = {"nodes": [], "edges": []}
        for n in sorted(self.nodes.values(), key=lambda x: x["id"]):
            out["nodes"].append({
                "id": n["id"], "kind": n["kind"], "name": n["name"],
                "path": n["path"], "summary": n["summary"],
                "external": "true" if n["external"] else "false",
            })
        for e in sorted(self.edges.values(), key=lambda x: x["id"]):
            out["edges"].append({
                "id": e["id"], "from": e["from"], "to": e["to"], "kind": e["kind"],
                "label": e["label"], "evidence": e["evidence"],
                "external": "true" if e["external"] else "false",
            })
        return out

    # ------------------------------------------------------------------
    # 导出层：复用 GR-STD-V1.0 规范（与 tools/info-graph 同构），
    # 用最好的开源工具架构产出「可看 / 可交互」的图谱：
    #   - Mermaid   : 文档/画布级静态图谱（info-graph export 同构，可被 CI 校验）
    #   - Cytoscape.js : 交互式网络图（网络图领域事实标准，零后端依赖单 HTML）
    # ------------------------------------------------------------------
    # 节点类型 -> 配色（与 tools/info-graph 的 InfoKind::color 保持一致）
    KIND_COLORS = {
        "Requirement": "#ffd54f",
        "Business": "#ffe0b2",
        "Data": "#b2dfdb",
        "CodeFile": "#cfd8dc",
        "Script": "#dcedc8",
        "Dependency": "#bbdefb",
        "Function": "#c5cae9",
        "Interface": "#f8bbd0",
        "ScheduleTask": "#fff9c4",
        "Config": "#e1bee7",
        "ThirdParty": "#ffccbc",
        "Doc": "#d7ccc8",
        "Runtime": "#b2ebf2",
    }

    @staticmethod
    def _mermaid_id(nid: str) -> str:
        """Mermaid 节点 id 必须是不含特殊字符的合法标识符。"""
        import re
        s = re.sub(r"[^0-9A-Za-z_]", "_", nid)
        return f"n_{s}" if not s[:1].isalpha() and s[:1] != "_" else s

    def to_mermaid(self) -> str:
        """导出 Mermaid（graph LR），与 info-graph export --format mermaid 同构。"""
        lines = ["graph LR"]
        # classDef（按类型上色）
        for kind, color in self.KIND_COLORS.items():
            lines.append(f"    classDef {kind} fill:{color},stroke:#555,stroke-width:1px;")
        # 节点
        id_map = {}
        for nid, n in self.nodes.items():
            mid = self._mermaid_id(nid)
            id_map[nid] = mid
            label = n["name"].replace('"', "'")
            lines.append(f'    {mid}["{label}"]:::{n["kind"]}')
        # 边
        for e in self.edges.values():
            f = id_map.get(e["from"], self._mermaid_id(e["from"]))
            t = id_map.get(e["to"], self._mermaid_id(e["to"]))
            lines.append(f'    {f} -->|{e["kind"]}| {t}')
        return "\n".join(lines) + "\n"

    def to_cytoscape_html(self, title: str = "旋律转谱领域信息关联图谱") -> str:
        """导出 Cytoscape.js 交互式网络图（CDN 引入，零后端依赖单 HTML）。"""
        import json as _json
        elements = []
        for n in self.nodes.values():
            elements.append({
                "data": {
                    "id": n["id"],
                    "label": n["name"],
                    "kind": n["kind"],
                    "path": n["path"],
                    "summary": n["summary"],
                    "external": n["external"],
                }
            })
        for e in self.edges.values():
            elements.append({
                "data": {
                    "id": e["id"],
                    "source": e["from"],
                    "target": e["to"],
                    "kind": e["kind"],
                    "label": e["label"],
                    "evidence": e["evidence"],
                }
            })
        color_map = {k: v for k, v in self.KIND_COLORS.items()}
        payload = _json.dumps(
            {"elements": elements, "colors": color_map, "title": title},
            ensure_ascii=False,
        )
        return _CYTOSCAPE_HTML_TEMPLATE.replace("__PAYLOAD__", payload).replace("__TITLE__", title)


# ---------------- 扫描 melody2score 代码文件 ----------------
def scan_code(g: Graph):
    code_files = []
    for dp, _, files in os.walk(ROOT):
        if "graph" in dp.split(os.sep) and os.path.basename(dp) == "graph":
            continue  # 子图产物本身不计入
        for fn in files:
            full = os.path.join(dp, fn)
            rel = "melody2score/" + os.path.relpath(full, ROOT).replace("\\", "/")
            if fn.endswith(".py") and fn != "build_melody_graph.py":
                code_files.append(rel)
                g.node("CodeFile", rel, fn, rel, "旋律转谱应用代码模块", False)
            elif fn.endswith(".sh"):
                g.node("Script", rel, fn, rel, "开发板/运行脚本", False)
    # 模块间依赖（core 层内部 Reference 边，evidence 取自 import）
    deps = [
        ("melody2score/melody2score_demo.py", "melody2score/core/pipeline.py", "import core"),
        ("melody2score/core/pipeline.py", "melody2score/core/capture.py", "import capture"),
        ("melody2score/core/pipeline.py", "melody2score/core/preprocess.py", "import preprocess"),
        ("melody2score/core/pipeline.py", "melody2score/core/pitch.py", "import pitch"),
        ("melody2score/core/pipeline.py", "melody2score/core/analysis.py", "import analysis"),
        ("melody2score/core/pipeline.py", "melody2score/core/score.py", "import score"),
        ("melody2score/core/pitch.py", "melody2score/core/config.py", "import config"),
        ("melody2score/core/analysis.py", "melody2score/core/config.py", "import config"),
    ]
    for a, b, ev in deps:
        g.edge(f"CodeFile:{a}", f"CodeFile:{b}", "Reference", "module-dependency",
               f"{a}: import {b.split('/')[-1].replace('.py','')}")
    return code_files


# ---------------- 主构建 ----------------
def main():
    manifest_path = os.path.join(ROOT, "audio", "manifest.json")
    results_path = os.path.join(ROOT, "results", "classic_results.json")
    g = Graph()

    # 1) 需求根 + 应用能力域
    g.node("Requirement", "D13", "哼唱旋律转歌谱应用",
           "melody2score", "需求根节点 | 域=melody2score | 状态=done", False)
    g.node("Business", "application/旋律转谱", "旋律转谱应用", "melody2score",
           "哼唱/演奏音频 -> 音高检测 -> 音符解析 -> 简谱/musicxml 的端到端应用", False)

    # 2) 扫描代码并 Bind 到 D13
    code_files = scan_code(g)
    for rel in code_files:
        g.edge("Requirement:D13", f"CodeFile:{rel}", "Bind", "需求-代码绑定",
               f"guantu-skeleton: Requirement:D13 -> {rel}")

    # 3) 内容分类 / 音色 业务节点
    cats = {"instrument": "乐器类", "voice": "人声类", "pure": "纯音乐类"}
    for cat, zh in cats.items():
        g.node("Business", f"category/{cat}", zh, cat, f"旋律内容分类：{zh}", False)
        g.edge("Business:application/旋律转谱", f"Business:category/{cat}",
               "Reference", "包含分类", f"应用覆盖分类 {cat}")

    # 4) 读取 manifest + results
    manifest = []
    if os.path.exists(manifest_path):
        with open(manifest_path, encoding="utf-8") as f:
            manifest = json.load(f)
    results = {}
    if os.path.exists(results_path):
        with open(results_path, encoding="utf-8") as f:
            rd = json.load(f)
        for it in rd.get("items", []):
            results[it["id"]] = it
    backend = results and rd.get("summary", {}).get("backend") or "unknown"

    used_timbres = set()
    for item in manifest:
        mid = item["id"]
        cat = item["category"]
        timbre = item["timbre"]
        used_timbres.add(timbre)
        # 音色业务节点
        g.node("Business", f"timbre/{timbre}", timbre, timbre,
               f"音色：{timbre}（用于合成旋律样本）", False)
        # 旋律样本 Data 节点
        res = results.get(mid, {})
        summary_bits = [f"标题={item['title_zh']}/{item['title_en']}",
                        f"分类={cat}", f"音色={timbre}",
                        f"期望音符数={len(item['expected_midi'])}"]
        if res:
            summary_bits.append(f"识别音高类准确率={res.get('pitch_class_acc',0)*100:.1f}%")
            summary_bits.append(f"音符召回={res.get('note_recall',0)*100:.1f}%")
            summary_bits.append(f"后端={backend}")
        did = f"Data:melody/{mid}"
        g.node("Data", f"melody/{mid}", item["title_zh"], item["file"],
               " | ".join(summary_bits), False)
        # 关联：样本 -> 分类 / 音色 / 应用
        g.edge(did, f"Business:category/{cat}", "Reference", "属于分类",
               f"{mid} 渲染为 {cat} 样本")
        g.edge(did, f"Business:timbre/{timbre}", "Reference", "使用音色",
               f"{mid} 使用 {timbre} 音色合成")
        # 流水线读取该旋律音频
        g.edge("CodeFile:melody2score/core/pipeline.py", did, "ReadWrite",
               "读取旋律音频", f"pipeline.run({item['file']})")

        # 识别结果 Data 节点
        if res:
            rid = f"Data:melody_result/{mid}"
            g.node("Data", f"melody_result/{mid}", f"{item['title_zh']}识别结果",
                   f"results/classic_results.json#{mid}",
                   f"后端={backend} | 音高类准确率={res.get('pitch_class_acc',0)*100:.1f}% "
                   f"| 音符召回={res.get('note_recall',0)*100:.1f}% "
                   f"| 识别音符数={res.get('n_recovered',0)}/{res.get('n_expected',0)}",
                   False)
            g.edge(did, rid, "Reference", "生成识别结果", f"{mid} -> 识别结果")
            g.edge("CodeFile:melody2score/core/pipeline.py", rid, "ReadWrite",
                   "写出识别结果", f"pipeline 产出 {mid} 音符序列")

    # 5) 音高检测后端依赖（实际使用的库）
    if backend == "torchcrepe":
        g.node("Dependency", "torchcrepe", "torchcrepe", "torchcrepe",
               "真实 CREPE tiny 模型（PyTorch/ONNXRuntime）", True)
        g.edge("CodeFile:melody2score/core/pitch.py", "Dependency:torchcrepe",
               "Dependency", "使用CREPE", "pitch.PitchDetector 后端 torchcrepe", True)
    elif backend == "crepe_onnx":
        g.node("Dependency", "crepe_onnx", "crepe_onnx", "crepe_onnx",
               "嵌入式 ONNX 版 CREPE tiny", True)
        g.edge("CodeFile:melody2score/core/pitch.py", "Dependency:crepe_onnx",
               "Dependency", "使用CREPE", "pitch.PitchDetector 后端 crepe_onnx", True)
    else:  # pyin
        g.node("Dependency", "librosa", "librosa", "librosa",
               "librosa.pyin 概率化 YIN 音高估计（兜底后端）", True)
        g.edge("CodeFile:melody2score/core/pitch.py", "Dependency:librosa",
               "Dependency", "使用pyin", "pitch.PitchDetector 后端 pyin(librosa)", True)

    # 6) 写出（多格式导出，遵循 GR-STD-V1.0，用最好的开源工具架构）
    out_dir = HERE
    data = g.to_json()

    # 6.1) 规范 JSON（可被 tools/info-graph 直接加载 / validate / 合并）
    out_json = os.path.join(out_dir, "melody_infograph.json")
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    # 6.2) Mermaid（文档/画布级静态图谱）
    out_mmd = os.path.join(out_dir, "melody_infograph.mmd")
    with open(out_mmd, "w", encoding="utf-8") as f:
        f.write(g.to_mermaid())

    # 6.3) Cytoscape.js 交互式网络图（零后端依赖单 HTML）
    out_html = os.path.join(out_dir, "melody_infograph.html")
    with open(out_html, "w", encoding="utf-8") as f:
        f.write(g.to_cytoscape_html("旋律转谱领域信息关联图谱"))

    print(f"已生成旋律领域子图（多格式导出）：")
    print(f"  JSON  : {out_json}  （{len(data['nodes'])} 节点 / {len(data['edges'])} 边，符合 GR-STD-V1.0）")
    print(f"  Mermaid: {out_mmd}")
    print(f"  HTML  : {out_html}  （Cytoscape.js 交互式，浏览器直接打开）")
    print(f"  识别后端：{backend}  样本数：{len(manifest)}")


# ===================== Cytoscape.js 交互式 HTML 模板 =====================
# 采用网络图领域事实标准 Cytoscape.js（CDN 引入，无需本地依赖/后端），
# 支持拖拽、缩放、滚轮平移、点击查看节点 summary / 边 evidence。
_CYTOSCAPE_HTML_TEMPLATE = r"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>__TITLE__</title>
<script src="https://cdn.jsdelivr.net/npm/cytoscape@3.30.2/dist/cytoscape.min.js"></script>
<style>
  html,body{margin:0;height:100%;font-family:-apple-system,"PingFang SC","Microsoft YaHei",sans-serif;background:#fafafa}
  #cy{position:absolute;inset:0;left:260px}
  #panel{position:absolute;top:0;left:0;bottom:0;width:260px;background:#fff;border-right:1px solid #e0e0e0;overflow:auto;padding:12px;box-sizing:border-box}
  #panel h1{font-size:15px;margin:0 0 8px}
  #panel .meta{font-size:12px;color:#666;margin-bottom:10px}
  #panel .legend{font-size:12px;line-height:1.9}
  #panel .legend i{display:inline-block;width:12px;height:12px;border-radius:2px;margin-right:6px;vertical-align:middle}
  #detail{position:absolute;right:0;bottom:0;max-width:360px;background:#fff;border:1px solid #e0e0e0;border-radius:8px;
          padding:10px 12px;font-size:12px;color:#333;display:none;box-shadow:0 2px 8px rgba(0,0,0,.12)}
  #detail h3{margin:0 0 6px;font-size:13px}
  #detail .k{color:#888}
</style>
</head>
<body>
<div id="panel">
  <h1>__TITLE__</h1>
  <div class="meta" id="meta"></div>
  <div class="legend" id="legend"></div>
</div>
<div id="cy"></div>
<div id="detail"></div>
<script>
const PAYLOAD = __PAYLOAD__;
const colors = PAYLOAD.colors || {};
document.getElementById('meta').textContent =
  PAYLOAD.elements.filter(e=>e.data.source===undefined).length + ' 节点 / ' +
  PAYLOAD.elements.filter(e=>e.data.source!==undefined).length + ' 边';
const legend = document.getElementById('legend');
Object.keys(colors).forEach(k=>{
  const d=document.createElement('div');
  d.innerHTML='<i style="background:'+colors[k]+'"></i>'+k;
  legend.appendChild(d);
});
const cy = cytoscape({
  container: document.getElementById('cy'),
  elements: PAYLOAD.elements,
  style: [
    { selector: 'node',
      style: {
        'background-color': ele => colors[ele.data('kind')] || '#cfd8dc',
        'label': 'data(label)',
        'color': '#222',
        'font-size': 9,
        'text-valign': 'center',
        'text-halign': 'center',
        'text-wrap': 'wrap',
        'text-max-width': 90,
        'width': 22, 'height': 22,
        'border-width': 1, 'border-color': '#888'
      }
    },
    { selector: 'edge',
      style: {
        'width': 1.2,
        'line-color': '#bbb',
        'target-arrow-color': '#bbb',
        'target-arrow-shape': 'triangle',
        'curve-style': 'bezier',
        'label': 'data(kind)',
        'font-size': 7,
        'color': '#999',
        'text-rotation': 'autorotate'
      }
    },
    { selector: 'node:selected', style: { 'border-width': 3, 'border-color': '#e65100' } },
    { selector: 'edge:selected', style: { 'line-color': '#e65100', 'target-arrow-color': '#e65100', 'width': 2.5 } }
  ],
  layout: { name: 'cose', animate: true, animationDuration: 600, padding: 30, nodeRepulsion: 8000, idealEdgeLength: 90 },
  wheelSensitivity: 0.2
});
const detail = document.getElementById('detail');
function show(el, data){
  let html = '';
  if (data.source === undefined){
    html += '<h3>'+data.label+'</h3>';
    html += '<div class="k">类型</div>'+data.kind+'<br/>';
    html += '<div class="k">ID</div>'+data.id+'<br/>';
    if (data.path) html += '<div class="k">路径</div>'+data.path+'<br/>';
    if (data.summary) html += '<div class="k">摘要</div>'+data.summary;
  } else {
    html += '<h3>'+data.kind+'</h3>';
    html += '<div class="k">From</div>'+data.source+'<br/>';
    html += '<div class="k">To</div>'+data.target+'<br/>';
    if (data.label) html += '<div class="k">标签</div>'+data.label+'<br/>';
    if (data.evidence) html += '<div class="k">证据</div>'+data.evidence;
  }
  detail.innerHTML = html;
  detail.style.display = 'block';
}
cy.on('tap', 'node', (e)=> show(e.target, e.target.data()));
cy.on('tap', 'edge', (e)=> show(e.target, e.target.data()));
cy.on('tap', (e)=> { if (e.target===cy) detail.style.display='none'; });
</script>
</body>
</html>
"""


if __name__ == "__main__":
    main()
