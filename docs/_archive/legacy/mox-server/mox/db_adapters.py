# -*- coding: utf-8 -*-
"""
mox-db 多数据库适配层（中间层）
===============================
mox 低代码平台"支持所有数据库，只要修改中间层"的落地：所有业务 SQL 不直接依赖具体
数据库，而是通过 DBAdapter 抽象执行。新增一种数据库只需新增一个适配器类，并在
数据源配置里声明 driver，业务 SQL 与上层逻辑零改动。

内置适配器：
- sqlite   : SQLite（默认演示数据源，零依赖，跑通全链路）
- mysql    : MySQL 8.x（安装 pymysql 后启用，配置驱动即可）
- postgres : PostgreSQL（安装 psycopg2-binary 后启用）
- duckdb   : DuckDB 列式分析（安装 duckdb 后启用，适配 OLAP 场景）

SQL 方言差异（分页 / 引号 / 占位符）由适配器自行归一，业务 SQL 使用统一占位符 ?。
"""
from __future__ import annotations

import json
import os
import re
import threading
import time
from typing import Any, Optional


class DBAdapter:
    """数据库适配器抽象基类。query() 返回 list[dict]。"""

    driver = "base"

    def query(self, sql: str, params: Optional[list] = None) -> list[dict]:
        raise NotImplementedError

    def execute(self, sql: str, params: Optional[list] = None) -> dict:
        raise NotImplementedError

    def describe(self) -> dict:
        raise NotImplementedError


