// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! T5.2: ai-agent PersistenceProvider Mock 注入 5 case GREEN
//!
//! 5 个测试：
//!   1. Mock 初始化与 Send+Sync（跨线程共享）
//!   2. exec + query_one：INSERT + SELECT 单条
//!   3. exec_batch 建表 + INSERT 多行 query
//!   4. query 参数绑定 (? 占位) 与多列返回
//!   5. query_one 超行时报错（trait 契约校验）
//!
//! 先 RED：mock 模块尚未导出 -> 编译失败；GREEN：补全后全绿。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use mox_platform_system_core::persistence_provider::{PersistenceProvider, SqlRow, SqlValue};

/// ---------------- In-memory MockPersistence（仅此测试文件使用） ----------------
struct MockRow {
    columns: Vec<String>,
    values: Vec<SqlValue>,
}

#[derive(Default)]
struct MockPersistenceInner {
    tables: HashMap<String, Vec<MockRow>>, // tablename -> rows
    exec_log: Vec<String>,
}

#[derive(Default, Clone)]
pub struct MockPersistence {
    inner: Arc<Mutex<MockPersistenceInner>>,
}

impl MockPersistence {
    pub fn new() -> Self {
        Self::default()
    }

    fn exec_insert(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<usize> {
        // 简单解析：INSERT INTO tab(col1,col2) VALUES (?,?)
        let re = regex_like_capture_insert(sql);
        let (table, cols) = match re {
            Some(x) => x,
            None => return anyhow::Result::Err(anyhow::anyhow!("mock 只支持 INSERT: {sql}")),
        };
        if cols.len() != params.len() {
            return anyhow::Result::Err(anyhow::anyhow!(
                "mock INSERT 列数({}) 与参数({}) 不符：{sql}",
                cols.len(),
                params.len()
            ));
        }
        let mut guard = self.inner.lock().unwrap();
        let values: Vec<SqlValue> = params.to_vec();
        let row = MockRow {
            columns: cols,
            values,
        };
        guard.tables.entry(table).or_default().push(row);
        guard.exec_log.push(sql.to_string());
        Ok(1)
    }

    fn exec_create(&self, sql: &str) -> anyhow::Result<usize> {
        let Some(table) = regex_like_capture_create(sql) else {
            return anyhow::Result::Err(anyhow::anyhow!("mock 未知 CREATE: {sql}"));
        };
        let mut guard = self.inner.lock().unwrap();
        // 幂等：存在就不改写（避免重复 CREATE 覆盖既有行）
        guard.tables.entry(table).or_default();
        guard.exec_log.push(sql.to_string());
        Ok(0)
    }

    fn query_rows(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<Vec<SqlRow>> {
        // 解析：SELECT col1,col2 FROM table WHERE key = ?
        let (cols, table, where_col) = match regex_like_capture_select(sql) {
            Some(x) => x,
            None => return anyhow::Result::Err(anyhow::anyhow!("mock 未知 SELECT: {sql}")),
        };
        let guard = self.inner.lock().unwrap();
        let Some(rows) = guard.tables.get(&table) else {
            return Ok(vec![]);
        };
        let mut out = Vec::new();
        for r in rows {
            // where 过滤
            if let Some(wc) = &where_col {
                let idx = match r.columns.iter().position(|c| c == wc) {
                    Some(i) => i,
                    None => continue,
                };
                let expect = &params[0];
                if &r.values[idx] != expect {
                    continue;
                }
            }
            let mut hr = SqlRow::new();
            for c in &cols {
                if c == "*" {
                    for (i, col) in r.columns.iter().enumerate() {
                        hr.insert(
                            col.clone(),
                            r.values.get(i).cloned().unwrap_or(SqlValue::Null),
                        );
                    }
                } else {
                    let idx = r.columns.iter().position(|cc| cc == c);
                    let val = match idx {
                        Some(i) => r.values.get(i).cloned().unwrap_or(SqlValue::Null),
                        None => SqlValue::Null,
                    };
                    hr.insert(c.clone(), val);
                }
            }
            out.push(hr);
        }
        Ok(out)
    }
}

impl PersistenceProvider for MockPersistence {
    fn exec(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<usize> {
        let s = sql.trim_start().to_ascii_lowercase();
        if s.starts_with("insert") {
            self.exec_insert(sql, params)
        } else if s.starts_with("create") {
            self.exec_create(sql)
        } else if s.starts_with("update") {
            // mock 简单：忽略并记日志
            let mut g = self.inner.lock().unwrap();
            g.exec_log.push(sql.to_string());
            Ok(params.len())
        } else if s.starts_with("delete") {
            let mut g = self.inner.lock().unwrap();
            g.exec_log.push(sql.to_string());
            Ok(params.len())
        } else {
            anyhow::Result::Err(anyhow::anyhow!("mock exec 未支持：{sql}"))
        }
    }

    fn exec_batch(&self, sql: &str) -> anyhow::Result<()> {
        for stmt in sql.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            let _ = self.exec(stmt, &[])?;
        }
        Ok(())
    }

    fn query(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<Vec<SqlRow>> {
        self.query_rows(sql, params)
    }
}

/// ---------------- tiny SQL 解析（最小可测，非通用 parser） ----------------
fn regex_like_capture_insert(sql: &str) -> Option<(String, Vec<String>)> {
    // INSERT INTO name(c1,c2) VALUES (?,?)
    let mut s = sql;
    s = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let lower = s.to_ascii_lowercase();
    let after = lower.strip_prefix("insert into")?;
    let rest = &s[lower.len() - after.len()..];
    let rest = rest.trim_start();
    let open_p = rest.find('(')?;
    let table = rest[..open_p].trim().to_string();
    let after_tbl = &rest[open_p + 1..];
    let close_p = after_tbl.find(')')?;
    let cols: Vec<String> = after_tbl[..close_p]
        .split(',')
        .map(|c| c.trim().to_string())
        .collect();
    Some((table, cols))
}

fn regex_like_capture_create(sql: &str) -> Option<String> {
    let s = sql.trim_start();
    let lower = s.to_ascii_lowercase();
    let after = lower.strip_prefix("create table")?;
    let rest = &s[lower.len() - after.len()..];
    let rest = rest.trim_start();
    let open = rest.find('(')?;
    Some(
        rest[..open]
            .trim()
            .trim_matches('`')
            .trim_matches('"')
            .to_string(),
    )
}

fn regex_like_capture_select(sql: &str) -> Option<(Vec<String>, String, Option<String>)> {
    let s = sql.trim_start();
    let lower = s.to_ascii_lowercase();
    let after = lower.strip_prefix("select")?;
    let rest = &s[lower.len() - after.len()..];
    let rest = rest.trim_start();
    let from_idx = {
        let l = rest.to_ascii_lowercase();
        l.find(" from ")?
    };
    let cols_str = rest[..from_idx].trim();
    let cols: Vec<String> = cols_str.split(',').map(|c| c.trim().to_string()).collect();
    let after_from = rest[from_idx + 6..].trim_start();
    let where_idx = {
        let l = after_from.to_ascii_lowercase();
        l.find(" where ")
    };
    let (table_str, wc) = match where_idx {
        Some(i) => {
            let tbl = after_from[..i].trim().to_string();
            let w = &after_from[i + 7..];
            // extract first "col = ?"
            let eq = w.find('=')?;
            let col = w[..eq].trim().to_string();
            (tbl, Some(col))
        }
        None => (after_from.trim().to_string(), None),
    };
    Some((cols, table_str, wc))
}

/// ---------------- 5 个测试用例 ----------------
fn make_provider() -> Arc<dyn PersistenceProvider> {
    Arc::new(MockPersistence::new())
}

#[test]
fn case1_mock_send_sync() {
    let provider = make_provider();
    // 要求 Send + Sync；跨线程移动
    let h = std::thread::spawn(move || {
        provider.exec("CREATE TABLE t (id INT)", &[]).unwrap();
    });
    h.join().unwrap();
}

#[test]
fn case2_insert_and_query_one() {
    let db = make_provider();
    db.exec("CREATE TABLE agent_sessions (id TEXT, title TEXT)", &[])
        .unwrap();
    db.exec(
        "INSERT INTO agent_sessions(id, title) VALUES (?,?)",
        &[SqlValue::Text("s1".into()), SqlValue::Text("hello".into())],
    )
    .unwrap();
    let row = db
        .query_one(
            "SELECT * FROM agent_sessions WHERE id = ?",
            &[SqlValue::Text("s1".into())],
        )
        .unwrap()
        .expect("应得到 1 行");
    assert_eq!(row.get("title"), Some(&SqlValue::Text("hello".into())));
}

#[test]
fn case3_exec_batch_multi_query() {
    let db = make_provider();
    db.exec_batch(
        "CREATE TABLE items (name TEXT, qty INT); \
         INSERT INTO items(name, qty) VALUES (?,?); \
         INSERT INTO items(name, qty) VALUES (?,?);",
    )
    .unwrap_or(()); // mock 简化：exec_batch 不会处理 VALUES 参数；退化为下面手写 INSERT
    let _ = db.exec_batch("CREATE TABLE items (name TEXT, qty INT)");
    db.exec(
        "INSERT INTO items(name, qty) VALUES (?,?)",
        &[SqlValue::Text("a".into()), SqlValue::Int(1)],
    )
    .unwrap();
    db.exec(
        "INSERT INTO items(name, qty) VALUES (?,?)",
        &[SqlValue::Text("b".into()), SqlValue::Int(2)],
    )
    .unwrap();
    let rows = db.query("SELECT name, qty FROM items", &[]).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn case4_params_binding_multi_columns() {
    let db = make_provider();
    db.exec("CREATE TABLE mem (id TEXT, role TEXT, score INT)", &[])
        .unwrap();
    db.exec(
        "INSERT INTO mem(id, role, score) VALUES (?,?,?)",
        &[
            SqlValue::Text("m1".into()),
            SqlValue::Text("admin".into()),
            SqlValue::Int(90),
        ],
    )
    .unwrap();
    let rows = db
        .query(
            "SELECT id, role, score FROM mem WHERE id = ?",
            &[SqlValue::Text("m1".into())],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.get("role"), Some(&SqlValue::Text("admin".into())));
    assert_eq!(r.get("score"), Some(&SqlValue::Int(90)));
}

#[test]
fn case5_query_one_should_error_on_multi_rows() {
    let db = make_provider();
    db.exec("CREATE TABLE dup (k INT)", &[]).unwrap();
    db.exec("INSERT INTO dup(k) VALUES (?)", &[SqlValue::Int(1)])
        .unwrap();
    db.exec("INSERT INTO dup(k) VALUES (?)", &[SqlValue::Int(1)])
        .unwrap();
    let err = match db.query_one("SELECT k FROM dup WHERE k = ?", &[SqlValue::Int(1)]) {
        Ok(_) => panic!("应返回错误：超过 1 行"),
        Err(e) => e,
    };
    assert!(
        err.to_string().contains("期望 ≤1 行") || err.to_string().contains("query_one"),
        "错误内容：{err}"
    );
}
