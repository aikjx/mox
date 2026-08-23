//! AIS A-05 · L5 基础设施 · 持久化 Provider 抽象层（DIP：上层只依赖 trait，不依赖具体 driver）
//!
//! 设计原则：
//! - 接口仅暴露领域语义（`exec_sql / query_rows / open / batch`），不泄漏 rusqlite / sqlx 细节；
//! - sqlite / in-memory / postgres 等实现放在 L5-Infra 的 `xuanji-system` crate（该 crate 独占 rusqlite）；
//! - 其他所有 crate（ai-agent / primiflow-core / hermes...）一律通过本 trait 操作持久化，
//!   从而做到「rusqlite 全仓库只在 xuanji-system Cargo.toml 出现 1 次」（AC-11 规则）。

use std::collections::HashMap;

/// 统一 SQL 绑定值（所有 driver 都必须可无损转换这 6 种值；足够覆盖 ai-agent / primiflow 全量场景）
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
    Bool(bool),
}

/// 统一查询返回行（列名 -> 值）
pub type SqlRow = HashMap<String, SqlValue>;

/// 持久化 Provider 抽象：跨 L3/L4 共享（DIP：上层依赖本抽象，不依赖 rusqlite 具体实现）
///
/// 约束：
/// - `Send + Sync + 'static`：保证被 `Arc<Mutex<...>>` 跨线程共享（与原 rusqlite 用法一一对应）；
/// - 只定义 ai-agent / primiflow-core 实际用到的方法，避免接口膨胀；
/// - 所有实现必须幂等可重入。
pub trait PersistenceProvider: Send + Sync + 'static {
    /// 执行单条 DDL/DML（`INSERT / UPDATE / DELETE / CREATE TABLE` 等），返回受影响行数。
    fn exec(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<usize>;

    /// 批量 DDL（`execute_batch` 语义），无返回。
    fn exec_batch(&self, sql: &str) -> anyhow::Result<()>;

    /// 查询行集合（SQL 占位 `?` 与 params 对齐），每行以 列名 -> SqlValue 返回。
    fn query(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<Vec<SqlRow>>;

    /// 查询零或一行（用于 `query_row` 语义）：
    /// - 0 行 → `Ok(None)`；1 行 → `Ok(Some(...))`；多行 → `Err`（约束保证语义与 rusqlite 一致）。
    fn query_one(&self, sql: &str, params: &[SqlValue]) -> anyhow::Result<Option<SqlRow>> {
        let rows = self.query(sql, params)?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows.into_iter().next().unwrap())),
            n => Err(anyhow::anyhow!("query_one 期望 ≤1 行，实际 {n}")),
        }
    }
}
