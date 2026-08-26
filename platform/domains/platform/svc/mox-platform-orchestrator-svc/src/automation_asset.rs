//! # AI 自动化中枢：共享资产模型与持久化
//!
//! 独立模块，避免 `automation`（编排+API）与 `market`（算子商城）形成循环依赖。
//! 本模块只定义数据结构与文件存储（不引用任何其它 mox_platform_orchestrator_svc 子模块），
//! 由 `automation` 与 `main` 单向引用。

use mox_ai_agent_svc::requirement_compiler::SystemBlueprint;
use mox_ai_flow_svc::automation::{AutoTest, RolePermission};
use serde::{Deserialize, Serialize};

/// 自动生成的全栈代码（与 flow-ai 解耦，避免其内部 model 类型耦合）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCode {
    pub python: String,
    pub sql: String,
    pub vue: String,
}

/// 一次运行/修复记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub ts: String,
    pub exit_code: i32,
    pub success: bool,
    pub category: Option<String>,
    pub fixed: bool,
    pub stderr_tail: String,
}

/// 自动化资产（需求 → 蓝图/流程图 + 自动代码 + 测试 + RBAC + 运行历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AutomationAsset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    /// 业务蓝图（功能点 + 实体 + 关联关系）
    pub blueprint: SystemBlueprint,
    /// 自动生成的全栈代码（python/sql/vue）
    pub code: GeneratedCode,
    /// 自动生成的测试
    pub tests: Vec<AutoTest>,
    /// 自动推导的 RBAC
    pub rbac: Vec<RolePermission>,
    /// 运行/修复历史
    pub run_history: Vec<RunRecord>,
    pub created_at: String,
    pub updated_at: String,
}

// ===================== 持久化 =====================

/// 归一化根目录：`$OUS_HOME`（与 market 共用）
fn ous_home() -> std::path::PathBuf {
    let home = std::env::var("OUS_HOME").unwrap_or_else(|_| ".ous".to_string());
    std::path::PathBuf::from(home)
}

/// 自动化资产存储目录：`$OUS_HOME/automation`
fn automation_dir() -> std::path::PathBuf {
    ous_home().join("automation")
}

/// 把任意资产 id 清洗为安全文件名组件（防路径穿越），仅保留字母数字 `.` `_` `-`。
/// 与 `market_migration::sanitize_file_component` 语义一致，避免 `/`、`\`、`..` 逃逸出资产目录。
fn sanitize_asset_id(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn automation_path(id: &str) -> std::path::PathBuf {
    automation_dir().join(format!("{}.json", sanitize_asset_id(id)))
}

/// 保存（创建或覆盖）一份自动化资产
pub fn save_automation(asset: AutomationAsset) -> Result<(), String> {
    let dir = automation_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("创建自动化目录失败: {}", e))?;
    }
    let content =
        serde_json::to_string_pretty(&asset).map_err(|e| format!("序列化自动化资产失败: {}", e))?;
    std::fs::write(automation_path(&asset.id), content)
        .map_err(|e| format!("写入自动化资产失败: {}", e))
}

/// 读取一份自动化资产；不存在返回 None
pub fn get_automation(id: &str) -> Result<Option<AutomationAsset>, String> {
    let path = automation_path(id);
    if !path.exists() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("读取自动化资产失败: {}", e))?;
    let asset: AutomationAsset =
        serde_json::from_str(&content).map_err(|e| format!("解析自动化资产失败: {}", e))?;
    Ok(Some(asset))
}

/// 列出全部自动化资产（按更新时间倒序）
pub fn list_automations() -> Result<Vec<AutomationAsset>, String> {
    let dir = automation_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("枚举自动化目录失败: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(asset) = serde_json::from_str::<AutomationAsset>(&content) {
                    out.push(asset);
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}
