// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家存储（SQLite 实现）

use rusqlite::{Connection, params};
use std::sync::Arc;
use std::sync::Mutex;

use crate::models::Expert;

/// 专家存储
pub struct ExpertStore {
    conn: Arc<Mutex<Connection>>,
}

impl ExpertStore {
    /// 打开或创建数据库
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS experts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                title TEXT,
                organization TEXT,
                domains TEXT,
                skills TEXT,
                bio TEXT,
                enabled INTEGER,
                rating REAL,
                total_consultations INTEGER
            );"
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 内存数据库（测试用）
    pub fn memory() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS experts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                title TEXT,
                organization TEXT,
                domains TEXT,
                skills TEXT,
                bio TEXT,
                enabled INTEGER,
                rating REAL,
                total_consultations INTEGER
            );"
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 列出所有专家
    pub fn list(&self) -> Result<Vec<Expert>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, title, organization, domains, skills, bio, enabled, rating, total_consultations FROM experts")?;
        let experts = stmt.query_map([], |row| {
            let domains_json: String = row.get(4)?;
            let skills_json: String = row.get(5)?;
            Ok(Expert {
                id: row.get(0)?,
                name: row.get(1)?,
                title: row.get(2)?,
                organization: row.get(3)?,
                domains: serde_json::from_str(&domains_json).unwrap_or_default(),
                skills: serde_json::from_str(&skills_json).unwrap_or_default(),
                bio: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                rating: row.get(8)?,
                total_consultations: row.get(9)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(experts)
    }

    /// 获取单个专家
    pub fn get(&self, id: &str) -> Result<Option<Expert>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, title, organization, domains, skills, bio, enabled, rating, total_consultations FROM experts WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let domains_json: String = row.get(4)?;
            let skills_json: String = row.get(5)?;
            Ok(Some(Expert {
                id: row.get(0)?,
                name: row.get(1)?,
                title: row.get(2)?,
                organization: row.get(3)?,
                domains: serde_json::from_str(&domains_json).unwrap_or_default(),
                skills: serde_json::from_str(&skills_json).unwrap_or_default(),
                bio: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                rating: row.get(8)?,
                total_consultations: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 创建专家
    pub fn create(&self, expert: &Expert) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO experts (id, name, title, organization, domains, skills, bio, enabled, rating, total_consultations) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                expert.id,
                expert.name,
                expert.title,
                expert.organization,
                serde_json::to_string(&expert.domains)?,
                serde_json::to_string(&expert.skills)?,
                expert.bio,
                expert.enabled as i32,
                expert.rating,
                expert.total_consultations,
            ],
        )?;
        Ok(())
    }

    /// 更新专家
    pub fn update(&self, expert: &Expert) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE experts SET name = ?2, title = ?3, organization = ?4, domains = ?5, skills = ?6, bio = ?7, enabled = ?8, rating = ?9, total_consultations = ?10 WHERE id = ?1",
            params![
                expert.id,
                expert.name,
                expert.title,
                expert.organization,
                serde_json::to_string(&expert.domains)?,
                serde_json::to_string(&expert.skills)?,
                expert.bio,
                expert.enabled as i32,
                expert.rating,
                expert.total_consultations,
            ],
        )?;
        Ok(())
    }

    /// 删除专家
    pub fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM experts WHERE id = ?1", params![id])?;
        Ok(())
    }
}
