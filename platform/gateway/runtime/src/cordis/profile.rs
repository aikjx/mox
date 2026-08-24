//! Profile 配置加载

use serde::{Deserialize, Serialize};

/// Profile加载器
pub struct ProfileLoader {
    cache: std::collections::HashMap<String, Profile>,
}

impl ProfileLoader {
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// 加载Profile
    pub async fn load(&mut self, path: &str) -> Result<Profile, ProfileError> {
        // 检查缓存
        if let Some(profile) = self.cache.get(path) {
            return Ok(profile.clone());
        }

        // 从文件加载
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ProfileError::LoadError(format!("Failed to read {}: {}", path, e)))?;

        let profile: Profile = serde_yaml::from_str(&content)
            .map_err(|e| ProfileError::ParseError(format!("YAML parse error: {}", e)))?;

        // 缓存
        self.cache.insert(path.to_string(), profile.clone());

        Ok(profile)
    }
}

impl Default for ProfileLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Profile配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub llm: Option<LlmConfig>,
    pub tools: Vec<ToolConfig>,
    pub agents: Vec<AgentConfig>,
    pub environment: Option<std::collections::HashMap<String, String>>,
}

/// LLM配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub handler: String,
}

/// Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Profile错误
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Load error: {0}")]
    LoadError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_profile() {
        let yaml = r#"
name: test-profile
version: "1.0"
description: Test profile
llm:
  provider: openai
  model: gpt-4
tools:
  - name: test-tool
    description: A test tool
    input_schema: {}
    handler: test_handler
agents:
  - name: test-agent
    system_prompt: "You are a test agent"
    tools:
      - test-tool
"#;

        let profile: Profile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.name, "test-profile");
        assert_eq!(profile.tools.len(), 1);
        assert_eq!(profile.agents.len(), 1);
    }
}
