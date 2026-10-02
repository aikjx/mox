"""Validate and render business requirements; never infer runtime readiness."""
import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / "docs/modules/enterprise-capabilities"


def load():
    return json.loads((FOLDER / "registry.json").read_text(encoding="utf-8"))


def validate(data):
    if data.get("version") != 1 or not isinstance(data.get("modules"), list):
        raise ValueError("Unsupported capability registry")
    rows = data["modules"]
    ids = [r["id"] for r in rows]
    if len(ids) != len(set(ids)) or any(not re.fullmatch(r"EM-\d{2}", i) for i in ids):
        raise ValueError("Module identifiers must be unique EM-nn")
    domains = {p.name for p in (ROOT / "platform/domains").iterdir() if p.is_dir() and not p.name.startswith(".")}
    covered = set()
    requirement_ids = set()
    for row in rows:
        for key in ["name", "owns", "gap"]:
            if not isinstance(row.get(key), str) or not row[key].strip():
                raise ValueError(f"{row['id']}: missing {key}")
        if row.get("priority") not in ["P0", "P1", "P2", "P3"]:
            raise ValueError("Invalid priority")
        for key in ["domains", "owners", "flow"]:
            values = row.get(key)
            if not isinstance(values, list) or not values or any(not isinstance(v, str) or not v.strip() for v in values):
                raise ValueError(f"{row['id']}: invalid {key}")
        if not isinstance(row.get("requirements"), list) or not row["requirements"]:
            raise ValueError("Missing requirements")
        for requirement in row["requirements"]:
            key = requirement.get("id", "")
            if not re.fullmatch(re.escape(row["id"]) + r"-R\d{2}", key) or key in requirement_ids:
                raise ValueError("Requirement identifiers must be explicit and unique")
            if not isinstance(requirement.get("acceptance"), str) or not requirement["acceptance"].strip():
                raise ValueError("Missing acceptance criterion")
            requirement_ids.add(key)
        if set(row["domains"]) - domains:
            raise ValueError("Unknown code domain")
        covered.update(row["domains"])
        for value in row["owners"] + [row["authority"]]:
            path = ROOT / value
            try:
                path.resolve().relative_to(ROOT.resolve())
            except ValueError:
                raise ValueError(f"Invalid repository path: {value}")
            if Path(value).is_absolute() or ".." in Path(value).parts or not path.exists():
                raise ValueError(f"Invalid repository path: {value}")
        if any(d not in ids or d == row["id"] for d in row["depends_on"]):
            raise ValueError("Unknown or self dependency")
    if covered != domains:
        raise ValueError(f"Uncovered domains: {sorted(domains - covered)}")
    by_id = {r["id"]: r for r in rows}
    done, visiting = set(), set()

    def visit(key):
        if key in visiting:
            raise ValueError("Prerequisite cycle")
        if key in done:
            return
        visiting.add(key)
        for dep in by_id[key]["depends_on"]:
            visit(dep)
        visiting.remove(key)
        done.add(key)

    for key in ids:
        visit(key)
    return rows


def label(text):
    return json.dumps(text, ensure_ascii=False)


def link(path, title=None):
    return f"[{title or path}](<../../../{path}>)"


def render(rows):
    intro = ["", "由 `python scripts/registry/enterprise_capabilities.py` 从 registry.json 生成，请勿手改。",
             "", "需求与逻辑前置关系为目标契约；不表示所有能力已开发、部署或验收。运行现状、细节与证据只引用各模块权威，不复制为第二份状态主源。", "",
             "返回 [主题入口](README.md) · [登记源](registry.json)", ""]
    cards = ["# 企业功能模块与逐模块需求"] + intro
    for row in rows:
        key = row["id"]
        cards += [f"## {key} {row['name']}", "", f"优先级：{row['priority']}。业务域：{' / '.join(row['domains'])}。",
                  "", f"事实所有权：{row['owns']}。", "",
                  "代码落点：" + " · ".join(link(p) for p in row["owners"]), "",
                  "现状与详细契约：" + link(row["authority"]), "",
                  "前置能力：" + ("、".join(row["depends_on"]) or "无"), "",
                  "### 功能需求与验收", ""]
        cards += [f"- **{requirement['id']}**：{requirement['acceptance']}。" for requirement in row["requirements"]]
        cards += ["", f"当前核验边界：{row['gap']}。", "", "### 模块业务流程（验收目标）", "", "```mermaid", "flowchart LR"]
        prefix = key.replace("-", "")
        for i, stage in enumerate(row["flow"], 1):
            cards.append(f"    {prefix}S{i}[{label(stage)}]")
            if i > 1:
                cards.append(f"    {prefix}S{i-1} --> {prefix}S{i}")
        cards += [f"    {prefix}S2 --> {prefix}F[\"授权、依赖或执行失败：明确拒绝，不生成成功事实\"]", "```", ""]
    graph = ["# 模块前置能力与边界关系"] + intro
    graph += ["箭头表示调用或实施所需的前置能力 → 消费模块；这是逻辑依赖，不能解释为已部署进程或 Cargo 导入。真实代码依赖见 " + link("docs/modules/CODE-CATALOG.md", "代码目录") + "。", "", "```mermaid", "flowchart TD"]
    for row in rows:
        graph.append(f"    {row['id'].replace('-', '')}[{label(row['id'] + ' ' + row['name'])}]")
    for row in rows:
        graph += [f"    {d.replace('-', '')} --> {row['id'].replace('-', '')}" for d in row["depends_on"]]
    graph += ["```", "", "## 所有权和归一规则", "",
              "身份和权限只由 IAM 裁决；对象字节、文件元数据、知识版本、图投影、任务状态与消息回执分别归各自模块。跨域传标识、版本和回执，不共享可写内存集合或复制另一套主源。",
              "", "低代码仅装配已注册能力，不替代业务授权或事务。AI 分析、工具执行、外部接受、用户接受是不同事实；各阶段保留实际状态和证据。",
              "", "流程中的业务回调与读引用可能双向；不能为满足图形无环而隐藏业务关系。登记的 depends_on 仅是可排序的前置能力图，跨模块业务流程见 [业务主链](BUSINESS-FLOWS.md)。", ""]
    return {"MODULES.md": "\n".join(cards), "RELATIONSHIPS.md": "\n".join(graph)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rows = validate(load())
    for name, content in render(rows).items():
        path = FOLDER / name
        if args.check:
            if not path.exists() or path.read_text(encoding="utf-8") != content:
                raise SystemExit(f"Generated capability document has drifted: {name}")
        else:
            path.write_text(content, encoding="utf-8")
    print(f"Validated {len(rows)} modules, {sum(len(r['requirements']) for r in rows)} requirements; runtime readiness not asserted")


if __name__ == "__main__":
    main()
