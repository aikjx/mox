// =============================================================================
// SqliteKbStore：KbStore trait 的 SQLite + FTS5 持久化实现
// =============================================================================
//
// 替换 mox-kb-server 中原先的 InMemoryKbStore，trait 接口（save_document /
// get_document / search_documents / delete_document / list_versions）保持不变。
//
// 表结构：
// - kb_doc(id, title, source, mime, created_at, deleted_at, payload)
//   · payload 存完整 Document JSON，保证 get_document 无损回读
//   · source  ← author（出处），mime ← doc_type（便于按类型过滤）
//   · deleted_at 非空即软删除
// - kb_chunk(doc_id, seq, text, embedding BLOB)
//   · embedding 仅预留字段，本次不实现向量检索算法（只读路径走 BM25）
// - kb_fts：FTS5 外部内容表，content='kb_chunk'，按 rowid 回表
//   · tokenize='trigram'：支持中文子串匹配（unicode61 会把整段中文切成一个 token）
//
// 运行态：data/kb.db（可用环境变量 KB_DB_PATH 覆盖），WAL + synchronous=NORMAL。

use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::{params, Connection};

use crate::{Document, DocumentVersion, KbError, KbResult, KbStore, SearchQuery, SearchResult};

/// 默认数据库文件路径（相对工作目录）
const DEFAULT_DB_PATH: &str = "data/kb.db";
/// 路径覆盖环境变量名
const DB_PATH_ENV: &str = "KB_DB_PATH";

/// DDL：PRAGMA + 建表（幂等，可重复执行）
const SCHEMA_SQL: &str = "
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=5000;

