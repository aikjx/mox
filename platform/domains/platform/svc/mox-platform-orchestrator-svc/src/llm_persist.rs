// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! LLM 配置持久化（重启不丢）
//!
//! # 设计
//! - 配置文件：`<cwd>/.runtime/llm-config.json`（运行态目录，与 data/ 同族，gitignore）。
//! - **写盘**：`update_llm_config` 成功后原子落盘（写临时文件再 rename）。
//! - **读回**：启动时先读盘；存在且可解析则覆盖 env 默认；缺失/损坏回退 env（DEEPSEEK_API_KEY）。
//! - **Key 脱敏**：磁盘保留明文 Key（LLM 客户端运行需要）；但 HTTP 响应只回 `has_api_key: bool`，
//!   绝不回传明文（与既有 get_llm_config 行为一致）。
//!
//! # 禁桩
//! 读写均为真实文件 IO；任何 IO/解析错误一律回退（返回 None），不伪造内存态。

use std::fs;
use std::io;
use std::path::PathBuf;
use mox_ai_agent_svc::LLMConfig;

/// 配置文件路径（cwd 相对）
pub fn llm_config_path() -> PathBuf {
    PathBuf::from(".runtime").join("llm-config.json")
}

/// 持久化 LLM 配置（原子写：临时文件 → rename）
pub fn persist_llm_config(config: &LLMConfig) -> io::Result<()> {
    let path = llm_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("序列化 LLM 配置失败: {e}")))?;
    fs::write(&tmp, &bytes)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// 读回持久化配置；缺失/损坏一律返回 None（调用方回退 env 默认）
pub fn load_persisted_llm_config() -> Option<LLMConfig> {
    let path = llm_config_path();
    let bytes = fs::read(&path).ok()?;
    serde_json::from_slice::<LLMConfig>(&bytes).ok()
}

/// 删除持久化文件（测试用/重置用）
#[allow(dead_code)]
pub fn reset_persisted_llm_config() -> io::Result<()> {
    let path = llm_config_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// 供 handler 使用：返回脱敏后的配置视图（不回传 api_key 明文）
pub fn masked_view(config: &LLMConfig) -> serde_json::Value {
    serde_json::json!({
        "api_base": config.api_base,
        "model": config.model,
        "temperature": config.temperature,
        "max_tokens": config.max_tokens,
        "enabled": config.enabled,
        "has_api_key": !config.api_key.is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LLMConfig {
        LLMConfig {
            api_base: "https://api.deepseek.com/v1".to_string(),
            api_key: "sk-test-1234567890".to_string(),
            model: "deepseek-chat".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            enabled: true,
        }
    }

    #[test]
    fn persist_then_load_roundtrip() {
        let cfg = sample();
        persist_llm_config(&cfg).expect("写盘成功");
        let loaded = load_persisted_llm_config().expect("读回成功");
        assert_eq!(loaded.api_base, cfg.api_base);
        assert_eq!(loaded.api_key, cfg.api_key); // 磁盘保留 Key
        assert_eq!(loaded.model, cfg.model);
        assert_eq!(loaded.temperature, cfg.temperature);
        assert_eq!(loaded.max_tokens, cfg.max_tokens);
        assert_eq!(loaded.enabled, cfg.enabled);
        let _ = reset_persisted_llm_config();
    }

    #[test]
    fn masked_view_never_leaks_key() {
        let cfg = sample();
        let v = masked_view(&cfg);
        assert_eq!(v["has_api_key"], serde_json::json!(true));
        // 视图中不得出现 api_key 字段明文
        assert!(v.get("api_key").is_none());
        let s = serde_json::to_string(&v).unwrap();
        assert!(!s.contains("sk-test-1234567890"));
        let _ = reset_persisted_llm_config();
    }

    #[test]
    fn corrupt_file_falls_back_to_none() {
        // 写入损坏 JSON
        let path = llm_config_path();
        if let Some(parent) = path.parent() { fs::create_dir_all(parent).unwrap(); }
        fs::write(&path, "{ this is : not valid json ").unwrap();
        let loaded = load_persisted_llm_config();
        assert!(loaded.is_none(), "损坏文件必须回退 None");
        let _ = reset_persisted_llm_config();
    }

    #[test]
    fn missing_file_returns_none() {
        let _ = reset_persisted_llm_config();
        assert!(load_persisted_llm_config().is_none());
    }
}
