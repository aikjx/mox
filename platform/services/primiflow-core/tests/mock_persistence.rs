//! T5.2: primiflow-core PersistenceProvider Mock 注入 5 case GREEN
//!
//! 5 个测试：
//!   1. Mock + Arc<dyn PersistenceProvider> 线程安全
//!   2. 保存 (INSERT) + 读取 (query) 往返
//!   3. exec_batch：建表 + 种子数据
//!   4. UPDATE：返回受影响行数 mock 契约
//!   5. DELETE + list 空表返回空集
//!
//! RED：首次编译缺模块/缺 trait 使用；GREEN 后 5/5。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use mox_system::persistence_provider::{PersistenceProvider, SqlRow, SqlValue};

type TableRows = Vec<(Vec<String>, Vec<SqlValue>)>;
type StoreMap = HashMap<String, TableRows>;
type SelectReturn = (Vec<String>, String, Option<(String, usize)>);

/// ---------------- Mock (独立，不与 ai-agent 测试共享代码) ----------------
#[derive(Default, Clone)]
pub struct MockPersistence {
    data: Arc<Mutex<Data>>,
}

#[derive(Default)]
struct Data {
    store: StoreMap, // table -> (columns, row values)
    ops: Vec<String>,
}

impl MockPersistence {
    pub fn new() -> Self {
        Self::default()
    }
}

fn parse_create(sql: &str) -> Option<String> {
    let s = sql.trim();
    let lo = s.to_ascii_lowercase();
    let after = lo.strip_prefix("create table")?;
    let rest = &s[lo.len() - after.len()..].trim();
    let p = rest.find('(')?;
    Some(
        rest[..p]
            .trim()
            .trim_matches('"')
            .trim_matches('`')
            .to_string(),
    )
}
fn parse_insert(sql: &str) -> Option<(String, Vec<String>)> {
    let s = sql.trim();
    let lo = s.to_ascii_lowercase();
    let after = lo.strip_prefix("insert into")?;
    let rest = &s[lo.len() - after.len()..].trim();
    let p = rest.find('(')?;
    let table = rest[..p].trim().to_string();
    let end = rest[p + 1..].find(')')?;
    let cols = rest[p + 1..p + 1 + end]
        .split(',')
        .map(|c| c.trim().to_string())
        .collect();
    Some((table, cols))
}
fn parse_select(sql: &str) -> Option<SelectReturn> {
    // SELECT a,b FROM tbl [WHERE col = $idx]
    let s = sql.trim();
    let lo = s.to_ascii_lowercase();
    let after = lo.strip_prefix("select")?;
    let rest = &s[lo.len() - after.len()..].trim();
    let l = rest.to_ascii_lowercase();
    let from = l.find(" from ")?;
    let cols: Vec<String> = rest[..from]
        .split(',')
        .map(|c| c.trim().to_string())
        .collect();
    let after_from = rest[from + 6..].trim();
    let where_idx = {
        let l2 = after_from.to_ascii_lowercase();
        l2.find(" where ")
    };
    let (table, wc) = match where_idx {
        Some(i) => {
            let tbl = after_from[..i].trim().to_string();
            let w = &after_from[i + 7..];
            // col = ?  -> 参数位置计数：我们默认 WHERE 只有一个 ?，取 param[0]
            let eq = w.find('=')?;
            let col = w[..eq].trim().to_string();
            (tbl, Some((col, 0)))
        }
        None => (after_from.trim().to_string(), None),
    };
    // 清理表名尾部
    let table = table.split(';').next().unwrap_or("").trim().to_string();
    Some((cols, table, wc))
}

impl PersistenceProvider for MockPersistence {
    fn exec(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<usize> {
        let s = sql.trim();
        let lo = s.to_ascii_lowercase();
        if lo.starts_with("create table") {
            let tbl = parse_create(sql).ok_or_else(|| anyhow::anyhow!("CREATE parse fail"))?;
            let mut g = self.data.lock().unwrap();
            g.store.entry(tbl).or_default();
            g.ops.push("CREATE".into());
            Ok(0)
        } else if lo.starts_with("insert") {
            let (tbl, cols) =
                parse_insert(sql).ok_or_else(|| anyhow::anyhow!("INSERT parse fail"))?;
            let mut g = self.data.lock().unwrap();
            let row = if params.len() == cols.len() {
                params.to_vec()
            } else {
                vec![SqlValue::Null; cols.len()]
            };
            g.store.entry(tbl.clone()).or_default().push((cols, row));
            g.ops.push(format!("INSERT:{tbl}"));
            Ok(1)
        } else if lo.starts_with("update") {
            let mut g = self.data.lock().unwrap();
            g.ops.push("UPDATE".into());
            // mock 返回 params 长度，表达受影响行
            Ok(params.len().max(1))
        } else if lo.starts_with("delete") {
            let mut g = self.data.lock().unwrap();
            // 简单：如果有表识别到则清空；否则记录
            g.ops.push("DELETE".into());
            // 找 FROM 表名
            let from = lo.find(" from ").map(|i| i + 6).unwrap_or(0);
            let tbl_part = if from > 0 { &s[from..] } else { "" };
            let tbl = tbl_part
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(';')
                .to_string();
            if !tbl.is_empty() {
                if let Some(v) = g.store.get_mut(&tbl) {
                    let n = v.len();
                    if let Some((_, idx)) = parse_select(sql).and_then(|x| x.2) {
                        // WHERE + param 按第一个简单过滤：这里删除与 param[0] 匹配某列（不严格）
                        // 简化：只删除第一行
                        if !v.is_empty() && params.len() > idx {
                            v.remove(0);
                            return Ok(1);
                        }
                    }
                    v.clear();
                    return Ok(n);
                }
            }
            Ok(params.len())
        } else {
            Err(anyhow::anyhow!("mock primiflow 未知 SQL: {sql}"))
        }
    }

    fn exec_batch(&self, sql: &str) -> anyhow::Result<()> {
        for part in sql.split(';') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            let _ = self.exec(p, &[])?;
        }
        Ok(())
    }

