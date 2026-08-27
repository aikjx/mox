// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Postgres 后端实现（sqlx 连接池，全异步）
//!
//! 等价 Spring Boot 的 JPA Repository：连接串形如
//! `postgres://user:pass@host:5432/dbname`，启动时自动建表（IF NOT EXISTS），
//! 写入走 `ON CONFLICT DO UPDATE` upsert，加载走全表 SELECT 重放。

use std::collections::HashMap;

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use crate::config::Backend;
use crate::model::*;
use crate::rbac::RoleBinding;
use crate::store::{id_of, State};

use super::schema;
use super::Repository;

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub async fn open(url: &str) -> Result<Self, String> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(|e| format!("Postgres 连接失败: {}", e))?;
        Ok(Self { pool })
    }

    /// 执行单条 upsert（所有参数均为字符串类型）
    async fn exec_str(&self, sql: &str, args: &[&str]) {
        let mut q = sqlx::query(sql);
        for a in args {
            q = q.bind(*a);
        }
        if let Err(e) = q.execute(&self.pool).await {
            tracing::error!("postgres 写穿失败 [{}]: {}", sql, e);
        }
    }

    /// 加载单表（data 列）到实体 map，复用 store::id_of 提取主键
    async fn load_into<T: serde::de::DeserializeOwned + serde::Serialize + Clone>(
        &self,
        map: &mut HashMap<String, T>,
        sql: &str,
    ) {
        match sqlx::query(sql).fetch_all(&self.pool).await {
            Ok(rows) => {
                for row in rows {
                    if let Ok(data) = row.try_get::<String, _>("data") {
                        if let Ok(v) = serde_json::from_str::<T>(&data) {
                            if let Some(key) = id_of::<T>(&v) {
                                map.insert(key, v);
                            }
                        }
                    }
                }
            }
            Err(e) => tracing::error!("postgres 加载失败 [{}]: {}", sql, e),
        }
    }
}

#[async_trait::async_trait]
impl Repository for PostgresRepository {
    async fn migrate(&self) -> Result<(), String> {
        for sql in schema::create_tables_sql(Backend::Postgres) {
            sqlx::query(&sql)
                .execute(&self.pool)
                .await
                .map_err(|e| format!("建表失败 [{}]: {}", sql, e))?;
        }
        Ok(())
    }

    async fn load_all(&self) -> State {
        let mut st = State::default();
        let mox_sql = schema::select_all_sql("moxs", "data");
        match sqlx::query(&mox_sql).fetch_all(&self.pool).await {
            Ok(rows) => {
                for row in rows {
                    if let Ok(data) = row.try_get::<String, _>("data") {
                        if let Ok(v) = serde_json::from_str::<Mox>(&data) {
                            st.moxs.insert(v.id.clone(), v);
                        }
                    }
                }
            }
            Err(e) => tracing::error!("postgres 加载 moxs 失败: {}", e),
        }
        self.load_into(&mut st.members, "SELECT data FROM members")
            .await;
        self.load_into(&mut st.tasks, "SELECT data FROM tasks")
            .await;
        self.load_into(&mut st.channels, "SELECT data FROM channels")
            .await;
        self.load_into(&mut st.messages, "SELECT data FROM messages")
            .await;
        self.load_into(&mut st.notifications, "SELECT data FROM notifications")
            .await;

        if let Ok(rows) = sqlx::query("SELECT member_id, data FROM bindings")
            .fetch_all(&self.pool)
            .await
        {
            for row in rows {
                if let Ok(member_id) = row.try_get::<String, _>("member_id") {
                    if let Ok(data) = row.try_get::<String, _>("data") {
                        if let Ok(binds) = serde_json::from_str::<Vec<RoleBinding>>(&data) {
                            st.bindings.insert(member_id, binds);
                        }
                    }
                }
            }
        }
        if let Ok(rows) = sqlx::query("SELECT hash, member_id FROM tokens")
            .fetch_all(&self.pool)
            .await
        {
            for row in rows {
                if let (Ok(hash), Ok(member_id)) = (
                    row.try_get::<String, _>("hash"),
                    row.try_get::<String, _>("member_id"),
                ) {
                    st.tokens.insert(hash, member_id);
                }
            }
        }
        if let Ok(rows) = sqlx::query("SELECT data FROM audit ORDER BY seq")
            .fetch_all(&self.pool)
            .await
        {
            for row in rows {
                if let Ok(data) = row.try_get::<String, _>("data") {
                    if let Ok(v) = serde_json::from_str::<AuditRecord>(&data) {
                        st.audit.push(v);
                    }
                }
            }
        }
        st
    }

