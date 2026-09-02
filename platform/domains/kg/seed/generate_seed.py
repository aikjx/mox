#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
功能需求知识图谱种子数据生成器
从 functional-requirements-inventory.json + 前端 API 文件生成 KG 节点和边
输出: functional-requirements-graph-seed.json
"""
import json
import re
import os
from pathlib import Path

# ========== 路径配置 ==========
BASE_DIR = Path(r"D:\a10\aikjx\gitcode\infotopograph")
INVENTORY_PATH = BASE_DIR / "platform" / "domains" / "kg" / "seed" / "functional-requirements-inventory.json"
API_DIR = BASE_DIR / "frontend-ui" / "src" / "api"
OUTPUT_PATH = BASE_DIR / "platform" / "domains" / "kg" / "seed" / "functional-requirements-graph-seed.json"

# ========== 边权重配置 ==========
EDGE_WEIGHTS = {
    "belongs_to": 0.9,
    "has_function": 0.8,
    "has_endpoint": 0.8,
    "calls": 0.85,
    "implements": 0.7,
    "blocked_by": 0.95,
    "related_gap": 0.6,
    "depends_on": 0.75,
}


def sanitize_id(s):
    """将路径/字符串转换为合法的ID后缀"""
    s = re.sub(r'[^a-zA-Z0-9_]', '_', s)
    s = re.sub(r'_+', '_', s)
    return s.strip('_')


def parse_http_api_file(filepath):
    """
    解析使用 http.js (axios baseURL=/api) 的 API 文件
    返回 {function_name: {http_method, target_path, file_path}}
    """
    results = {}
    fname = os.path.basename(filepath)
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception:
        return results

    # 匹配 export const funcName = (...) => http.METHOD('/path', ...)
    # 也匹配 export const funcName = http.get(...) 别名
    pattern = re.compile(
        r"export\s+const\s+(\w+)\s*=\s*(?:"
        r"(?:\([^)]*\)\s*=>\s*)?"
        r"http\.(get|post|put|delete|patch)\s*\(\s*"
        r"(?:'([^']*)'|\"([^\"]*)\"|`([^`]*)`)"
        r")"
    )
    for m in pattern.finditer(content):
        func_name = m.group(1)
        method = m.group(2).upper()
        path = m.group(3) or m.group(4) or m.group(5) or ""
        # 处理模板字符串中的 ${...} — 保留路径模板
        path = re.sub(r'\$\{[^}]*\}', '{}', path)
        # http.js baseURL=/api, 所以目标路径是 /api + path
        target_path = "/api" + path if not path.startswith("/api") else path
        results[func_name] = {
            "http_method": method,
            "target_path": target_path,
            "file_path": f"src/api/{fname}",
        }

    # 匹配别名导出: export const funcName = otherFunc
    alias_pattern = re.compile(r"export\s+const\s+(\w+)\s*=\s*(\w+)\s*$", re.MULTILINE)
    for m in alias_pattern.finditer(content):
        alias_name = m.group(1)
        orig_name = m.group(2)
        if orig_name in results and alias_name not in results:
            results[alias_name] = dict(results[orig_name])
            results[alias_name]["file_path"] = f"src/api/{fname}"
            results[alias_name]["alias_of"] = orig_name

    return results


def parse_alliance_file(filepath):
    """
    解析 alliance.js (直接 fetch, 无 /api 前缀)
    返回 {function_name: {http_method, target_path, file_path}}
    """
    results = {}
    fname = os.path.basename(filepath)
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception:
        return results

    # 匹配 async function funcName(...) { ... fetch(`${ALLIANCE_BASE}/path`, { method: "METHOD" ... })
    # 需要在函数体内找 fetch 调用
    func_pattern = re.compile(
        r"(?:async\s+)?function\s+(\w+)\s*\([^)]*\)\s*\{([^}]*?(?:\{[^}]*\}[^}]*?)*)\}",
        re.DOTALL
    )
    fetch_pattern = re.compile(
        r"fetch\s*\(\s*`\$\{ALLIANCE_BASE\}([^`]*)`\s*,\s*\{[^}]*?method\s*:\s*[\"'](\w+)[\"']",
        re.DOTALL
    )

    for m in func_pattern.finditer(content):
        func_name = m.group(1)
        body = m.group(2)
        fm = fetch_pattern.search(body)
        if fm:
            path = fm.group(1)
            method = fm.group(2).upper()
            path = re.sub(r'\$\{[^}]*\}', '{}', path)
            results[func_name] = {
                "http_method": method,
                "target_path": path,  # alliance.js 无 /api 前缀
                "file_path": f"src/api/{fname}",
            }

    return results


def build_api_function_info():
    """扫描所有前端 API 文件，构建函数信息字典"""
    info = {}
    api_files = sorted(API_DIR.glob("*.js"))
    for fpath in api_files:
        if fpath.name in ("http.js", "index.js"):
            continue
        if fpath.name == "alliance.js":
            info.update(parse_alliance_file(str(fpath)))
        else:
            info.update(parse_http_api_file(str(fpath)))
    return info


def normalize_list(val):
    """将空格分隔字符串或数组统一为列表"""
    if val is None:
        return []
    if isinstance(val, list):
        return [v for v in val if v]
    if isinstance(val, str):
        return [v.strip() for v in val.split() if v.strip()]
    return []


def generate_seed():
    # 读取 inventory
    with open(INVENTORY_PATH, 'r', encoding='utf-8') as f:
        inv = json.load(f)

    # 构建 API 函数信息
    api_info = build_api_function_info()
    print(f"[INFO] 从前端 API 文件解析到 {len(api_info)} 个函数定义")

    nodes = []
    edges = []
    node_ids = set()
    edge_keys = set()

    def add_node(nid, label, ntype, props):
        if nid in node_ids:
            return
        node_ids.add(nid)
        nodes.append({
            "id": nid,
            "label": label,
            "node_type": ntype,
            "properties": props,
        })

    def add_edge(source, target, relation):
        key = f"{source}->{target}:{relation}"
        if key in edge_keys:
            return
        edge_keys.add(key)
        edges.append({
            "source": source,
            "target": target,
            "weight": EDGE_WEIGHTS.get(relation, 0.5),
            "relation_type": relation,
        })

    # 构建页面路由映射 (从 frontend_views_inventory)
    page_route_map = {}
    page_name_map = {}
    page_domain_map = {}
    for pv in inv.get("frontend_views_inventory", []):
        page_route_map[pv["path"]] = pv.get("route", "")
        page_name_map[pv["path"]] = pv.get("name", pv["path"])
        page_domain_map[pv["path"]] = pv.get("domain", "")

    # ========== 1. Domain 节点 ==========
    domain_ids = set()
    for dom in inv["domains"]:
        did = f"dom_{dom['id']}"
        domain_ids.add(dom['id'])
        add_node(did, dom["name"], "domain", {
            "domain": dom["id"],
            "description": dom["name"],
        })

    # ========== 2-6. 遍历 domains/modules/features ==========
    page_to_features = {}  # page_path -> [feature_id]
    apifn_to_features = {}  # func_name -> [feature_id]
    endpoint_to_features = {}  # endpoint_id -> [feature_id]

    for dom in inv["domains"]:
        dom_id = dom["id"]
        for mod in dom.get("modules", []):
            mod_id = f"mod_{mod['id']}"
            # Module 节点
            add_node(mod_id, mod["name"], "module", {
                "domain": dom_id,
                "description": mod["name"],
            })
            # module -> domain (belongs_to)
            add_edge(mod_id, f"dom_{dom_id}", "belongs_to")

            for feat in mod.get("features", []):
                feat_id = feat["id"]
                # Feature 节点
                add_node(feat_id, feat["name"], "feature", {
                    "domain": dom_id,
                    "status": feat.get("status", "planned"),
                    "priority": feat.get("priority", "P3"),
                    "missing_backend": feat.get("missing_backend", False),
                    "description": feat.get("description", ""),
                })
                # feature -> module (belongs_to)
                add_edge(feat_id, mod_id, "belongs_to")

                # Frontend pages
                pages = normalize_list(feat.get("frontend_pages", []))
                for page_path in pages:
                    page_sid = sanitize_id(page_path)
                    page_id = f"page_{page_sid}"
                    page_name = page_name_map.get(page_path, os.path.basename(page_path))
                    page_route = page_route_map.get(page_path, "")
                    page_dom = page_domain_map.get(page_path, dom_id)
                    add_node(page_id, page_name, "frontend_page", {
                        "domain": page_dom,
                        "file_path": page_path,
                        "route_path": page_route,
                        "description": page_name,
                    })
                    # frontend_page -> feature (implements)
                    add_edge(page_id, feat_id, "implements")
                    page_to_features.setdefault(page_path, []).append(feat_id)

                # API functions
                funcs = normalize_list(feat.get("frontend_api_functions", []))
                for func_name in funcs:
                    apifn_id = f"apifn_{func_name}"
                    finfo = api_info.get(func_name, {})
                    add_node(apifn_id, func_name, "api_function", {
                        "domain": dom_id,
                        "file_path": finfo.get("file_path", ""),
                        "function_name": func_name,
                        "http_method": finfo.get("http_method", "UNKNOWN"),
                        "target_path": finfo.get("target_path", ""),
                        "description": func_name,
                    })
                    # feature -> api_function (has_function)
                    add_edge(feat_id, apifn_id, "has_function")
                    apifn_to_features.setdefault(func_name, []).append(feat_id)

                # Backend endpoints
                endpoints = feat.get("backend_endpoints", [])
                if isinstance(endpoints, str):
                    endpoints = []  # 端点是对象数组，不是字符串
                for ep in endpoints:
                    if not isinstance(ep, dict):
                        continue
                    method = ep.get("method", "GET")
                    path = ep.get("path", "")
                    handler = ep.get("handler", "")
                    ep_sid = sanitize_id(f"{method}_{path}")
                    ep_id = f"ep_{ep_sid}"
                    # 确定 source
                    source = "orchestrator"
                    if path.startswith("/api/system/") or path.startswith("/api/security/"):
                        source = "gateway"
                    elif path.startswith("/api/projects/"):
                        source = "primiflow"
                    add_node(ep_id, f"{method} {path}", "backend_endpoint", {
                        "domain": dom_id,
                        "http_method": method,
                        "path": path,
                        "handler": handler,
                        "source": source,
                        "description": f"{method} {path} ({handler})",
                    })
                    # feature -> backend_endpoint (has_endpoint)
                    add_edge(feat_id, ep_id, "has_endpoint")
                    endpoint_to_features.setdefault(ep_id, []).append(feat_id)

                # blocked_by gaps
                blocked = normalize_list(feat.get("blocked_by", []))
                for gap_id in blocked:
                    # gap 节点在后面统一创建，这里先建边
                    add_edge(feat_id, gap_id, "blocked_by")

    # ========== api_function -> backend_endpoint (calls) ==========
    # 当函数的 target_path 与端点 path 匹配时
    endpoint_by_path = {}
    for n in nodes:
        if n["node_type"] == "backend_endpoint":
            p = n["properties"]["path"]
            endpoint_by_path.setdefault(p, []).append(n["id"])

    for n in nodes:
        if n["node_type"] == "api_function":
            target = n["properties"].get("target_path", "")
            if not target:
                continue
            # 精确匹配
            if target in endpoint_by_path:
                for epid in endpoint_by_path[target]:
                    add_edge(n["id"], epid, "calls")
            else:
                # 尝试路径参数匹配: 将端点路径中的 :param 替换为 {}
                for ep_path, ep_ids in endpoint_by_path.items():
                    # 规范化: 端点路径 /api/ai/chat/history/:session vs 函数 /api/ai/chat/history/{}
                    ep_norm = re.sub(r':\w+', '{}', ep_path)
                    target_norm = target
                    if ep_norm == target_norm:
                        for epid in ep_ids:
                            add_edge(n["id"], epid, "calls")
                        break

    # ========== 7. Gap 节点 ==========
    for gap in inv.get("gaps", []):
        gap_id = gap["id"]
        affected = gap.get("affected_frontend_functions", [])
        if isinstance(affected, str):
            affected = affected.split()
        add_node(gap_id, gap_id, "gap", {
            "domain": gap.get("domain", ""),
            "severity": gap.get("severity", "P2"),
            "status": gap.get("status", "open"),
            "affected_count": len(affected),
            "description": gap.get("description", ""),
        })

    # ========== 8. Pending Decision 节点 ==========
    for dec in inv.get("pending_decisions", []):
        # decision_xxx -> dec_xxx
        dec_id = dec["id"].replace("decision_", "dec_", 1)
        add_node(dec_id, dec.get("title", dec_id), "pending_decision", {
            "domain": dec.get("domain", ""),
            "status": "pending",
            "decision_options": dec.get("description", ""),
            "description": dec.get("description", ""),
        })

    # ========== pending_decision -> gap (related_gap) 按域关联 ==========
    gap_by_domain = {}
    for n in nodes:
        if n["node_type"] == "gap":
            d = n["properties"].get("domain", "")
            gap_by_domain.setdefault(d, []).append(n["id"])

    for n in nodes:
        if n["node_type"] == "pending_decision":
            d = n["properties"].get("domain", "")
            # 按域关联: 如果决策域与缺口域匹配
            for gap_dom, gap_ids in gap_by_domain.items():
                if gap_dom == d or d in ("data", "backend", "frontend", "security"):
                    # data/backend/frontend 是跨域决策，关联所有同域或相关缺口
                    if gap_dom == d or (d == "security" and gap_dom == "security"):
                        for gid in gap_ids:
                            add_edge(n["id"], gid, "related_gap")

    # ========== 统计 ==========
    type_counts = {}
    for n in nodes:
        nt = n["node_type"]
        type_counts[nt] = type_counts.get(nt, 0) + 1

    rel_counts = {}
    for e in edges:
        rt = e["relation_type"]
        rel_counts[rt] = rel_counts.get(rt, 0) + 1

    print(f"[INFO] 节点总数: {len(nodes)}")
    for nt, cnt in sorted(type_counts.items()):
        print(f"  {nt}: {cnt}")
    print(f"[INFO] 边总数: {len(edges)}")
    for rt, cnt in sorted(rel_counts.items()):
        print(f"  {rt}: {cnt}")

    # ========== 输出 ==========
    seed = {"nodes": nodes, "edges": edges}
    with open(OUTPUT_PATH, 'w', encoding='utf-8') as f:
        json.dump(seed, f, ensure_ascii=False, indent=2)
    print(f"[OK] 种子数据已保存: {OUTPUT_PATH}")
    return seed


if __name__ == "__main__":
    generate_seed()
