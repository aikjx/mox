//! T5.3: xuanji-system PersistenceProvider 集成测试：5 模型 × 4 CRUD = 20 条 GREEN
//!
//! 模型：Member / Task / Document / Resource / Notification
//!     - Document, Resource 是 T5 新加入的轻量领域模型（不改动算法与路由语义）。
//! 操作：save / get / update / delete
//!     提供方：SqlitePersistence::memory()（xuanji-system 独占 rusqlite）

use std::sync::Arc;
use xuanji_system::persistence_provider::{PersistenceProvider, SqlRow, SqlValue};
use xuanji_system::sqlite_provider::SqlitePersistence;

// ---- 小工具：把 SqlRow 中的 TEXT / INT 读出来 ----
fn text(row: &SqlRow, col: &str) -> String {
    match row.get(col) {
        Some(SqlValue::Text(s)) => s.clone(),
        Some(SqlValue::Null) => String::new(),
        _ => "".into(),
    }
}
fn int(row: &SqlRow, col: &str) -> i64 {
    match row.get(col) {
        Some(SqlValue::Int(i)) => *i,
        _ => 0,
    }
}

fn new_provider() -> Arc<dyn PersistenceProvider> {
    Arc::new(SqlitePersistence::memory().expect("SqlitePersistence::memory 应成功"))
}

fn init_schema(db: &dyn PersistenceProvider) {
    // 5 张表，纯文本/INT，避免复杂类型
    let sql = r#"
CREATE TABLE IF NOT EXISTS members (
    id TEXT PRIMARY KEY,
    xuanji_id TEXT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    status TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    xuanji_id TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INT NOT NULL
);
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    version INT NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS resources (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    location TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    member_id TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    read_flag INT NOT NULL DEFAULT 0
);
"#;
    db.exec_batch(sql).expect("建表 5 张应成功");
}

// ========== 20 CRUD 用例 ==========

// ---------- Member (4) ----------
#[test]
fn crud_member_1_save() {
    let db = new_provider();
    init_schema(db.as_ref());
    let n = db.exec(
        "INSERT INTO members(id, xuanji_id, name, email, status) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("m1".into()),
            SqlValue::Text("xj1".into()),
            SqlValue::Text("Alice".into()),
            SqlValue::Text("a@x".into()),
            SqlValue::Text("Active".into()),
        ],
    ).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn crud_member_2_get() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO members(id, xuanji_id, name, email, status) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("m1".into()),
            SqlValue::Text("xj1".into()),
            SqlValue::Text("Alice".into()),
            SqlValue::Text("a@x".into()),
            SqlValue::Text("Active".into()),
        ],
    ).unwrap();
    let row = db.query_one("SELECT id, xuanji_id, name, email, status FROM members WHERE id = ?",
        &[SqlValue::Text("m1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "name"), "Alice");
    assert_eq!(text(&row, "status"), "Active");
}

#[test]
fn crud_member_3_update() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO members(id, xuanji_id, name, email, status) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("m1".into()),
            SqlValue::Text("xj1".into()),
            SqlValue::Text("Alice".into()),
            SqlValue::Text("a@x".into()),
            SqlValue::Text("Active".into()),
        ],
    ).unwrap();
    let n = db.exec("UPDATE members SET status = ?, email = ? WHERE id = ?",
        &[SqlValue::Text("Suspended".into()), SqlValue::Text("a2@x".into()), SqlValue::Text("m1".into())]
    ).unwrap();
    assert_eq!(n, 1);
    let row = db.query_one("SELECT status, email FROM members WHERE id = ?",
        &[SqlValue::Text("m1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "status"), "Suspended");
    assert_eq!(text(&row, "email"), "a2@x");
}

#[test]
fn crud_member_4_delete() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO members(id, xuanji_id, name, email, status) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("m1".into()),
            SqlValue::Text("xj1".into()),
            SqlValue::Text("Alice".into()),
            SqlValue::Text("a@x".into()),
            SqlValue::Text("Active".into()),
        ],
    ).unwrap();
    let n = db.exec("DELETE FROM members WHERE id = ?",
        &[SqlValue::Text("m1".into())]).unwrap();
    assert_eq!(n, 1);
    let rows = db.query("SELECT id FROM members WHERE id = ?",
        &[SqlValue::Text("m1".into())]).unwrap();
    assert!(rows.is_empty());
}