CREATE TABLE IF NOT EXISTS kb_doc (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    source      TEXT NOT NULL DEFAULT '',
    mime        TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL,
    deleted_at  TEXT,
    payload     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS kb_chunk (
    doc_id    TEXT NOT NULL REFERENCES kb_doc(id) ON DELETE CASCADE,
    seq       INTEGER NOT NULL,
    text      TEXT NOT NULL,
    embedding BLOB
);
CREATE INDEX IF NOT EXISTS idx_kb_chunk_doc ON kb_chunk(doc_id);

CREATE VIRTUAL TABLE IF NOT EXISTS kb_fts USING fts5(
    text,
    content='kb_chunk',
    content_rowid='rowid',
    tokenize='trigram'
);
";

/// SQLite 持久化知识库存储
pub struct SqliteKbStore {
    conn: Mutex<Connection>,
}

impl SqliteKbStore {
    /// 按路径打开（或创建）数据库并建表。父目录不存在会自动创建。
    pub fn open(path: impl AsRef<std::path::Path>) -> KbResult<Self> {
        let path = path.as_ref();
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| KbError::StorageError(format!("创建数据库目录失败: {e}")))?;
            }
        }
        let conn = Connection::open(path).map_err(map_sqlite_err)?;
        conn.execute_batch(SCHEMA_SQL).map_err(map_sqlite_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// 默认路径打开：`KB_DB_PATH` 环境变量优先，否则 `data/kb.db`。
    pub fn open_default() -> KbResult<Self> {
        let path = std::env::var(DB_PATH_ENV).unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
        Self::open(path)
    }

    fn lock_conn(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Connection>, KbError> {
        self.conn
            .lock()
            .map_err(|e| KbError::StorageError(format!("SQLite 连接锁中毒: {e}")))
    }
}

#[async_trait]
impl KbStore for SqliteKbStore {
    async fn save_document(&self, doc: &Document) -> KbResult<()> {
        let conn = self.lock_conn()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(map_sqlite_err)?;

        let payload = serde_json::to_string(doc)
            .map_err(|e| KbError::StorageError(format!("文档序列化失败: {e}")))?;

        // 1) upsert kb_doc（软删除的文档重新 save 时复活）
        tx.execute(
            "INSERT INTO kb_doc(id, title, source, mime, created_at, deleted_at, payload)
             VALUES(?1, ?2, ?3, ?4, ?5, NULL, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 title = excluded.title,
                 source = excluded.source,
                 mime = excluded.mime,
                 deleted_at = NULL,
                 payload = excluded.payload",
            params![doc.id, doc.title, doc.author, doc.doc_type, doc.created_at, payload],
        )
        .map_err(map_sqlite_err)?;

        // 2) 清掉旧 chunk 与其 FTS 索引项，再写入新 chunk（单 chunk：标题+正文）
        tx.execute(
            "DELETE FROM kb_fts WHERE rowid IN (SELECT rowid FROM kb_chunk WHERE doc_id = ?1)",
            params![doc.id],
        )
        .map_err(map_sqlite_err)?;
        tx.execute("DELETE FROM kb_chunk WHERE doc_id = ?1", params![doc.id])
            .map_err(map_sqlite_err)?;

        let chunk_text = format!("{}\n{}", doc.title, doc.content);
        tx.execute(
            "INSERT INTO kb_chunk(doc_id, seq, text, embedding) VALUES(?1, 0, ?2, NULL)",
            params![doc.id, chunk_text],
        )
        .map_err(map_sqlite_err)?;
        let chunk_rowid = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO kb_fts(rowid, text) VALUES(?1, ?2)",
            params![chunk_rowid, chunk_text],
        )
        .map_err(map_sqlite_err)?;

        tx.commit().map_err(map_sqlite_err)?;
        Ok(())
    }

    async fn get_document(&self, doc_id: &str) -> KbResult<Option<Document>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT payload FROM kb_doc WHERE id = ?1 AND deleted_at IS NULL",
            )
            .map_err(map_sqlite_err)?;
        let mut rows = stmt
            .query(params![doc_id])
            .map_err(map_sqlite_err)?;
        let row = match rows.next().map_err(map_sqlite_err)? {
            Some(r) => r,
            None => return Ok(None),
        };
        let payload: String = row.get(0).map_err(map_sqlite_err)?;
        let doc: Document = serde_json::from_str(&payload)
            .map_err(|e| KbError::StorageError(format!("文档反序列化失败: {e}")))?;
        Ok(Some(doc))
    }

    async fn search_documents(&self, query: &SearchQuery) -> KbResult<SearchResult> {
        let started = std::time::Instant::now();
        let conn = self.lock_conn()?;

        // 命中的文档 id + payload（按 BM25 相关性排序，先全量取回再分页）
        let mut docs: Vec<Document> = if query.keyword.trim().is_empty() {
            // 空关键词：列出全部未删除文档（按创建时间倒序）
            let mut stmt = conn
                .prepare(
                    "SELECT payload FROM kb_doc
                     WHERE deleted_at IS NULL
                       AND (?1 IS NULL OR mime = ?1)
                     ORDER BY created_at DESC",
                )
                .map_err(map_sqlite_err)?;
            let rows = stmt
                .query(params![query.doc_type])
                .map_err(map_sqlite_err)?;
            collect_docs(rows)?
        } else {
            // FTS5 MATCH 取命中 rowid + BM25 分（必须在只含 kb_fts 的查询里求 bm25，
            // 与其它表 JOIN 后子查询会被合并展开，触发
            // "unable to use function bm25 in the requested context"）。
            let match_expr = fts_phrase(&query.keyword);
            let mut hit_stmt = conn
                .prepare(
                    "SELECT rowid, bm25(kb_fts) FROM kb_fts WHERE kb_fts MATCH ?1 ORDER BY bm25(kb_fts)",
                )
                .map_err(map_sqlite_err)?;
            let hit_rows = hit_stmt
                .query(params![match_expr])
                .map_err(|e| {
                    KbError::SearchError(format!("FTS 查询失败 ({e})，关键词: {match_expr}"))
                })?;

            // (doc_id, rank) 列表；同一文档多 chunk 命中时保留最佳（最小）rank
            let mut best: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
            let mut rows = hit_rows;
            while let Some(hit) = rows.next().map_err(map_sqlite_err)? {
                let rid: i64 = hit.get(0).map_err(map_sqlite_err)?;
                let rank: f64 = hit.get(1).map_err(map_sqlite_err)?;
                let mut c_stmt = conn
                    .prepare("SELECT doc_id FROM kb_chunk WHERE rowid = ?1")
                    .map_err(map_sqlite_err)?;
                let mut c_rows = c_stmt.query(params![rid]).map_err(map_sqlite_err)?;
                if let Some(cr) = c_rows.next().map_err(map_sqlite_err)? {
                    let doc_id: String = cr.get(0).map_err(map_sqlite_err)?;
                    match best.get_mut(&doc_id) {
                        Some(r) if *r <= rank => {}
                        _ => {
                            best.insert(doc_id, rank);
                        }
                    }
                }
            }
            drop(rows);

            // 按 BM25 升序（分值越小越相关）回表取 payload，过滤软删除与 doc_type
            let mut ranked: Vec<(&String, &f64)> = best.iter().collect();
            ranked.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));

            let mut docs = Vec::new();
            for (doc_id, _rank) in ranked {
                let mut d_stmt = conn
                    .prepare(
                        "SELECT payload FROM kb_doc
                         WHERE id = ?1 AND deleted_at IS NULL
                           AND (?2 IS NULL OR mime = ?2)",
                    )
                    .map_err(map_sqlite_err)?;
                let mut d_rows = d_stmt
                    .query(params![doc_id, query.doc_type])
                    .map_err(map_sqlite_err)?;
                if let Some(row) = d_rows.next().map_err(map_sqlite_err)? {
                    let payload: String = row.get(0).map_err(map_sqlite_err)?;
                    let doc: Document = serde_json::from_str(&payload)
                        .map_err(|e| KbError::StorageError(format!("文档反序列化失败: {e}")))?;
                    docs.push(doc);
                }
            }
            docs
        };

        let total = docs.len() as u64;
        let start = ((query.page.saturating_sub(1)) * query.page_size) as usize;
        docs.truncate(start + query.page_size as usize);
        let items = if start < docs.len() {
            docs.split_off(start)
        } else {
            Vec::new()
        };

        Ok(SearchResult {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    async fn delete_document(&self, doc_id: &str) -> KbResult<()> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn
            .execute(
                "UPDATE kb_doc SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
                params![now, doc_id],
            )
            .map_err(map_sqlite_err)?;
        if affected == 0 {
            return Err(KbError::DocumentNotFound(doc_id.to_string()));
        }
        Ok(())
    }

    async fn list_versions(&self, _doc_id: &str) -> KbResult<Vec<DocumentVersion>> {
        // 当前模型未落版本快照，与 InMemoryKbStore 行为保持一致：返回空列表。
        // 后续接入 kb_version 表时只需扩展此实现，trait 签名不变。
        Ok(Vec::new())
    }
}

