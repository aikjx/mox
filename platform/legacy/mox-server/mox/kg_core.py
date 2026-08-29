# -*- coding: utf-8 -*-
"""
mox-kg-core 自研知识图谱引擎
============================
mox 低代码平台自研知识图谱（不依赖 Neo4j/Nebula 等外部图数据库），支持：

- 实体（顶点）：type / label / props / domain（行业域，用于多行业融合与隔离）
- 关系（边）：source -> target，带 relation 标签与权重
- 邻接遍历：one-hop 邻居（出边 / 入边）
- 多跳查询：k 跳可达集合
- 最短路径：无权图 BFS
- 子图导出：供前端可视化（nodes + edges）
- 行业融合：domain 过滤 + 跨域关联查询，实现"通过知识图谱快速融合所有行业，无限扩展"

存储：顶点表 + 边表（SQLite 持久化），读写带线程锁。
"""
from __future__ import annotations

import json
import threading
import time
from collections import deque
from typing import Any, Optional


class KnowledgeGraph:
    """自研图引擎：基于邻接表 + 元数据持久化。"""

    def __init__(self, meta: Any, kg_vertices_tbl: str = "kg_vertices",
                 kg_edges_tbl: str = "kg_edges"):
        self._meta = meta  # 元数据库连接（提供 query/execute）
        self._vt = kg_vertices_tbl
        self._et = kg_edges_tbl
        self._lock = threading.RLock()

    # ---------------- 顶点 ----------------
    def upsert_vertex(self, vid: str, vtype: str, label: str,
                      props: Optional[dict] = None,
                      domain: str = "default") -> dict:
        props = props or {}
        with self._lock:
            rows = self._meta.query(
                f"SELECT vid FROM {self._vt} WHERE vid = ?", [vid])
            if rows:
                self._meta.execute(
                    f"UPDATE {self._vt} SET type=?, label=?, props=?, domain=?, updated_at=? "
                    f"WHERE vid=?",
                    [vtype, label, json.dumps(props, ensure_ascii=False),
                     domain, int(time.time()), vid])
            else:
                self._meta.execute(
                    f"INSERT INTO {self._vt}(vid,type,label,props,domain,created_at,updated_at) "
                    f"VALUES(?,?,?,?,?,?,?)",
                    [vid, vtype, label, json.dumps(props, ensure_ascii=False),
                     domain, int(time.time()), int(time.time())])
        return {"vid": vid, "type": vtype, "label": label, "props": props, "domain": domain}

    def get_vertex(self, vid: str) -> Optional[dict]:
        rows = self._meta.query(
            f"SELECT vid,type,label,props,domain FROM {self._vt} WHERE vid=?", [vid])
        if not rows:
            return None
        r = rows[0]
        r["props"] = json.loads(r.get("props") or "{}")
        return r

    def delete_vertex(self, vid: str) -> int:
        with self._lock:
            # 级联删除关联边
            self._meta.execute(f"DELETE FROM {self._et} WHERE source=? OR target=?", [vid, vid])
            return self._meta.execute(f"DELETE FROM {self._vt} WHERE vid=?", [vid])["rows_affected"]

    def list_vertices(self, domain: Optional[str] = None, vtype: Optional[str] = None) -> list[dict]:
        sql = f"SELECT vid,type,label,props,domain FROM {self._vt} WHERE 1=1"
        params: list = []
        if domain:
            sql += " AND domain=?"
            params.append(domain)
        if vtype:
            sql += " AND type=?"
            params.append(vtype)
        sql += " ORDER BY updated_at DESC"
        rows = self._meta.query(sql, params)
        for r in rows:
            r["props"] = json.loads(r.get("props") or "{}")
        return rows

    # ---------------- 边 ----------------
    def upsert_edge(self, source: str, relation: str, target: str,
                    weight: float = 1.0) -> dict:
        with self._lock:
            rows = self._meta.query(
                f"SELECT id FROM {self._et} WHERE source=? AND target=? AND relation=?",
                [source, target, relation])
            if rows:
                self._meta.execute(
                    f"UPDATE {self._et} SET weight=?, updated_at=? WHERE id=?",
                    [weight, int(time.time()), rows[0]["id"]])
            else:
                self._meta.execute(
                    f"INSERT INTO {self._et}(source,relation,target,weight,created_at,updated_at) "
                    f"VALUES(?,?,?,?,?,?)",
                    [source, relation, target, weight, int(time.time()), int(time.time())])
        return {"source": source, "relation": relation, "target": target, "weight": weight}

    def delete_edge(self, source: str, relation: str, target: str) -> int:
        return self._meta.execute(
            f"DELETE FROM {self._et} WHERE source=? AND relation=? AND target=?",
            [source, relation, target])["rows_affected"]

    def list_edges(self, domain: Optional[str] = None) -> list[dict]:
        sql = (f"SELECT e.source,e.relation,e.target,e.weight "
               f"FROM {self._et} e "
               f"JOIN {self._vt} sv ON sv.vid=e.source "
               f"JOIN {self._vt} tv ON tv.vid=e.target "
               f"WHERE 1=1")
        params: list = []
        if domain:
            sql += " AND (sv.domain=? OR tv.domain=?)"
            params += [domain, domain]
        return self._meta.query(sql, params)

    # ---------------- 查询能力 ----------------
    def neighbors(self, vid: str, direction: str = "out") -> list[dict]:
        """邻接遍历：direction = out | in | both。"""
        with self._lock:
            out_rows = []
            in_rows = []
            if direction in ("out", "both"):
                out_rows = self._meta.query(
                    f"SELECT e.target AS neighbor, e.relation FROM {self._et} e "
                    f"WHERE e.source=?", [vid])
            if direction in ("in", "both"):
                in_rows = self._meta.query(
                    f"SELECT e.source AS neighbor, e.relation FROM {self._et} e "
                    f"WHERE e.target=?", [vid])
            result = []
            for r in out_rows + in_rows:
                nb = self.get_vertex(r["neighbor"])
                if nb:
                    result.append({"relation": r["relation"], "vertex": nb})
            return result

    def reachable(self, vid: str, hops: int = 2, direction: str = "out") -> list[dict]:
        """多跳可达集合（BFS），返回 {vid, label, type, hops, path}。"""
        if hops < 1:
            return []
        visited = {vid: 0}
        queue = deque([vid])
        result: list[dict] = []
        while queue:
            cur = queue.popleft()
            cur_hops = visited[cur]
            if cur_hops >= hops:
                continue
            for nb in self.neighbors(cur, direction):
                nid = nb["vertex"]["vid"]
                if nid in visited:
                    continue
                visited[nid] = cur_hops + 1
                queue.append(nid)
                result.append({
                    "vid": nid,
                    "label": nb["vertex"]["label"],
                    "type": nb["vertex"]["type"],
                    "hops": cur_hops + 1,
                    "relation": nb["relation"],
                    "via": cur,
                })
        return result

    def shortest_path(self, start: str, end: str, direction: str = "out") -> Optional[list[str]]:
        """无权图 BFS 最短路径。"""
        if start == end:
            return [start]
        visited = {start}
        queue = deque([[start]])
        while queue:
            path = queue.popleft()
            cur = path[-1]
            for nb in self.neighbors(cur, direction):
                nid = nb["vertex"]["vid"]
                if nid in visited:
                    continue
                new_path = path + [nid]
                if nid == end:
                    return new_path
                visited.add(nid)
                queue.append(new_path)
        return None

    def graph_data(self, domain: Optional[str] = None) -> dict:
        """导出可视化子图：{ nodes:[...], edges:[...] }。"""
        nodes = self.list_vertices(domain=domain)
        edges = self.list_edges(domain=domain)
        ids = {n["vid"] for n in nodes}
        edges = [e for e in edges if e["source"] in ids and e["target"] in ids]
        return {"nodes": nodes, "edges": edges, "vertex_count": len(nodes), "edge_count": len(edges)}

    def stats(self) -> dict:
        v = self._meta.query(f"SELECT COUNT(*) AS c FROM {self._vt}")[0]["c"]
        e = self._meta.query(f"SELECT COUNT(*) AS c FROM {self._et}")[0]["c"]
        types = self._meta.query(
            f"SELECT type, COUNT(*) AS c FROM {self._vt} GROUP BY type ORDER BY c DESC")
        return {"vertices": v, "edges": e, "types": {t["type"]: t["c"] for t in types}}