// ---------- Task (4) ----------
#[test]
fn crud_task_1_save() {
    let db = new_provider();
    init_schema(db.as_ref());
    let n = db.exec(
        "INSERT INTO tasks(id, xuanji_id, title, status, priority) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("t1".into()), SqlValue::Text("xj1".into()),
            SqlValue::Text("Build spec".into()), SqlValue::Text("Draft".into()), SqlValue::Int(2),
        ],
    ).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn crud_task_2_get() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO tasks(id, xuanji_id, title, status, priority) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("t1".into()), SqlValue::Text("xj1".into()),
            SqlValue::Text("Build spec".into()), SqlValue::Text("Draft".into()), SqlValue::Int(2),
        ],
    ).unwrap();
    let row = db.query_one("SELECT * FROM tasks WHERE id = ?",
        &[SqlValue::Text("t1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "title"), "Build spec");
    assert_eq!(int(&row, "priority"), 2);
}

#[test]
fn crud_task_3_update() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO tasks(id, xuanji_id, title, status, priority) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("t1".into()), SqlValue::Text("xj1".into()),
            SqlValue::Text("Build spec".into()), SqlValue::Text("Draft".into()), SqlValue::Int(2),
        ],
    ).unwrap();
    let n = db.exec("UPDATE tasks SET status = ?, priority = ? WHERE id = ?",
        &[SqlValue::Text("InProgress".into()), SqlValue::Int(3), SqlValue::Text("t1".into())]).unwrap();
    assert_eq!(n, 1);
    let row = db.query_one("SELECT status, priority FROM tasks WHERE id = ?",
        &[SqlValue::Text("t1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "status"), "InProgress");
    assert_eq!(int(&row, "priority"), 3);
}

#[test]
fn crud_task_4_delete() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO tasks(id, xuanji_id, title, status, priority) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("t1".into()), SqlValue::Text("xj1".into()),
            SqlValue::Text("Build spec".into()), SqlValue::Text("Draft".into()), SqlValue::Int(2),
        ],
    ).unwrap();
    let n = db.exec("DELETE FROM tasks WHERE id = ?", &[SqlValue::Text("t1".into())]).unwrap();
    assert_eq!(n, 1);
    let rows = db.query("SELECT id FROM tasks WHERE id = ?",
        &[SqlValue::Text("t1".into())]).unwrap();
    assert!(rows.is_empty());
}

// ---------- Document (4) ----------
#[test]
fn crud_document_1_save() {
    let db = new_provider();
    init_schema(db.as_ref());
    let n = db.exec(
        "INSERT INTO documents(id, owner_id, title, body, version) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("d1".into()), SqlValue::Text("u1".into()),
            SqlValue::Text("README".into()), SqlValue::Text("body here".into()), SqlValue::Int(1),
        ],
    ).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn crud_document_2_get() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO documents(id, owner_id, title, body, version) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("d1".into()), SqlValue::Text("u1".into()),
            SqlValue::Text("README".into()), SqlValue::Text("body here".into()), SqlValue::Int(1),
        ],
    ).unwrap();
    let row = db.query_one("SELECT title, body, version FROM documents WHERE id = ?",
        &[SqlValue::Text("d1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "title"), "README");
    assert_eq!(int(&row, "version"), 1);
    assert_eq!(text(&row, "body"), "body here");
}

#[test]
fn crud_document_3_update() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO documents(id, owner_id, title, body, version) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("d1".into()), SqlValue::Text("u1".into()),
            SqlValue::Text("README".into()), SqlValue::Text("body here".into()), SqlValue::Int(1),
        ],
    ).unwrap();
    let n = db.exec("UPDATE documents SET body = ?, version = ? WHERE id = ?",
        &[SqlValue::Text("body v2".into()), SqlValue::Int(2), SqlValue::Text("d1".into())]
    ).unwrap();
    assert_eq!(n, 1);
    let row = db.query_one("SELECT body, version FROM documents WHERE id = ?",
        &[SqlValue::Text("d1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "body"), "body v2");
    assert_eq!(int(&row, "version"), 2);
}

#[test]
fn crud_document_4_delete() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO documents(id, owner_id, title, body, version) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("d1".into()), SqlValue::Text("u1".into()),
            SqlValue::Text("README".into()), SqlValue::Text("body here".into()), SqlValue::Int(1),
        ],
    ).unwrap();
    let n = db.exec("DELETE FROM documents WHERE id = ?", &[SqlValue::Text("d1".into())]).unwrap();
    assert_eq!(n, 1);
    let rows = db.query("SELECT id FROM documents WHERE id = ?",
        &[SqlValue::Text("d1".into())]).unwrap();
    assert!(rows.is_empty());
}

