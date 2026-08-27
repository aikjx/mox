// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 12-Factor 应用配置（企业级可运维基线）
//!
//! 所有配置项均可通过环境变量（`MOX_*` 前缀）覆盖，便于容器化部署与多环境切换，
//! 无需重新编译即可调整配额、绑定地址、限流与 CORS 策略。
use serde::Deserialize;

/// 资源配额（BR-03 / I-03）：防止单一璇玑内的无界增长拖垮整体
#[derive(Clone, Debug, Deserialize)]
pub struct Quotas {
    /// 单璇玑最大成员数
    pub max_members: usize,
    /// 单璇玑最大任务数
    pub max_tasks: usize,
    /// 单个任务最大被分派人数
    pub max_assignees: usize,
    /// 单个任务最大子任务数
    pub max_subtasks: usize,
    /// 单个任务最大关注人数
    pub max_watchers: usize,
    /// 依赖图最大深度（防深链与潜在栈溢出）
    pub max_dependency_depth: usize,
}

impl Default for Quotas {
    fn default() -> Self {
        Self {
            max_members: 500,
            max_tasks: 2000,
            max_assignees: 20,
            max_subtasks: 50,
            max_watchers: 20,
            max_dependency_depth: 8,
        }
    }
}

/// 持久化后端类型（对应 Spring Boot 的多数据源抽象）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Default)]
pub enum Backend {
    #[default]
    Sqlite,
    Postgres,
    MySql,
}

impl Backend {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "postgres" | "pg" | "postgresql" => Backend::Postgres,
            "mysql" | "mariadb" => Backend::MySql,
            _ => Backend::Sqlite,
        }
    }
}

/// 全局应用配置
#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    /// 是否启用持久化（true=系统记录落盘，false=纯内存）
    pub persist: bool,
    /// 持久化严格模式：打开后若系统记录仓库打开/建表失败，**直接启动失败**
    /// （fail-fast），而非静默回退内存模式。生产环境强烈建议开启，避免"连不上数据库却照常起服务"导致数据只进内存、丢失不可恢复。
    pub strict_persist: bool,
    /// 持久化后端：sqlite / postgres / mysql（默认 sqlite）
    pub backend: Backend,
    /// 数据目录（SQLite 单文件与快照存放处）
    pub data_dir: String,
    /// 数据库连接串（postgres/mysql 使用；sqlite 留空则按 data_dir 生成）
    pub db_url: String,
    /// HTTP 绑定地址
    pub bind_addr: String,
    /// CORS 允许的源（逗号分隔；为空表示不开放跨域）
    pub cors_allowed_origins: Vec<String>,
    /// 限流：每个令牌（或匿名 IP）在窗口内的最大请求数
    pub rate_limit: u32,
    /// 限流窗口（秒）
    pub rate_window_secs: u64,
    /// 日志级别（trace/debug/info/warn/error）
    pub log_level: String,
    /// 资源配额
    pub quotas: Quotas,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            persist: false,
            // 默认关闭严格模式，保持与历史"静默回退内存"行为兼容（测试/演示更友好）
            strict_persist: false,
            backend: Backend::Sqlite,
            data_dir: "./data".to_string(),
            db_url: String::new(),
            bind_addr: "0.0.0.0:3000".to_string(),
            cors_allowed_origins: vec![],
            rate_limit: 120,
            rate_window_secs: 60,
            log_level: "info".to_string(),
            quotas: Quotas::default(),
        }
    }
}

impl AppConfig {
    /// 从环境变量加载，未设置的项回退到默认值（12-Factor 配置）
    pub fn load() -> Self {
        let mut cfg = AppConfig::default();
        let get = |k: &str| std::env::var(format!("MOX_{}", k)).ok();

        if let Some(v) = get("PERSIST") {
            cfg.persist = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Some(v) = get("STRICT_PERSIST") {
            cfg.strict_persist = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Some(v) = get("BACKEND") {
            cfg.backend = Backend::parse(&v);
        }
        if let Some(v) = get("DB_URL") {
            cfg.db_url = v;
        }
        if let Some(v) = get("DATA_DIR") {
            cfg.data_dir = v;
        }
        if let Some(v) = get("BIND") {
            cfg.bind_addr = v;
        }
        if let Some(v) = get("CORS_ORIGINS") {
            cfg.cors_allowed_origins = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Some(v) = get("RATE_LIMIT") {
            if let Ok(n) = v.parse() {
                cfg.rate_limit = n;
            }
        }
        if let Some(v) = get("RATE_WINDOW") {
            if let Ok(n) = v.parse() {
                cfg.rate_window_secs = n;
            }
        }
        if let Some(v) = get("LOG_LEVEL") {
            cfg.log_level = v;
        }
        if let Some(v) = get("MAX_MEMBERS") {
            if let Ok(n) = v.parse() {
                cfg.quotas.max_members = n;
            }
        }
        if let Some(v) = get("MAX_TASKS") {
            if let Ok(n) = v.parse() {
                cfg.quotas.max_tasks = n;
            }
        }
        if let Some(v) = get("MAX_ASSIGNEES") {
            if let Ok(n) = v.parse() {
                cfg.quotas.max_assignees = n;
            }
        }
        if let Some(v) = get("MAX_SUBTASKS") {
            if let Ok(n) = v.parse() {
                cfg.quotas.max_subtasks = n;
            }
        }
        if let Some(v) = get("MAX_DEP_DEPTH") {
            if let Ok(n) = v.parse() {
                cfg.quotas.max_dependency_depth = n;
            }
        }
        cfg
    }

    /// SQLite 数据库文件路径
    pub fn db_path(&self) -> String {
        format!("{}/mox.db", self.data_dir.trim_end_matches('/'))
    }

    /// 解析实际连接串（按 backend 选择）
    /// - sqlite：data_dir/mox.db（或 db_url 若为 file: 形式）
    /// - postgres/mysql：直接使用 db_url（如 postgres://user:pass@host:5432/db）
    pub fn connection_url(&self) -> String {
        match self.backend {
            Backend::Sqlite => {
                if self.db_url.starts_with("file:") || self.db_url.ends_with(".db") {
                    self.db_url.clone()
                } else {
                    self.db_path()
                }
            }
            _ => self.db_url.clone(),
        }
    }

    /// 是否开放跨域（允许列表非空）
    pub fn cors_enabled(&self) -> bool {
        !self.cors_allowed_origins.is_empty()
    }
}