    async fn persist_mox(&self, a: &Mox) {
        let sql = schema::upsert_sql(Backend::Postgres, "moxs", "id", &["data"]);
        self.exec_str(
            &sql,
            &[&a.id, &serde_json::to_string(a).unwrap_or_default()],
        )
        .await;
    }
    async fn persist_member(&self, m: &Member) {
        let sql = schema::upsert_sql(Backend::Postgres, "members", "id", &["mox_id", "data"]);
        self.exec_str(
            &sql,
            &[
                &m.id,
                &m.mox_id,
                &serde_json::to_string(m).unwrap_or_default(),
            ],
        )
        .await;
    }
    async fn persist_task(&self, t: &Task) {
        let sql = schema::upsert_sql(Backend::Postgres, "tasks", "id", &["mox_id", "data"]);
        self.exec_str(
            &sql,
            &[
                &t.id,
                &t.mox_id,
                &serde_json::to_string(t).unwrap_or_default(),
            ],
        )
        .await;
    }
    async fn persist_channel(&self, c: &Channel) {
        let sql = schema::upsert_sql(Backend::Postgres, "channels", "id", &["mox_id", "data"]);
        self.exec_str(
            &sql,
            &[
                &c.id,
                &c.mox_id,
                &serde_json::to_string(c).unwrap_or_default(),
            ],
        )
        .await;
    }
    async fn persist_message(&self, m: &Message) {
        let sql = schema::upsert_sql(Backend::Postgres, "messages", "id", &["channel_id", "data"]);
        self.exec_str(
            &sql,
            &[
                &m.id,
                &m.channel_id,
                &serde_json::to_string(m).unwrap_or_default(),
            ],
        )
        .await;
    }
    async fn persist_notification(&self, n: &Notification) {
        let sql = schema::upsert_sql(
            Backend::Postgres,
            "notifications",
            "id",
            &["member_id", "data"],
        );
        self.exec_str(
            &sql,
            &[
                &n.id,
                &n.member_id,
                &serde_json::to_string(n).unwrap_or_default(),
            ],
        )
        .await;
    }
    async fn persist_bindings(&self, member_id: &str, bindings: &[RoleBinding]) {
        let sql = schema::upsert_sql(Backend::Postgres, "bindings", "member_id", &["data"]);
        self.exec_str(
            &sql,
            &[
                member_id,
                &serde_json::to_string(bindings).unwrap_or_default(),
            ],
        )
        .await;
    }
    async fn persist_token(&self, hash: &str, member_id: &str) {
        let sql = schema::upsert_sql(Backend::Postgres, "tokens", "hash", &["member_id"]);
        self.exec_str(&sql, &[hash, member_id]).await;
    }
    async fn persist_audit(&self, r: &AuditRecord) {
        // audit 表 seq 自增主键，只追加不更新
        if let Err(e) = sqlx::query("INSERT INTO audit (id, data, at) VALUES ($1, $2, $3)")
            .bind(&r.id)
            .bind(serde_json::to_string(r).unwrap_or_default())
            .bind(r.at.timestamp())
            .execute(&self.pool)
            .await
        {
            tracing::error!("postgres 写穿 audit 失败: {}", e);
        }
    }
}
