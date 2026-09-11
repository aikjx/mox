# -*- coding: utf-8 -*-
"""
mox-cache 缓存适配层
====================
mox 低代码平台统一缓存抽象。业务 SQL 查询结果可按 sql_code + 参数哈希 + 权限维度缓存，
命中缓存时跳过数据库执行，实现"比写死 SQL 更快"的快速查询目标。

设计要点：
- CacheAdapter 为统一抽象，提供 get/set/delete/clear/stats。
- MemoryCache 为默认实现：LRU 淘汰 + TTL 过期，线程安全，进程内零依赖。
- RedisCache 为可插拔实现：检测到 redis-py 时启用，即可切换为 redis 缓存。
- 通过配置项 CACHE_DRIVER = memory | redis 一键切换，无需改动业务代码。
  —— 这正是"支持所有数据库 / 只要修改中间层即可"的同一思想在缓存层的体现。
"""
from __future__ import annotations

import hashlib
import json
import threading
import time
from typing import Any, Optional


class CacheAdapter:
    """缓存适配器抽象基类。新增缓存后端只需继承并实现四个方法。"""

    name = "base"

    def get(self, key: str) -> Optional[Any]:
        raise NotImplementedError

    def set(self, key: str, value: Any, ttl: int) -> None:
        raise NotImplementedError

    def delete(self, key: str) -> None:
        raise NotImplementedError

    def clear(self) -> int:
        raise NotImplementedError

    def stats(self) -> dict:
        raise NotImplementedError


class MemoryCache(CacheAdapter):
    """进程内 LRU + TTL 缓存。默认容量 20_000 条。"""

    name = "memory"

    def __init__(self, capacity: int = 20_000, default_ttl: int = 60):
        self._capacity = capacity
        self._default_ttl = default_ttl
        self._store: "dict[str, tuple[float, Any]]" = {}
        self._order: "dict[str, float]" = {}  # key -> last access seq
        self._seq = 0
        self._hits = 0
        self._misses = 0
        self._lock = threading.RLock()

    def _touch(self, key: str):
        self._seq += 1
        self._order[key] = self._seq

    def _evict_if_needed(self):
        while len(self._store) >= self._capacity and self._store:
            # 淘汰最久未使用
            lru_key = min(self._order, key=self._order.get)
            self._store.pop(lru_key, None)
            self._order.pop(lru_key, None)

    def get(self, key: str) -> Optional[Any]:
        with self._lock:
            item = self._store.get(key)
            if item is None:
                self._misses += 1
                return None
            expire_at, value = item
            if expire_at is not None and time.time() > expire_at:
                self._store.pop(key, None)
                self._order.pop(key, None)
                self._misses += 1
                return None
            self._hits += 1
            self._touch(key)
            return value

    def set(self, key: str, value: Any, ttl: int) -> None:
        with self._lock:
            expire_at = time.time() + ttl if ttl and ttl > 0 else None
            self._store[key] = (expire_at, value)
            self._touch(key)
            self._evict_if_needed()

    def delete(self, key: str) -> None:
        with self._lock:
            self._store.pop(key, None)
            self._order.pop(key, None)

    def clear(self) -> int:
        with self._lock:
            n = len(self._store)
            self._store.clear()
            self._order.clear()
            self._hits = self._misses = 0
            return n

    def stats(self) -> dict:
        with self._lock:
            total = self._hits + self._misses
            return {
                "driver": self.name,
                "capacity": self._capacity,
                "size": len(self._store),
                "hits": self._hits,
                "misses": self._misses,
                "hit_rate": round(self._hits / total, 4) if total else 0.0,
                "default_ttl": self._default_ttl,
            }


class RedisCache(CacheAdapter):
    """Redis 缓存（可选）。安装 redis-py 后启用：CACHE_DRIVER=redis。"""

    name = "redis"

    def __init__(self, url: str = "redis://127.0.0.1:6379/0", default_ttl: int = 60):
        import redis  # 延迟导入，未安装时不阻塞启动

        self._client = redis.Redis.from_url(url, decode_responses=True)
        self._default_ttl = default_ttl

    def get(self, key: str) -> Optional[Any]:
        raw = self._client.get(key)
        if raw is None:
            return None
        try:
            return json.loads(raw)
        except Exception:
            return raw

    def set(self, key: str, value: Any, ttl: int) -> None:
        ttl = ttl if ttl and ttl > 0 else self._default_ttl
        self._client.setex(key, ttl, json.dumps(value, ensure_ascii=False, default=str))

    def delete(self, key: str) -> None:
        self._client.delete(key)

    def clear(self) -> int:
        keys = self._client.keys("mox:*")
        if keys:
            return self._client.delete(*keys)
        return 0

    def stats(self) -> dict:
        info = self._client.info("memory") if self._client.ping() else {}
        return {
            "driver": self.name,
            "used_memory": info.get("used_memory", 0),
            "default_ttl": self._default_ttl,
        }


def build_cache(driver: str = "memory", **kwargs) -> CacheAdapter:
    """工厂：按配置创建缓存适配器。CACHE_DRIVER 支持 memory / redis。"""
    driver = (driver or "memory").lower()
    if driver == "redis":
        try:
            return RedisCache(url=kwargs.get("url", "redis://127.0.0.1:6379/0"),
                              default_ttl=kwargs.get("default_ttl", 60))
        except Exception:
            # redis 不可用时回退内存缓存，保证服务不中断
            return MemoryCache(default_ttl=kwargs.get("default_ttl", 60))
    return MemoryCache(capacity=kwargs.get("capacity", 20_000),
                       default_ttl=kwargs.get("default_ttl", 60))


def cache_key(namespace: str, parts: dict) -> str:
    """统一缓存键生成：namespace + 规范化参数哈希 + 权限维度。"""
    canonical = json.dumps(parts, ensure_ascii=False, sort_keys=True, default=str)
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:24]
    return f"mox:{namespace}:{digest}"
