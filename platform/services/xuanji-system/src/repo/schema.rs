//! 方言无关 SQL 构建层（等价 Spring Boot 的方言抽象）
//!
//! 当前用 `sea-query` 生成**建表语句**，按 `Backend` 选择 `SqliteQueryBuilder` /
//! `PostgresQueryBuilder` / `MysqlQueryBuilder`，自动处理：
//! - 自增：`AUTOINCREMENT`(sqlite) / `SERIAL`(pg) / `AUTO_INCREMENT`(mysql)
//! - 标识引用符：`` ` ``(mysql) / `"`(pg/sqlite)
//!
//! 写穿 SQL（upsert/select）在 sqlite 后端仍走手写 `INSERT OR REPLACE`；后续接入
//! Postgres/MySQL 后端时，可在本层扩展 `upsert_sql` 生成 ON CONFLICT / ON DUPLICATE KEY，
//! 实现"一套逻辑多后端"。

use sea_query::{Alias, ColumnDef, Iden, Table, SqliteQueryBuilder, PostgresQueryBuilder, MysqlQueryBuilder};
use crate::config::Backend;

#[derive(Iden)]
pub enum Xuanjis { Table, Id, Data }
#[derive(Iden)]
pub enum Members { Table, Id, XuanjiId, Data }
#[derive(Iden)]
pub enum Tasks { Table, Id, XuanjiId, Data }
#[derive(Iden)]
pub enum Channels { Table, Id, XuanjiId, Data }
#[derive(Iden)]
pub enum Messages { Table, Id, ChannelId, Data }
#[derive(Iden)]
pub enum Notifications { Table, Id, MemberId, Data }
#[derive(Iden)]
pub enum Bindings { Table, MemberId, Data }
#[derive(Iden)]
pub enum Tokens { Table, Hash, MemberId }
#[derive(Iden)]
pub enum Audit { Table, Seq, Id, Data, At }

/// 展开 9 张表的建表语句（用具体方言构建器 $b 值；每次 .build 用 fresh builder，无 move 冲突）
macro_rules! ddl {
    ($b:expr) => {
        vec![
            Table::create().table(Xuanjis::Table).if_not_exists()
                .col(ColumnDef::new(Xuanjis::Id).string().not_null().primary_key())
                .col(ColumnDef::new(Xuanjis::Data).text().not_null())
                .build($b),
            Table::create().table(Members::Table).if_not_exists()
                .col(ColumnDef::new(Members::Id).string().not_null().primary_key())
                .col(ColumnDef::new(Members::XuanjiId).string().not_null())
                .col(ColumnDef::new(Members::Data).text().not_null())
                .build($b),
            Table::create().table(Tasks::Table).if_not_exists()
                .col(ColumnDef::new(Tasks::Id).string().not_null().primary_key())
                .col(ColumnDef::new(Tasks::XuanjiId).string().not_null())
                .col(ColumnDef::new(Tasks::Data).text().not_null())
                .build($b),
            Table::create().table(Channels::Table).if_not_exists()
                .col(ColumnDef::new(Channels::Id).string().not_null().primary_key())
                .col(ColumnDef::new(Channels::XuanjiId).string().not_null())
                .col(ColumnDef::new(Channels::Data).text().not_null())
                .build($b),
            Table::create().table(Messages::Table).if_not_exists()
                .col(ColumnDef::new(Messages::Id).string().not_null().primary_key())
                .col(ColumnDef::new(Messages::ChannelId).string().not_null())
                .col(ColumnDef::new(Messages::Data).text().not_null())
                .build($b),
            Table::create().table(Notifications::Table).if_not_exists()
                .col(ColumnDef::new(Notifications::Id).string().not_null().primary_key())
                .col(ColumnDef::new(Notifications::MemberId).string().not_null())
                .col(ColumnDef::new(Notifications::Data).text().not_null())
                .build($b),
            Table::create().table(Bindings::Table).if_not_exists()
                .col(ColumnDef::new(Bindings::MemberId).string().not_null().primary_key())
                .col(ColumnDef::new(Bindings::Data).text().not_null())
                .build($b),
            Table::create().table(Tokens::Table).if_not_exists()
                .col(ColumnDef::new(Tokens::Hash).string().not_null().primary_key())
                .col(ColumnDef::new(Tokens::MemberId).string().not_null())
                .build($b),
            Table::create().table(Audit::Table).if_not_exists()
                .col(ColumnDef::new(Audit::Seq).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Audit::Id).string().not_null())
                .col(ColumnDef::new(Audit::Data).text().not_null())
                .col(ColumnDef::new(Audit::At).big_integer().not_null())
                .build($b),
        ]
    };
}

/// 建表语句（所有表，按方言生成）
pub fn create_tables_sql(backend: Backend) -> Vec<String> {
    match backend {
        Backend::Sqlite => ddl!(SqliteQueryBuilder),
        Backend::Postgres => ddl!(PostgresQueryBuilder),
        Backend::MySql => ddl!(MysqlQueryBuilder),
    }
}

/// 通用 SELECT 某表 data 列的语句（表名/列名经由 Alias 动态化）
pub fn select_all_sql(table: &str, col: &str) -> String {
    format!("SELECT {} FROM {}", Alias::new(col).to_string(), Alias::new(table).to_string())
}

/// 按方言生成实体 upsert 语句（主键冲突则更新其余列）
///
/// - sqlite：`INSERT OR REPLACE`（整行替换，语义与原手写 SQL 一致）
/// - postgres：`ON CONFLICT (pk) DO UPDATE SET ...`
/// - mysql：`ON DUPLICATE KEY UPDATE ...`
///
/// 占位符：sqlite `?N`、postgres `$N`、mysql `?`。
/// 表名/列名统一不带引号（小写标识符，三方言均合法），与建表 DDL 的 Iden 一致。
pub fn upsert_sql(backend: Backend, table: &str, pk: &str, cols: &[&str]) -> String {
    let all_cols: Vec<&str> = std::iter::once(pk).chain(cols.iter().copied()).collect();
    let col_list = all_cols.join(", ");
    let set_clause = cols.iter().map(|c| format!("{c} = EXCLUDED.{c}")).collect::<Vec<_>>().join(", ");
    match backend {
        Backend::Sqlite => {
            let ph = all_cols.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect::<Vec<_>>().join(", ");
            format!("INSERT OR REPLACE INTO {table} ({col_list}) VALUES ({ph})")
        }
        Backend::Postgres => {
            let ph = all_cols.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect::<Vec<_>>().join(", ");
            format!(
                "INSERT INTO {table} ({col_list}) VALUES ({ph}) ON CONFLICT ({pk}) DO UPDATE SET {set_clause}"
            )
        }
        Backend::MySql => {
            let ph = vec!["?"; all_cols.len()].join(", ");
            let dup = cols.iter().map(|c| format!("{c} = VALUES({c})")).collect::<Vec<_>>().join(", ");
            format!("INSERT INTO {table} ({col_list}) VALUES ({ph}) ON DUPLICATE KEY UPDATE {dup}")
        }
    }
}
