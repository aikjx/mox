-- =========================================================================
-- MOX v2.0 · Step1 · 知识图谱 L3.5 中枢 graph_edges DDL 脚本
-- 对应：《归一化总纲》§5.6 Step1（1h） / §5.2 Schema + §5.5 治理红线 3
--
-- 执行前必读：
--   1) 先备份 ous.db（SQLite）或 pg_dump（PostgreSQL）；命令见下方"备份校验"段
--   2) 本脚本幂等：全部使用 IF NOT EXISTS，重复执行无副作用
--   3) 双方言兼容：SQLite 3.37+ / PostgreSQL 13+（自动跳过不适用段）
-- =========================================================================

-- =========================================================================
-- 【方言 A · SQLite】—— 生产默认引擎（单机/开发环境）
-- 适用: platform/backend-node/data/ous.db
-- 执行: sqlite3 data/ous.db < deploy/sql/mox-step1-graph-edges.sql
-- =========================================================================
-- 1. 关系主表（UNIQUE(src,rel,dst) 支撑 upsert；tombstone=红线3 只墓碑不物理删；reason=7 年审计）
CREATE TABLE IF NOT EXISTS graph_edges (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  src         TEXT    NOT NULL,
  rel         TEXT    NOT NULL,
  dst         TEXT    NOT NULL,
  props       TEXT,                   -- JSON 序列化的关系属性
  tombstone   INTEGER DEFAULT 0,      -- 红线 3：永远只 tombstone，不物理删
  reason      TEXT,                   -- 7 年审计：谁/为何/何时删的
  created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
  UNIQUE(src, rel, dst)
);
-- 2. 四索引（覆盖 §5.4 的 6 接口邻域/路径/PageRank 全部查询模式）
CREATE INDEX IF NOT EXISTS idx_edges_src ON graph_edges(src);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON graph_edges(dst);
CREATE INDEX IF NOT EXISTS idx_edges_rel ON graph_edges(rel);
CREATE INDEX IF NOT EXISTS idx_edges_alive ON graph_edges(tombstone);
-- 3. 自检行数（DBA 确认 DDL 落地 + 备份校验对比基准）
SELECT 'SQLite graph_edges DDL 落地 OK. 当前行数=' || COUNT(*) AS sanity_check
  FROM (SELECT 1 FROM graph_edges LIMIT 1);

-- =========================================================================
-- 【方言 B · PostgreSQL 13+】—— T2 分库分存/上云阶段启用
-- 适用: AWS RDS / 阿里云 RDS / 自建 PG / YugabyteDB
-- 执行: psql $PG_DSN -f deploy/sql/mox-step1-graph-edges.sql
-- =========================================================================
-- PostgreSQL 段仅在连接到 PG 时有效；SQLite 环境会因语法差异跳过（安全）
DO $$
BEGIN
  IF current_database() IS NOT NULL AND version() LIKE '%PostgreSQL%' THEN
    -- 1. 关系主表
    CREATE TABLE IF NOT EXISTS graph_edges (
      id          BIGSERIAL PRIMARY KEY,
      src         TEXT    NOT NULL,
      rel         TEXT    NOT NULL,
      dst         TEXT    NOT NULL,
      props       JSONB,
      tombstone   SMALLINT DEFAULT 0 CHECK (tombstone IN (0,1)),
      reason      TEXT,
      created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
      UNIQUE(src, rel, dst)
    );
    -- 2. 四索引
    CREATE INDEX IF NOT EXISTS idx_edges_src ON graph_edges(src);
    CREATE INDEX IF NOT EXISTS idx_edges_dst ON graph_edges(dst);
    CREATE INDEX IF NOT EXISTS idx_edges_rel ON graph_edges(rel);
    CREATE INDEX IF NOT EXISTS idx_edges_alive ON graph_edges(tombstone);
    -- 3. 注释对齐归一化总纲
    COMMENT ON TABLE  graph_edges              IS 'MOX L3.5 知识图谱中枢 · 关系表（§5.2 Schema · 40 种 rel）';
    COMMENT ON COLUMN graph_edges.tombstone    IS '治理红线 3：永远只墓碑不物理删（0=活 1=死）';
    COMMENT ON COLUMN graph_edges.reason       IS '7 年审计链：删除原因 + 操作人身份指纹';
    RAISE NOTICE 'PostgreSQL graph_edges DDL 落地 OK';
  END IF;
END $$;

-- =========================================================================
-- 【备份校验命令】—— DDL 执行前后跑一遍，保证行数+结构一致（CR-003 合规）
-- =========================================================================
--
-- ┌───────── SQLite ───────────────────────────────────────────────────────┐
-- │ # 备份（先备份再 DDL）                                                  │
-- │   cp data/ous.db data/ous.db.bak.$(date +%Y%m%d-%H%M%S)               │
-- │   md5sum data/ous.db data/ous.db.bak.* > data/ous.db.md5sum            │
-- │ # DDL 执行                                                              │
-- │   sqlite3 data/ous.db < deploy/sql/mox-step1-graph-edges.sql 2>&1      │
-- │ # 校验（DDL 后行数 0 或 原备份一致）                                    │
-- │   sqlite3 data/ous.db "SELECT COUNT(*) FROM graph_edges;"              │
-- │   sqlite3 data/ous.db ".indices graph_edges" | grep -E "idx_edges_.*"  │
-- └─────────────────────────────────────────────────────────────────────────┘
--
-- ┌───────── PostgreSQL ───────────────────────────────────────────────────┐
-- │ # 备份（先备份再 DDL）                                                  │
-- │   pg_dump -Fc $PG_DSN -f deploy/sql/pg-bak-$(date +%Y%m%d-%H%M%S).dump │
-- │ # DDL 执行                                                              │
-- │   psql $PG_DSN -f deploy/sql/mox-step1-graph-edges.sql                 │
-- │ # 校验                                                                  │
-- │   psql $PG_DSN -c "\d graph_edges; \di idx_edges_*;"                   │
-- └─────────────────────────────────────────────────────────────────────────┘
--
-- 异常处理 Runbook 引用：见 B-01 §4.3 / B-02 §3.6 的 F5 / F7 条目；
-- 若备份失败或 DDL 执行后索引缺失 < 4 个：立即 STOP，禁止进入 Step2，
-- 走 Runbook 回滚到备份（SQLite 直接 cp；PostgreSQL pg_restore -c）。
