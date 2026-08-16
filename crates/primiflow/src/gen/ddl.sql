-- 由关联图谱自动生成 · primiflow/SPEC.md §4
-- 执行前请确保已启用 pgvector 扩展: CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE projects (
  id UUID PRIMARY KEY,
  name TEXT,
  tenant_id TEXT,
  k_t_pref TEXT,
  budget_c REAL,
  created_at TIMESTAMPTZ
);

CREATE TABLE conversations (
  id UUID PRIMARY KEY,
  project_id UUID,
  role TEXT,
  content TEXT,
  meta TEXT,
  created_at TIMESTAMPTZ
);

CREATE TABLE topologys (
  id UUID PRIMARY KEY,
  project_id UUID,
  status TEXT,
  k REAL,
  t REAL,
  c REAL,
  residual_delta REAL,
  graph_json TEXT,
  created_at TIMESTAMPTZ
);

CREATE TABLE assets (
  id UUID PRIMARY KEY,
  topology_id UUID,
  name TEXT,
  domain TEXT,
  graph_json TEXT,
  frozen_at TIMESTAMPTZ
);

CREATE TABLE artifacts (
  id UUID PRIMARY KEY,
  project_id UUID,
  kind TEXT,
  title TEXT,
  content TEXT,
  created_at TIMESTAMPTZ
);

CREATE TABLE trace_links (
  id UUID PRIMARY KEY,
  project_id UUID,
  requirement_id TEXT,
  feature_id TEXT,
  business_id TEXT,
  algorithm_id TEXT,
  task_id TEXT,
  code_id TEXT
);

