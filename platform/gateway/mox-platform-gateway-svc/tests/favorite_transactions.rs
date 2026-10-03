use mox_platform_gateway_svc::alliance::favorite_repository::{
    read_on_connection, toggle_idempotent_on_connection, toggle_on_connection, FavoriteError,
};
use rusqlite::Connection;
use std::sync::{Arc, Barrier};

fn schema(conn: &Connection) {
    conn.execute_batch("CREATE TABLE experts(tenant_id TEXT,id TEXT,PRIMARY KEY(tenant_id,id));
        CREATE TABLE favorites(tenant_id TEXT,expert_id TEXT,created_at TEXT,PRIMARY KEY(tenant_id,expert_id));
        INSERT INTO experts VALUES('a','expert'),('b','expert');").unwrap();
}

#[test]
fn authoritative_batch_reads_survive_reopen_and_reject_foreign_objects() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("read.db");
    let mut conn = Connection::open(&path).unwrap();
    schema(&conn);
    toggle_on_connection(&mut conn, "b", "expert").unwrap();
    conn.execute("INSERT INTO experts VALUES('b','foreign')", []).unwrap();
    drop(conn);
    let mut conn = Connection::open(&path).unwrap();
    assert_eq!(
        read_on_connection(&mut conn, "a", &["expert".into()]).unwrap(),
        vec![("expert".into(), false)]
    );
    assert_eq!(
        read_on_connection(&mut conn, "b", &["expert".into()]).unwrap(),
        vec![("expert".into(), true)]
    );
    assert!(matches!(
        read_on_connection(&mut conn, "a", &["expert".into(), "foreign".into()]),
        Err(FavoriteError::NotFound)
    ));
    assert!(read_on_connection(&mut conn, "a", &[]).unwrap().is_empty());
    conn.execute_batch("DROP TABLE favorites").unwrap();
    assert!(matches!(
        read_on_connection(&mut conn, "a", &["expert".into()]),
        Err(FavoriteError::Storage(_))
    ));
}

#[test]
fn idempotent_retry_survives_reopen_and_does_not_reapply_old_result() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("receipts.db");
    let mut conn = Connection::open(&path).unwrap();
    schema(&conn);
    let first = toggle_idempotent_on_connection(&mut conn, "a", "user", "expert", "key-1").unwrap();
    assert!(first.favorite);
    assert!(!first.replayed);
    assert!(!toggle_on_connection(&mut conn, "a", "expert").unwrap());
    drop(conn);
    let mut conn = Connection::open(&path).unwrap();
    let retry = toggle_idempotent_on_connection(&mut conn, "a", "user", "expert", "key-1").unwrap();
    assert!(retry.favorite);
    assert!(!retry.current_favorite);
    assert!(retry.replayed);
    assert_eq!(retry.committed_at, first.committed_at);
    assert!(matches!(
        toggle_idempotent_on_connection(&mut conn, "a", "user", "different", "key-1"),
        Err(FavoriteError::Conflict)
    ));
    assert!(
        toggle_idempotent_on_connection(&mut conn, "b", "user", "expert", "key-1")
            .unwrap()
            .favorite
    );
    assert!(
        toggle_idempotent_on_connection(&mut conn, "a", "another-user", "expert", "key-1")
            .unwrap()
            .favorite
    );
}

#[test]
fn receipt_write_failure_rolls_back_the_favorite_mutation() {
    let mut conn = Connection::open_in_memory().unwrap();
    schema(&conn);
    toggle_idempotent_on_connection(&mut conn, "a", "user", "expert", "first").unwrap();
    toggle_on_connection(&mut conn, "a", "expert").unwrap();
    conn.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON favorite_request_receipts BEGIN SELECT RAISE(ABORT,'receipt failure'); END;").unwrap();
    assert!(toggle_idempotent_on_connection(&mut conn, "a", "user", "expert", "second").is_err());
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM favorites", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);
    conn.execute_batch("DROP TRIGGER fail_receipt").unwrap();
    assert!(
        !toggle_idempotent_on_connection(&mut conn, "a", "user", "expert", "second")
            .unwrap()
            .replayed
    );
}

#[test]
fn concurrent_same_key_commits_once() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent-receipts.db");
    schema(&Connection::open(&path).unwrap());
    let barrier = Arc::new(Barrier::new(12));
    let threads: Vec<_> = (0..12)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::open(path).unwrap();
                conn.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
                barrier.wait();
                toggle_idempotent_on_connection(&mut conn, "a", "user", "expert", "same-key")
                    .unwrap()
            })
        })
        .collect();
    let receipts: Vec<_> = threads.into_iter().map(|thread| thread.join().unwrap()).collect();
    assert_eq!(receipts.iter().filter(|receipt| !receipt.replayed).count(), 1);
    assert!(receipts.iter().all(|receipt| receipt.favorite && receipt.current_favorite));
    assert!(receipts.iter().all(|receipt| receipt.committed_at == receipts[0].committed_at));
}

#[test]
fn missing_objects_are_rejected_and_failed_write_rolls_back() {
    let mut conn = Connection::open_in_memory().unwrap();
    schema(&conn);
    assert!(matches!(toggle_on_connection(&mut conn, "c", "expert"), Err(FavoriteError::NotFound)));
    conn.execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON favorites BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(matches!(
        toggle_on_connection(&mut conn, "a", "expert"),
        Err(FavoriteError::Storage(_))
    ));
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM favorites", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 0);
    conn.execute_batch("DROP TRIGGER fail_insert").unwrap();
    assert!(toggle_on_connection(&mut conn, "a", "expert").unwrap());
    conn.execute_batch("CREATE TRIGGER fail_delete BEFORE DELETE ON favorites BEGIN SELECT RAISE(ABORT,'failure'); END;").unwrap();
    assert!(toggle_on_connection(&mut conn, "a", "expert").is_err());
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM favorites", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn independent_connections_serialize_toggles_without_lost_updates() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("favorites.db");
    schema(&Connection::open(&path).unwrap());
    let barrier = Arc::new(Barrier::new(12));
    let threads: Vec<_> = (0..12)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut conn = Connection::open(path).unwrap();
                conn.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
                barrier.wait();
                toggle_on_connection(&mut conn, "a", "expert").unwrap()
            })
        })
        .collect();
    let outcomes: Vec<_> = threads.into_iter().map(|thread| thread.join().unwrap()).collect();
    assert_eq!(outcomes.iter().filter(|result| **result).count(), 6);
    let conn = Connection::open(path).unwrap();
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM favorites", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn exhausted_writer_wait_returns_error_and_recovers_after_unlock() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("busy.db");
    let owner = Connection::open(&path).unwrap();
    schema(&owner);
    let mut contender = Connection::open(&path).unwrap();
    contender.busy_timeout(std::time::Duration::from_millis(10)).unwrap();
    owner.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert!(matches!(
        toggle_on_connection(&mut contender, "a", "expert"),
        Err(FavoriteError::Storage(_))
    ));
    owner.execute_batch("ROLLBACK").unwrap();
    assert!(toggle_on_connection(&mut contender, "a", "expert").unwrap());
    assert!(toggle_on_connection(&mut contender, "b", "expert").unwrap());
    assert!(!toggle_on_connection(&mut contender, "a", "expert").unwrap());
    let count: i64 = contender
        .query_row("SELECT COUNT(*) FROM favorites WHERE tenant_id='b'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