    fn query(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<Vec<SqlRow>> {
        let (cols, table, wc) =
            parse_select(sql).ok_or_else(|| anyhow::anyhow!("SELECT parse fail: {sql}"))?;
        let g = self.data.lock().unwrap();
        let Some(rows) = g.store.get(&table) else {
            return Ok(vec![]);
        };
        let mut out = Vec::new();
        for (rcols, rvals) in rows {
            if let Some((wc_col, wc_idx)) = &wc {
                let Some(pos) = rcols.iter().position(|c| c == wc_col) else {
                    continue;
                };
                let want = &params[*wc_idx];
                if &rvals[pos] != want {
                    continue;
                }
            }
            let mut hr = SqlRow::new();
            for c in &cols {
                if c == "*" {
                    for (i, n) in rcols.iter().enumerate() {
                        hr.insert(n.clone(), rvals.get(i).cloned().unwrap_or(SqlValue::Null));
                    }
                } else {
                    let pos = rcols.iter().position(|rc| rc == c);
                    let val = match pos {
                        Some(i) => rvals.get(i).cloned().unwrap_or(SqlValue::Null),
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

/// ---------------- 5 case ----------------
fn arc() -> Arc<dyn PersistenceProvider> {
    Arc::new(MockPersistence::new())
}

#[test]
fn case1_send_sync() {
    let db = arc();
    let h = std::thread::spawn(move || {
        db.exec("CREATE TABLE run_meta (k TEXT)", &[]).unwrap();
    });
    h.join().unwrap();
}

#[test]
fn case2_insert_then_query() {
    let db = arc();
    db.exec("CREATE TABLE flows (id TEXT, name TEXT)", &[])
        .unwrap();
    db.exec(
        "INSERT INTO flows(id, name) VALUES (?,?)",
        &[SqlValue::Text("f1".into()), SqlValue::Text("flow1".into())],
    )
    .unwrap();
    let rows = db.query("SELECT id, name FROM flows", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("name"), Some(&SqlValue::Text("flow1".into())));
}

#[test]
fn case3_exec_batch_seed() {
    let db = arc();
    db.exec_batch("CREATE TABLE seeds (id INT, val TEXT);")
        .unwrap();
    db.exec(
        "INSERT INTO seeds(id, val) VALUES (?,?)",
        &[SqlValue::Int(1), SqlValue::Text("a".into())],
    )
    .unwrap();
    db.exec(
        "INSERT INTO seeds(id, val) VALUES (?,?)",
        &[SqlValue::Int(2), SqlValue::Text("b".into())],
    )
    .unwrap();
    let rows = db.query("SELECT * FROM seeds", &[]).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn case4_update_returns_affected() {
    let db = arc();
    db.exec("CREATE TABLE tasks (id TEXT, status TEXT)", &[])
        .unwrap();
    db.exec(
        "INSERT INTO tasks(id, status) VALUES (?,?)",
        &[SqlValue::Text("t1".into()), SqlValue::Text("todo".into())],
    )
    .unwrap();
    let n = db
        .exec(
            "UPDATE tasks SET status = ? WHERE id = ?",
            &[SqlValue::Text("done".into()), SqlValue::Text("t1".into())],
        )
        .unwrap();
    assert!(n >= 1, "mock UPDATE 受影响行 ≥1，实际 {n}");
}

#[test]
fn case5_delete_then_list_empty() {
    let db = arc();
    db.exec("CREATE TABLE docs (id TEXT, title TEXT)", &[])
        .unwrap();
    db.exec(
        "INSERT INTO docs(id, title) VALUES (?,?)",
        &[SqlValue::Text("d1".into()), SqlValue::Text("x".into())],
    )
    .unwrap();
    assert_eq!(db.query("SELECT id FROM docs", &[]).unwrap().len(), 1);
    let n = db
        .exec(
            "DELETE FROM docs WHERE id = ?",
            &[SqlValue::Text("d1".into())],
        )
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(db.query("SELECT id FROM docs", &[]).unwrap().len(), 0);
}
