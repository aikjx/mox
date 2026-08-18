-- 由关联图谱自动生成 · primiflow/SPEC.md §4
-- 执行前请确保已启用 pgvector 扩展: CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE data_r1s (
  id UUID PRIMARY KEY,
  project_id UUID,
  graph_json TEXT,
  created_at TIMESTAMPTZ
);

CREATE TABLE data_r2s (
  id UUID PRIMARY KEY,
  project_id UUID,
  graph_json TEXT,
  created_at TIMESTAMPTZ
);

CREATE TABLE data_r3s (
  id UUID PRIMARY KEY,
  project_id UUID,
  graph_json TEXT,
  created_at TIMESTAMPTZ
);

CREATE TABLE data_r4s (
  id UUID PRIMARY KEY,
  project_id UUID,
  graph_json TEXT,
  created_at TIMESTAMPTZ
);

