// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 企业级运行时配置
//!
//! 采用 12-factor 配置原则：
//! - 默认值内置于代码；
//! - 可选配置文件（JSON）覆盖默认值；
//! - 环境变量（`OUS_FUSION_*`）最后覆盖（密钥类如 `auth_token` 仅走环境变量，不落盘）。
//!
//! 配置项：
//! - `bind_addr`：HTTP 监听地址，如 `0.0.0.0:8080`
//! - `persistence_path`：六维注册表 JSON 落盘路径（企业级跨重启复用）；空=仅内存
//! - `docs_dir`：PT-DOC 标准文档导出目录
//! - `log_level`：`trace`/`debug`/`info`/`warn`/`error`
//! - `auth_token`：Bearer 令牌；`None`=关闭鉴权（仅限内网/测试）
//! - `access_log`：是否开启请求访问日志
//! - `json_log`：是否输出结构化(JSON)日志（对接 ELK/Loki 等）

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP 监听地址
    pub bind_addr: String,
    /// 六维注册表持久化路径（JSON）；`None` 表示仅内存运行
    #[serde(default)]
    pub persistence_path: Option<PathBuf>,
    /// PT-DOC 导出目录
    #[serde(default = "default_docs_dir")]
    pub docs_dir: PathBuf,
    /// 日志级别
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Bearer 鉴权令牌；`None` 关闭鉴权
    #[serde(default)]
    pub auth_token: Option<String>,
    /// 访问日志
    #[serde(default = "default_true")]
    pub access_log: bool,
    /// 结构化(JSON)日志
    #[serde(default)]
    pub json_log: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            bind_addr: "0.0.0.0:8080".into(),
            persistence_path: None,
            docs_dir: default_docs_dir(),
            log_level: default_log_level(),
            auth_token: None,
            access_log: true,
            json_log: false,
        }
    }
}

fn default_docs_dir() -> PathBuf {
    PathBuf::from("data/fusion_docs")
}

fn default_log_level() -> String {
    "info".into()
}

fn default_true() -> bool {
    true
}

impl Config {
    /// 加载配置：先 `path`（可选 JSON 文件）覆盖默认值，再环境变量覆盖。
    ///
    /// 环境变量：
    /// - `OUS_FUSION_BIND_ADDR`
    /// - `OUS_FUSION_PERSISTENCE_PATH`
    /// - `OUS_FUSION_DOCS_DIR`
    /// - `OUS_FUSION_LOG_LEVEL`
    /// - `OUS_FUSION_AUTH_TOKEN`（密钥，建议仅通过此注入）
    /// - `OUS_FUSION_ACCESS_LOG`（`true`/`false`）
    /// - `OUS_FUSION_JSON_LOG`
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let mut cfg = Config::default();

        if let Some(p) = path {
            let txt = std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("读取配置文件 {p} 失败：{e}"))?;
            cfg = serde_json::from_str(&txt)
                .map_err(|e| anyhow::anyhow!("解析配置文件 {p} 失败：{e}"))?;
        }

        if let Ok(v) = std::env::var("OUS_FUSION_BIND_ADDR") {
            cfg.bind_addr = v;
        }
        if let Ok(v) = std::env::var("OUS_FUSION_PERSISTENCE_PATH") {
            cfg.persistence_path = Some(PathBuf::from(v));
        }
        if let Ok(v) = std::env::var("OUS_FUSION_DOCS_DIR") {
            cfg.docs_dir = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("OUS_FUSION_LOG_LEVEL") {
            cfg.log_level = v;
        }
        if let Ok(v) = std::env::var("OUS_FUSION_AUTH_TOKEN") {
            cfg.auth_token = Some(v);
        }
        if let Ok(v) = std::env::var("OUS_FUSION_ACCESS_LOG") {
            cfg.access_log = matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes");
        }
        if let Ok(v) = std::env::var("OUS_FUSION_JSON_LOG") {
            cfg.json_log = matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes");
        }

        cfg.validate()?;
        Ok(cfg)
    }

    /// 校验配置合法性（启动早期失败优于运行时崩溃）
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.bind_addr.is_empty() {
            anyhow::bail!("bind_addr 不能为空");
        }
        let lvl = self.log_level.as_str();
        if !matches!(lvl, "trace" | "debug" | "info" | "warn" | "error") {
            anyhow::bail!("log_level 非法（应为 trace/debug/info/warn/error）：{lvl}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn env_overrides_apply() {
        std::env::set_var("OUS_FUSION_BIND_ADDR", "127.0.0.1:9090");
        std::env::set_var("OUS_FUSION_LOG_LEVEL", "debug");
        let cfg = Config::load(None).unwrap();
        assert_eq!(cfg.bind_addr, "127.0.0.1:9090");
        assert_eq!(cfg.log_level, "debug");
        std::env::remove_var("OUS_FUSION_BIND_ADDR");
        std::env::remove_var("OUS_FUSION_LOG_LEVEL");
    }

    #[test]
    fn invalid_log_level_rejected() {
        let cfg = Config {
            log_level: "verbose".into(),
            ..Config::default()
        };
        assert!(cfg.validate().is_err());
    }
}
