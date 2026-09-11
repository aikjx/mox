# -*- coding: utf-8 -*-
"""
mox-apps 无限发布系统 · 应用管理中心
====================================
多应用（企业官网/业务系统）的创建、配置、发布、下线全生命周期管理。
- 应用状态机：draft → prepared → published → running → offline（含回退）
- 发布日志：每次发布/下线记录版本与操作人
- 应用维度 SQL 统计：每个应用拥有独立 SQL 集合（code 全局唯一，app_key 归属）
"""
from __future__ import annotations

import json
import time
from typing import Any, Optional

from .process import can_transition, next_states


class AppManager:
    def __init__(self, meta: Any):
        self._meta = meta

    # ---------------- 查询 ----------------
    def list_apps(self) -> list[dict]:
        rows = self._meta.query(
            "SELECT id,app_key,name,type,domain,status,config_json,publish_version,created_at,updated_at "
            "FROM apps ORDER BY updated_at DESC")
        for r in rows:
            try:
                r["config"] = json.loads(r.get("config_json") or "{}")
            except Exception:  # noqa: BLE001
                r["config"] = {}
        return rows

    def get_app(self, app_key: str) -> Optional[dict]:
        rows = self._meta.query("SELECT * FROM apps WHERE app_key=?", [app_key])
        if not rows:
            return None
        r = rows[0]
        try:
            r["config"] = json.loads(r.get("config_json") or "{}")
        except Exception:  # noqa: BLE001
            r["config"] = {}
        return r

    def sql_count(self, app_key: str) -> int:
        return self._meta.query("SELECT COUNT(*) c FROM dsql_sqls WHERE app_key=?", [app_key])[0]["c"]

    def enrich(self, app: dict) -> dict:
        app["sql_count"] = self.sql_count(app["app_key"])
        return app

    # ---------------- 写 ----------------
    def create_app(self, app_key: str, name: str, app_type: str = "website",
                   domain: str = "", config: Optional[dict] = None) -> dict:
        app_key = app_key.strip()
        if not app_key or not name:
            raise ValueError("app_key 与 name 必填")
        if self.get_app(app_key):
            raise ValueError(f"应用已存在: {app_key}")
        now = int(time.time())
        self._meta.execute(
            "INSERT INTO apps(app_key,name,type,domain,status,config_json,publish_version,created_at,updated_at) "
            "VALUES(?,?,?,?,?,?,?,?,?)",
            [app_key, name, app_type, domain or "", "draft",
             json.dumps(config or {}, ensure_ascii=False), 0, now, now])
        return self.enrich(self.get_app(app_key))

    def update_app(self, app_key: str, name: Optional[str] = None, domain: Optional[str] = None,
                   config: Optional[dict] = None) -> dict:
        app = self.get_app(app_key)
        if not app:
            raise KeyError(f"应用不存在: {app_key}")
        if name is not None:
            app["name"] = name
        if domain is not None:
            app["domain"] = domain
        if config is not None:
            app["config"] = config
        self._meta.execute(
            "UPDATE apps SET name=?, domain=?, config_json=?, updated_at=? WHERE app_key=?",
            [app["name"], app.get("domain", ""), json.dumps(app.get("config", {}), ensure_ascii=False),
             int(time.time()), app_key])
        return self.enrich(self.get_app(app_key))

    def delete_app(self, app_key: str) -> int:
        if app_key == "mox":
            raise ValueError("默认应用 mox 不可删除")
        return self._meta.execute("DELETE FROM apps WHERE app_key=?", [app_key])["rows_affected"]

    def transition(self, app_key: str, target: str, operator: str = "admin") -> dict:
        """状态机流转 + 发布日志。"""
        app = self.get_app(app_key)
        if not app:
            raise KeyError(f"应用不存在: {app_key}")
        cur = app["status"]
        if target not in ("draft", "prepared", "published", "running", "offline"):
            raise ValueError(f"非法状态: {target}")
        if not can_transition(cur, target):
            raise ValueError(f"不允许从 {cur} 流转到 {target}（可选: {next_states(cur)}）")
        version = app["publish_version"]
        if target in ("published", "running"):
            version = app["publish_version"] + 1
        now = int(time.time())
        self._meta.execute("UPDATE apps SET status=?, publish_version=?, updated_at=? WHERE app_key=?",
                           [target, version, now, app_key])
        self._meta.execute(
            "INSERT INTO publish_logs(app_key,action,version,operator,detail,created_at) "
            "VALUES(?,?,?,?,?,?)",
            [app_key, f"{cur}→{target}", version, operator, f"发布版本 v{version}", now])
        return self.enrich(self.get_app(app_key))

    def publish_logs(self, app_key: Optional[str] = None, limit: int = 30) -> list[dict]:
        sql = "SELECT app_key,action,version,operator,detail,created_at FROM publish_logs"
        params: list = []
        if app_key:
            sql += " WHERE app_key=?"
            params.append(app_key)
        sql += " ORDER BY id DESC LIMIT ?"
        params.append(min(limit, 200))
        return self._meta.query(sql, params)