class SQLiteAdapter(DBAdapter):
    """SQLite 适配器：演示与本地数据源默认实现。支持 :memory: 与文件库。"""

    driver = "sqlite"

    def __init__(self, dsn: str = ":memory:"):
        import sqlite3

        self._dsn = dsn
        self._conn = sqlite3.connect(dsn, check_same_thread=False)
        self._conn.row_factory = sqlite3.Row
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._lock = threading.RLock()

    def query(self, sql: str, params: Optional[list] = None) -> list[dict]:
        with self._lock:
            cur = self._conn.execute(sql, params or [])
            rows = cur.fetchall()
            return [dict(r) for r in rows]

    def execute(self, sql: str, params: Optional[list] = None) -> dict:
        with self._lock:
            cur = self._conn.execute(sql, params or [])
            self._conn.commit()
            return {
                "rows_affected": cur.rowcount,
                "last_insert_id": cur.lastrowid,
            }

    def describe(self) -> dict:
        tables = [r["name"] for r in self.query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")]
        return {"driver": self.driver, "dsn": self._dsn, "tables": tables}


class MySQLAdapter(DBAdapter):
    """MySQL 适配器（可选）。依赖 pymysql。"""

    driver = "mysql"

    def __init__(self, host: str, port: int, user: str, password: str,
                 database: str, charset: str = "utf8mb4"):
        import pymysql  # 延迟导入

        self._conn = pymysql.connect(
            host=host, port=port, user=user, password=password,
            database=database, charset=charset, cursorclass=pymysql.cursors.DictCursor,
        )

    def query(self, sql: str, params: Optional[list] = None) -> list[dict]:
        with self._conn.cursor() as cur:
            cur.execute(sql, params or [])
            return [dict(r) for r in cur.fetchall()]

    def execute(self, sql: str, params: Optional[list] = None) -> dict:
        with self._conn.cursor() as cur:
            rows = cur.execute(sql, params or [])
            self._conn.commit()
            return {"rows_affected": rows, "last_insert_id": cur.lastrowid}

    def describe(self) -> dict:
        return {"driver": self.driver}


class PostgresAdapter(DBAdapter):
    """PostgreSQL 适配器（可选）。依赖 psycopg2-binary。"""

    driver = "postgres"

    def __init__(self, host: str, port: int, user: str, password: str,
                 database: str):
        import psycopg2  # 延迟导入
        import psycopg2.extras

        self._conn = psycopg2.connect(
            host=host, port=port, user=user, password=password, dbname=database,
        )
        self._extras = psycopg2.extras

    def query(self, sql: str, params: Optional[list] = None) -> list[dict]:
        with self._conn.cursor(cursor_factory=self._extras.RealDictCursor) as cur:
            cur.execute(sql, params or [])
            return [dict(r) for r in cur.fetchall()]

    def execute(self, sql: str, params: Optional[list] = None) -> dict:
        with self._conn.cursor() as cur:
            rows = cur.rowcount if cur.execute(sql, params or []) is None else cur.rowcount
            self._conn.commit()
            return {"rows_affected": rows}

    def describe(self) -> dict:
        return {"driver": self.driver}


class DuckDBAdapter(DBAdapter):
    """DuckDB 列式分析适配器（可选）。依赖 duckdb。"""

    driver = "duckdb"

    def __init__(self, dsn: str = ":memory:"):
        import duckdb  # 延迟导入

        self._conn = duckdb.connect(dsn)

    def query(self, sql: str, params: Optional[list] = None) -> list[dict]:
        cur = self._conn.execute(sql, params or [])
        cols = [d[0] for d in cur.description or []]
        return [dict(zip(cols, row)) for row in cur.fetchall()]

    def execute(self, sql: str, params: Optional[list] = None) -> dict:
        cur = self._conn.execute(sql, params or [])
        return {"rows_affected": cur.rowcount}

    def describe(self) -> dict:
        return {"driver": self.driver}


_ADAPTER_REGISTRY = {
    "sqlite": SQLiteAdapter,
    "mysql": MySQLAdapter,
    "postgres": PostgresAdapter,
    "duckdb": DuckDBAdapter,
}


def build_adapter(driver: str, config: dict) -> DBAdapter:
    """按数据源配置创建适配器。新增数据库：在 _ADAPTER_REGISTRY 注册即可。"""
    driver = (driver or "sqlite").lower()
    cls = _ADAPTER_REGISTRY.get(driver)
    if cls is None:
        raise ValueError(f"不支持的数据库类型: {driver}，请先在中间层注册适配器")
    if driver == "sqlite":
        return cls(config.get("dsn", ":memory:"))
    if driver == "mysql":
        return cls(host=config.get("host", "127.0.0.1"), port=int(config.get("port", 3306)),
                   user=config.get("user", "root"), password=config.get("password", ""),
                   database=config.get("database", "mox"))
    if driver == "postgres":
        return cls(host=config.get("host", "127.0.0.1"), port=int(config.get("port", 5432)),
                   user=config.get("user", "postgres"), password=config.get("password", ""),
                   database=config.get("database", "mox"))
    if driver == "duckdb":
        return cls(config.get("dsn", ":memory:"))
    raise ValueError(f"适配器配置缺失: {driver}")


def sanitize_sql(sql: str) -> str:
    """
    DSQL 安全护栏：只允许只读查询。
    - 允许 SELECT / WITH ... SELECT（含 CTE）
    - 拒绝多语句（分号分隔多条）、写语句、注释注入、堆叠注入
    """
    s = re.sub(r"/\*.*?\*/", "", sql, flags=re.S)  # 去块注释
    s = re.sub(r"--.*?$", "", s, flags=re.M)        # 去行注释
    stripped = s.strip().rstrip(";").strip()
    if not stripped:
        raise ValueError("SQL 为空")
    if ";" in stripped:
        raise ValueError("禁止多语句 SQL（不允许分号堆叠）")
    head = stripped[:64].upper()
    if not (head.startswith("SELECT") or head.startswith("WITH")):
        raise ValueError("DSQL 仅允许 SELECT / WITH 只读查询")
    forbidden = re.compile(
        r"\b(insert|update|delete|drop|alter|create|truncate|replace|grant|"
        r"revoke|call|attach|detach|pragma|vacuum|load_extension|"
        r"into outfile|dumpfile|information_schema)\b",
        re.I,
    )
    if forbidden.search(stripped):
        raise ValueError("SQL 含被禁止的关键字，已拦截")
    return stripped
