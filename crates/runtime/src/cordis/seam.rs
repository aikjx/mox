//! 能力接缝（Seam）
//!
//! 外部世界（文件系统、子进程、LLM、数据库）的抽象层

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Seam注册表
pub struct SeamRegistry {
    seams: RwLock<HashMap<String, Arc<dyn Seam>>>,
}

impl SeamRegistry {
    pub fn new() -> Self {
        Self {
            seams: RwLock::new(HashMap::new()),
        }
    }

    /// 注册Seam
    pub fn register(&self, name: String, seam: Arc<dyn Seam>) {
        let mut seams = self.seams.write();
        seams.insert(name, seam);
    }

    /// 获取Seam
    pub fn get(&self, name: &str) -> Option<Arc<dyn Seam>> {
        let seams = self.seams.read();
        seams.get(name).cloned()
    }

    /// 获取能力
    pub fn get_capability(&self, name: &str, capability: &str) -> Option<SeamCapability> {
        let seams = self.seams.read();
        seams.get(name)?.get_capability(capability)
    }
}

impl Default for SeamRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Seam能力接缝
#[async_trait]
pub trait Seam: Send + Sync {
    /// Seam名称
    fn name(&self) -> &str;

    /// Seam描述
    fn description(&self) -> &str;

    /// 获取能力
    fn get_capability(&self, name: &str) -> Option<SeamCapability>;

    /// 列出所有能力
    fn list_capabilities(&self) -> Vec<SeamCapability>;
}

/// Seam能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamCapability {
    pub name: String,
    pub description: String,
    pub input_type: String,
    pub output_type: String,
    pub requires_token: bool,
}

/// 文件系统Seam
pub struct FileSystemSeam;

#[async_trait]
impl Seam for FileSystemSeam {
    fn name(&self) -> &str {
        "fs"
    }

    fn description(&self) -> &str {
        "File system operations"
    }

    fn get_capability(&self, name: &str) -> Option<SeamCapability> {
        match name {
            "read" => Some(SeamCapability {
                name: "read".to_string(),
                description: "Read file".to_string(),
                input_type: "path".to_string(),
                output_type: "bytes".to_string(),
                requires_token: true,
            }),
            "write" => Some(SeamCapability {
                name: "write".to_string(),
                description: "Write file".to_string(),
                input_type: "path, bytes".to_string(),
                output_type: "unit".to_string(),
                requires_token: true,
            }),
            _ => None,
        }
    }

    fn list_capabilities(&self) -> Vec<SeamCapability> {
        vec![
            self.get_capability("read").unwrap(),
            self.get_capability("write").unwrap(),
        ]
    }
}

/// 子进程Seam
pub struct SubprocessSeam;

#[async_trait]
impl Seam for SubprocessSeam {
    fn name(&self) -> &str {
        "subprocess"
    }

    fn description(&self) -> &str {
        "Subprocess execution"
    }

    fn get_capability(&self, name: &str) -> Option<SeamCapability> {
        match name {
            "spawn" => Some(SeamCapability {
                name: "spawn".to_string(),
                description: "Spawn subprocess".to_string(),
                input_type: "command, args".to_string(),
                output_type: "pid, exit_code".to_string(),
                requires_token: true,
            }),
            _ => None,
        }
    }

    fn list_capabilities(&self) -> Vec<SeamCapability> {
        vec![self.get_capability("spawn").unwrap()]
    }
}

/// LLM Seam
pub struct LlmSeam;

#[async_trait]
impl Seam for LlmSeam {
    fn name(&self) -> &str {
        "llm"
    }

    fn description(&self) -> &str {
        "Large Language Model integration"
    }

    fn get_capability(&self, name: &str) -> Option<SeamCapability> {
        match name {
            "complete" => Some(SeamCapability {
                name: "complete".to_string(),
                description: "Text completion".to_string(),
                input_type: "prompt, options".to_string(),
                output_type: "completion".to_string(),
                requires_token: true,
            }),
            "embed" => Some(SeamCapability {
                name: "embed".to_string(),
                description: "Text embedding".to_string(),
                input_type: "text".to_string(),
                output_type: "vector".to_string(),
                requires_token: true,
            }),
            _ => None,
        }
    }

    fn list_capabilities(&self) -> Vec<SeamCapability> {
        vec![
            self.get_capability("complete").unwrap(),
            self.get_capability("embed").unwrap(),
        ]
    }
}

/// Seam错误
#[derive(Debug, thiserror::Error)]
pub enum SeamError {
    #[error("Seam not found: {0}")]
    NotFound(String),

    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seam_registry() {
        let registry = SeamRegistry::new();
        registry.register("fs".to_string(), Arc::new(FileSystemSeam));

        let fs = registry.get("fs").unwrap();
        assert_eq!(fs.name(), "fs");
        assert_eq!(fs.list_capabilities().len(), 2);
    }
}