// ---------- Resource (4) ----------
#[test]
fn crud_resource_1_save() {
    let db = new_provider();
    init_schema(db.as_ref());
    let n = db.exec(
        "INSERT INTO resources(id, kind, name, location) VALUES (?,?,?,?)",
        &[
            SqlValue::Text("r1".into()), SqlValue::Text("file".into()),
            SqlValue::Text("logo.png".into()), SqlValue::Text("s3://x/a.png".into()),
        ],
    ).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn crud_resource_2_get() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO resources(id, kind, name, location) VALUES (?,?,?,?)",
        &[
            SqlValue::Text("r1".into()), SqlValue::Text("file".into()),
            SqlValue::Text("logo.png".into()), SqlValue::Text("s3://x/a.png".into()),
        ],
    ).unwrap();
    let row = db.query_one("SELECT kind, name, location FROM resources WHERE id = ?",
        &[SqlValue::Text("r1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "kind"), "file");
    assert_eq!(text(&row, "name"), "logo.png");
}

#[test]
fn crud_resource_3_update() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO resources(id, kind, name, location) VALUES (?,?,?,?)",
        &[
            SqlValue::Text("r1".into()), SqlValue::Text("file".into()),
            SqlValue::Text("logo.png".into()), SqlValue::Text("s3://x/a.png".into()),
        ],
    ).unwrap();
    let n = db.exec("UPDATE resources SET location = ? WHERE id = ?",
        &[SqlValue::Text("s3://x/b.png".into()), SqlValue::Text("r1".into())]
    ).unwrap();
    assert_eq!(n, 1);
    let row = db.query_one("SELECT location FROM resources WHERE id = ?",
        &[SqlValue::Text("r1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "location"), "s3://x/b.png");
}

#[test]
fn crud_resource_4_delete() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO resources(id, kind, name, location) VALUES (?,?,?,?)",
        &[
            SqlValue::Text("r1".into()), SqlValue::Text("file".into()),
            SqlValue::Text("logo.png".into()), SqlValue::Text("s3://x/a.png".into()),
        ],
    ).unwrap();
    let n = db.exec("DELETE FROM resources WHERE id = ?", &[SqlValue::Text("r1".into())]).unwrap();
    assert_eq!(n, 1);
    assert!(db.query("SELECT id FROM resources WHERE id = ?",
        &[SqlValue::Text("r1".into())]).unwrap().is_empty());
}

// ---------- Notification (4) ----------
#[test]
fn crud_notification_1_save() {
    let db = new_provider();
    init_schema(db.as_ref());
    let n = db.exec(
        "INSERT INTO notifications(id, member_id, title, body, read_flag) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("n1".into()), SqlValue::Text("m1".into()),
            SqlValue::Text("Hi".into()), SqlValue::Text("Welcome".into()), SqlValue::Int(0),
        ],
    ).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn crud_notification_2_get() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO notifications(id, member_id, title, body, read_flag) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("n1".into()), SqlValue::Text("m1".into()),
            SqlValue::Text("Hi".into()), SqlValue::Text("Welcome".into()), SqlValue::Int(0),
        ],
    ).unwrap();
    let row = db.query_one("SELECT member_id, title, read_flag FROM notifications WHERE id = ?",
        &[SqlValue::Text("n1".into())]).unwrap().unwrap();
    assert_eq!(text(&row, "title"), "Hi");
    assert_eq!(int(&row, "read_flag"), 0);
}

#[test]
fn crud_notification_3_update() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO notifications(id, member_id, title, body, read_flag) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("n1".into()), SqlValue::Text("m1".into()),
            SqlValue::Text("Hi".into()), SqlValue::Text("Welcome".into()), SqlValue::Int(0),
        ],
    ).unwrap();
    let n = db.exec("UPDATE notifications SET read_flag = ?, body = ? WHERE id = ?",
        &[SqlValue::Int(1), SqlValue::Text("Welcome!".into()), SqlValue::Text("n1".into())]).unwrap();
    assert_eq!(n, 1);
    let row = db.query_one("SELECT read_flag, body FROM notifications WHERE id = ?",
        &[SqlValue::Text("n1".into())]).unwrap().unwrap();
    assert_eq!(int(&row, "read_flag"), 1);
    assert_eq!(text(&row, "body"), "Welcome!");
}

#[test]
fn crud_notification_4_delete() {
    let db = new_provider();
    init_schema(db.as_ref());
    db.exec(
        "INSERT INTO notifications(id, member_id, title, body, read_flag) VALUES (?,?,?,?,?)",
        &[
            SqlValue::Text("n1".into()), SqlValue::Text("m1".into()),
            SqlValue::Text("Hi".into()), SqlValue::Text("Welcome".into()), SqlValue::Int(0),
        ],
    ).unwrap();
    let n = db.exec("DELETE FROM notifications WHERE id = ?", &[SqlValue::Text("n1".into())]).unwrap();
    assert_eq!(n, 1);
    assert!(db.query("SELECT id FROM notifications WHERE id = ?",
        &[SqlValue::Text("n1".into())]).unwrap().is_empty());
}