/// rusqlite 错误 → KbError
fn map_sqlite_err(e: rusqlite::Error) -> KbError {
    KbError::StorageError(e.to_string())
}

/// 把用户关键词包成 FTS5 短语字面量（双引号包裹，内部双引号双写），
/// 避免把用户输入里的 `:` `"` `-` 等字符当成 FTS 语法符号。
fn fts_phrase(keyword: &str) -> String {
    format!("\"{}\"", keyword.replace('"', "\"\""))
}

/// 遍历 rows，把 payload 列反序列化为 Document
fn collect_docs(
    mut rows: rusqlite::Rows<'_>,
) -> KbResult<Vec<Document>> {
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(map_sqlite_err)? {
        let payload: String = row.get(0).map_err(map_sqlite_err)?;
        let doc: Document = serde_json::from_str(&payload)
            .map_err(|e| KbError::StorageError(format!("文档反序列化失败: {e}")))?;
        out.push(doc);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    /// 建一个唯一的临时库路径（每个测试互不干扰）
    fn tmp_db_path(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mox_kb_sqlite_{tag}_{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).expect("创建临时目录");
        dir.join("kb.db")
    }

    fn sample_doc(title: &str, content: &str) -> Document {
        Document::new(title, content, "tester")
    }

    #[tokio::test]
    async fn test_open_default_compiles_and_empty_search() {
        // open_default 走默认路径会写仓库 data/kb.db，测试里不打它；
        // 这里只验证带路径打开 + 空库行为。
        let path = tmp_db_path("empty");
        let store = SqliteKbStore::open(&path).expect("打开空库");
        let res = store
            .search_documents(&SearchQuery {
                keyword: "anything".to_string(),
                ..Default::default()
            })
            .await
            .expect("搜索不报错");
        assert_eq!(res.total, 0);
        assert!(store
            .get_document("nope")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_save_get_search_roundtrip() {
        let path = tmp_db_path("roundtrip");
        let store = SqliteKbStore::open(&path).expect("打开库");

        let doc = sample_doc(
            "Rust 持久化指南",
            "Rust 语言使用 SQLite 做持久化，配合 FTS5 全文检索 BM25 排序。",
        );
        store.save_document(&doc).await.expect("保存");

        let fetched = store.get_document(&doc.id).await.unwrap().expect("应取回文档");
        assert_eq!(fetched.id, doc.id);
        assert_eq!(fetched.title, doc.title);
        assert_eq!(fetched.content, doc.content);

        // FTS 命中（ASCII 关键词 ≥3 字符，trigram 可索引）
        let res = store
            .search_documents(&SearchQuery {
                keyword: "Rust".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(res.total, 1, "搜索 Rust 应命中刚写入的文档");
        assert_eq!(res.items[0].id, doc.id);

        // doc_type 过滤：doc_type 默认 article，命中
        let res = store
            .search_documents(&SearchQuery {
                keyword: "Rust".to_string(),
                doc_type: Some("article".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(res.total, 1);
        // 不存在的类型过滤：0
        let res = store
            .search_documents(&SearchQuery {
                keyword: "Rust".to_string(),
                doc_type: Some("faq".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(res.total, 0);
    }

    /// 核心持久化用例：写入 → 搜索命中 → drop store → 同路径重开 → 文档仍在、FTS 仍能搜到
    #[tokio::test]
    async fn test_persistence_survives_reopen() {
        let path = tmp_db_path("reopen");
        let doc = sample_doc(
            "FTS5 重启验证",
            "SQLite FTS5 索引必须在进程结束后仍然可搜：BM25 排序 trigrams。",
        );
        let doc_id = doc.id.clone();

        // 第一阶段：写入并搜索命中
        {
            let store = SqliteKbStore::open(&path).expect("第一次打开");
            store.save_document(&doc).await.expect("保存");
            let res = store
                .search_documents(&SearchQuery {
                    keyword: "FTS5".to_string(),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(res.total, 1, "写入后应立即命中");
            // drop store：连接随作用域结束关闭（模拟进程退出）
        }

        // 证明文件确实落盘（含 WAL 副产物）
        assert!(path.exists(), "kb.db 文件应已落盘");

        // 第二阶段：同一路径重新打开，数据与 FTS 索引都应还在
        {
            let store = SqliteKbStore::open(&path).expect("第二次打开");
            let fetched = store
                .get_document(&doc_id)
                .await
                .unwrap()
                .expect("重开后文档仍在");
            assert_eq!(fetched.title, doc.title);
            assert_eq!(fetched.content, doc.content);

            let res = store
                .search_documents(&SearchQuery {
                    keyword: "FTS5".to_string(),
                    ..Default::default()
                })
                .await
                .unwrap();
            assert_eq!(res.total, 1, "重开后 FTS 仍能搜到");
            assert_eq!(res.items[0].id, doc_id);

            // 空关键词列表也应包含该文档
            let all = store
                .search_documents(&SearchQuery::default())
                .await
                .unwrap();
            assert_eq!(all.total, 1);
        }
    }

    #[tokio::test]
    async fn test_soft_delete_hides_from_get_and_search() {
        let path = tmp_db_path("softdelete");
        let store = SqliteKbStore::open(&path).expect("打开");
        let doc = sample_doc("删除测试文档", "delete me please");
        store.save_document(&doc).await.expect("保存");

        store.delete_document(&doc.id).await.expect("删除");
        // get 不可见
        assert!(store.get_document(&doc.id).await.unwrap().is_none());
        // search 不可见
        let res = store
            .search_documents(&SearchQuery {
                keyword: "delete".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(res.total, 0, "软删除后搜索不应命中");

        // 重复删除应报 DocumentNotFound
        assert!(matches!(
            store.delete_document(&doc.id).await,
            Err(KbError::DocumentNotFound(_))
        ));

        // 重新 save 同一文档 → 复活
        store.save_document(&doc).await.expect("复活");
        assert!(store.get_document(&doc.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_overwrite_keeps_single_row() {
        let path = tmp_db_path("overwrite");
        let store = SqliteKbStore::open(&path).expect("打开");
        let mut doc = sample_doc("原始标题", "original content body");
        store.save_document(&doc).await.expect("v1");
        doc.title = "更新后的标题".to_string();
        doc.content = "brand new content about Rust overwrite".to_string();
        store.save_document(&doc).await.expect("v2");

        let fetched = store.get_document(&doc.id).await.unwrap().unwrap();
        assert_eq!(fetched.title, "更新后的标题");
        // 旧词应搜不到（FTS 已重建），新词应命中
        let old = store
            .search_documents(&SearchQuery {
                keyword: "original".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(old.total, 0, "覆盖后旧内容不应再被搜到");
        let new = store
            .search_documents(&SearchQuery {
                keyword: "brand".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(new.total, 1, "覆盖后新内容应可被搜到");
    }

    #[tokio::test]
    async fn test_list_versions_returns_empty_unchanged() {
        let path = tmp_db_path("versions");
        let store = SqliteKbStore::open(&path).expect("打开");
        let doc = sample_doc("版本测试", "version snapshot test");
        store.save_document(&doc).await.expect("保存");
        let versions = store.list_versions(&doc.id).await.unwrap();
        assert!(versions.is_empty(), "与 InMemory 行为一致：暂无版本快照落库");
    }
}
