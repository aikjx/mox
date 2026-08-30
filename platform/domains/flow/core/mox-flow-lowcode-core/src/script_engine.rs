// Copyright (c) 2026 璇玑 RelGraph · 低代码核心 (Low-Code Core)
// Licensed under the MIT License.

//! 脚本引擎
//!
//! 支持自定义脚本扩展，提供：
//! - 脚本注册与管理
//! - 钩子（Hook）系统
//! - 脚本上下文管理
//! - 脚本执行记录

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{LowcodeError, LowcodeResult};
use crate::expression::ExpressionEvaluator;
use crate::types::DataType;

/// 钩子类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// 数据创建前
    BeforeCreate,
    /// 数据创建后
    AfterCreate,
    /// 数据更新前
    BeforeUpdate,
    /// 数据更新后
    AfterUpdate,
    /// 数据删除前
    BeforeDelete,
    /// 数据删除后
    AfterDelete,
    /// 数据查询前
    BeforeQuery,
    /// 数据查询后
    AfterQuery,
    /// 表单提交前
    BeforeSubmit,
    /// 表单提交后
    AfterSubmit,
    /// 页面加载
    OnPageLoad,
    /// 字段变化
    OnFieldChange,
    /// 按钮点击
    OnButtonClick,
    /// 定时触发
    OnSchedule,
    /// API 请求前
    BeforeApiCall,
    /// API 响应后
    AfterApiCall,
}

impl HookType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookType::BeforeCreate => "before_create",
            HookType::AfterCreate => "after_create",
            HookType::BeforeUpdate => "before_update",
            HookType::AfterUpdate => "after_update",
            HookType::BeforeDelete => "before_delete",
            HookType::AfterDelete => "after_delete",
            HookType::BeforeQuery => "before_query",
            HookType::AfterQuery => "after_query",
            HookType::BeforeSubmit => "before_submit",
            HookType::AfterSubmit => "after_submit",
            HookType::OnPageLoad => "on_page_load",
            HookType::OnFieldChange => "on_field_change",
            HookType::OnButtonClick => "on_button_click",
            HookType::OnSchedule => "on_schedule",
            HookType::BeforeApiCall => "before_api_call",
            HookType::AfterApiCall => "after_api_call",
        }
    }
}

/// 脚本钩子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptHook {
    /// 钩子 ID
    pub id: String,
    /// 钩子名称
    pub name: String,
    /// 钩子类型
    pub hook_type: HookType,
    /// 关联的实体/页面/表单 ID
    pub target_id: Option<String>,
    /// 关联类型
    pub target_type: Option<String>, // entity, page, form
    /// 脚本 ID
    pub script_id: String,
    /// 执行顺序
    pub order: u32,
    /// 是否启用
    pub enabled: bool,
    /// 描述
    pub description: Option<String>,
}

impl ScriptHook {
    /// 创建脚本钩子
    pub fn new(name: &str, hook_type: HookType, script_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            hook_type,
            target_id: None,
            target_type: None,
            script_id: script_id.to_string(),
            order: 100,
            enabled: true,
            description: None,
        }
    }

    /// 设置目标
    pub fn with_target(mut self, target_type: &str, target_id: &str) -> Self {
        self.target_type = Some(target_type.to_string());
        self.target_id = Some(target_id.to_string());
        self
    }
}

/// 脚本执行上下文
#[derive(Debug, Clone)]
pub struct ScriptContext {
    /// 上下文数据
    pub data: HashMap<String, DataType>,
    /// 环境变量
    pub env: HashMap<String, String>,
    /// 当前用户 ID
    pub user_id: Option<String>,
    /// 当前租户 ID
    pub tenant_id: Option<String>,
    /// 请求 ID（用于追踪）
    pub request_id: Option<String>,
}

impl ScriptContext {
    /// 创建空上下文
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            env: HashMap::new(),
            user_id: None,
            tenant_id: None,
            request_id: None,
        }
    }

    /// 设置数据变量
    pub fn set(&mut self, name: &str, value: DataType) {
        self.data.insert(name.to_string(), value);
    }

    /// 获取数据变量
    pub fn get(&self, name: &str) -> Option<&DataType> {
        self.data.get(name)
    }
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 脚本执行结果
#[derive(Debug, Clone)]
pub struct ScriptExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 返回值
    pub return_value: Option<DataType>,
    /// 错误消息
    pub error_message: Option<String>,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 日志输出
    pub logs: Vec<String>,
}

/// 脚本定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDef {
    /// 脚本 ID
    pub id: String,
    /// 脚本名称
    pub name: String,
    /// 脚本类型（表达式、函数等）
    pub script_type: ScriptType,
    /// 脚本代码
    pub code: String,
    /// 脚本参数定义
    pub parameters: Vec<ScriptParameter>,
    /// 返回值类型
    pub return_type: Option<String>,
    /// 所属模块
    pub module: String,
    /// 是否启用
    pub enabled: bool,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

/// 脚本类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptType {
    /// 表达式脚本（单行表达式）
    Expression,
    /// 函数脚本（多语句）
    Function,
    /// 验证脚本
    Validation,
    /// 转换脚本
    Transform,
    /// 触发器脚本
    Trigger,
}

/// 脚本参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

impl ScriptDef {
    /// 创建表达式脚本
    pub fn expression(name: &str, code: &str, module: &str) -> Self {
        let now = crate::types::now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            script_type: ScriptType::Expression,
            code: code.to_string(),
            parameters: Vec::new(),
            return_type: None,
            module: module.to_string(),
            enabled: true,
            timeout_ms: 5000,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 脚本执行器 trait
#[async_trait::async_trait]
pub trait ScriptExecutor: Send + Sync {
    /// 执行脚本
    async fn execute(
        &self,
        script: &ScriptDef,
        context: &ScriptContext,
    ) -> ScriptExecutionResult;
}

/// 表达式执行器（内置）
pub struct ExpressionExecutor;

#[async_trait::async_trait]
impl ScriptExecutor for ExpressionExecutor {
    async fn execute(
        &self,
        script: &ScriptDef,
        context: &ScriptContext,
    ) -> ScriptExecutionResult {
        let start = crate::types::now_ms();
        let mut result = ScriptExecutionResult {
            success: false,
            return_value: None,
            error_message: None,
            duration_ms: 0,
            logs: Vec::new(),
        };

        match ExpressionEvaluator::evaluate(&script.code, &context.data) {
            Ok(value) => {
                result.success = true;
                result.return_value = Some(value);
            }
            Err(e) => {
                result.error_message = Some(e.to_string());
            }
        }

        result.duration_ms = crate::types::now_ms().saturating_sub(start);
        result
    }
}

/// 脚本引擎
pub struct ScriptEngine {
    /// 脚本表
    scripts: RwLock<HashMap<String, ScriptDef>>,
    /// 钩子表
    hooks: RwLock<HashMap<String, ScriptHook>>,
    /// 类型钩子索引：hook_type:target_type:target_id -> Vec<hook_id>
    hook_index: RwLock<HashMap<String, Vec<String>>>,
    /// 执行器
    executors: RwLock<HashMap<ScriptType, Arc<dyn ScriptExecutor>>>,
    /// 执行历史（简化：只计数）
    execution_count: std::sync::atomic::AtomicU64,
}

impl ScriptEngine {
    /// 创建脚本引擎
    pub fn new() -> Self {
        let engine = Self {
            scripts: RwLock::new(HashMap::new()),
            hooks: RwLock::new(HashMap::new()),
            hook_index: RwLock::new(HashMap::new()),
            executors: RwLock::new(HashMap::new()),
            execution_count: std::sync::atomic::AtomicU64::new(0),
        };

        // 注册内置执行器
        engine.register_executor(
            ScriptType::Expression,
            Arc::new(ExpressionExecutor),
        );

        engine
    }

    /// 注册脚本执行器
    pub fn register_executor(
        &self,
        script_type: ScriptType,
        executor: Arc<dyn ScriptExecutor>,
    ) {
        self.executors.write().insert(script_type, executor);
    }

    // ---------- 脚本管理 ----------

    /// 注册脚本
    pub fn register_script(&self, script: ScriptDef) -> ScriptDef {
        self.scripts
            .write()
            .insert(script.id.clone(), script.clone());
        script
    }

    /// 获取脚本
    pub fn get_script(&self, script_id: &str) -> LowcodeResult<ScriptDef> {
        self.scripts
            .read()
            .get(script_id)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("script '{}' not found", script_id)))
    }

    /// 按名称获取脚本
    pub fn get_script_by_name(&self, name: &str) -> Option<ScriptDef> {
        self.scripts
            .read()
            .values()
            .find(|s| s.name == name)
            .cloned()
    }

    /// 更新脚本
    pub fn update_script(
        &self,
        script_id: &str,
        mut update: ScriptDef,
    ) -> LowcodeResult<ScriptDef> {
        let mut scripts = self.scripts.write();
        let existing = scripts
            .get_mut(script_id)
            .ok_or_else(|| LowcodeError::NotFound(format!("script '{}' not found", script_id)))?;

        update.id = script_id.to_string();
        update.created_at = existing.created_at;
        update.updated_at = crate::types::now_ms();

        *existing = update.clone();
        Ok(update)
    }

    /// 删除脚本
    pub fn delete_script(&self, script_id: &str) -> LowcodeResult<bool> {
        // 检查是否有钩子引用
        for hook in self.hooks.read().values() {
            if hook.script_id == script_id {
                return Err(LowcodeError::InvalidConfig(
                    "cannot delete script used by hooks".to_string(),
                ));
            }
        }

        Ok(self.scripts.write().remove(script_id).is_some())
    }

    /// 列出模块脚本
    pub fn list_scripts_by_module(&self, module: &str) -> Vec<ScriptDef> {
        self.scripts
            .read()
            .values()
            .filter(|s| s.module == module)
            .cloned()
            .collect()
    }

    // ---------- 钩子管理 ----------

    /// 注册钩子
    pub fn register_hook(&self, hook: ScriptHook) -> ScriptHook {
        // 加入索引
        let key = hook_index_key(hook.hook_type, hook.target_type.as_deref(), hook.target_id.as_deref());
        self.hook_index
            .write()
            .entry(key)
            .or_default()
            .push(hook.id.clone());

        self.hooks.write().insert(hook.id.clone(), hook.clone());
        hook
    }

    /// 获取钩子
    pub fn get_hook(&self, hook_id: &str) -> LowcodeResult<ScriptHook> {
        self.hooks
            .read()
            .get(hook_id)
            .cloned()
            .ok_or_else(|| LowcodeError::NotFound(format!("hook '{}' not found", hook_id)))
    }

    /// 删除钩子
    pub fn delete_hook(&self, hook_id: &str) -> LowcodeResult<bool> {
        let hook = self.get_hook(hook_id)?;

        // 从索引中移除
        let key = hook_index_key(hook.hook_type, hook.target_type.as_deref(), hook.target_id.as_deref());
        if let Some(vec) = self.hook_index.write().get_mut(&key) {
            vec.retain(|id| id != hook_id);
        }

        Ok(self.hooks.write().remove(hook_id).is_some())
    }

    /// 获取特定钩子列表
    pub fn get_hooks(
        &self,
        hook_type: HookType,
        target_type: Option<&str>,
        target_id: Option<&str>,
    ) -> Vec<ScriptHook> {
        let key = hook_index_key(hook_type, target_type, target_id);
        let hook_ids = self
            .hook_index
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_default();

        let hooks = self.hooks.read();
        let mut result: Vec<ScriptHook> = hook_ids
            .into_iter()
            .filter_map(|id| hooks.get(&id).cloned())
            .filter(|h| h.enabled)
            .collect();

        result.sort_by_key(|h| h.order);
        result
    }

    // ---------- 执行 ----------

    /// 执行脚本
    pub async fn execute(
        &self,
        script_id: &str,
        context: &ScriptContext,
    ) -> LowcodeResult<ScriptExecutionResult> {
        let script = self.get_script(script_id)?;

        if !script.enabled {
            return Err(LowcodeError::ScriptError("script is disabled".to_string()));
        }

        let executor = self
            .executors
            .read()
            .get(&script.script_type)
            .cloned()
            .ok_or_else(|| {
                LowcodeError::ScriptError(format!(
                    "no executor for script type '{:?}'",
                    script.script_type
                ))
            })?;

        self.execution_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let result = executor.execute(&script, context).await;
        Ok(result)
    }

    /// 按名称执行脚本
    pub async fn execute_by_name(
        &self,
        name: &str,
        context: &ScriptContext,
    ) -> LowcodeResult<ScriptExecutionResult> {
        let script = self
            .get_script_by_name(name)
            .ok_or_else(|| LowcodeError::NotFound(format!("script '{}' not found", name)))?;
        self.execute(&script.id, context).await
    }

    /// 触发钩子
    pub async fn trigger_hook(
        &self,
        hook_type: HookType,
        target_type: Option<&str>,
        target_id: Option<&str>,
        context: &mut ScriptContext,
    ) -> LowcodeResult<Vec<ScriptExecutionResult>> {
        let hooks = self.get_hooks(hook_type, target_type, target_id);
        let mut results = Vec::new();

        for hook in &hooks {
            let result = self.execute(&hook.script_id, context).await?;
            if !result.success {
                // 钩子失败时停止执行
                results.push(result);
                break;
            }
            results.push(result);
        }

        Ok(results)
    }

    /// 脚本总数
    pub fn script_count(&self) -> usize {
        self.scripts.read().len()
    }

    /// 钩子总数
    pub fn hook_count(&self) -> usize {
        self.hooks.read().len()
    }

    /// 执行总次数
    pub fn total_executions(&self) -> u64 {
        self.execution_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成钩子索引键
fn hook_index_key(hook_type: HookType, target_type: Option<&str>, target_id: Option<&str>) -> String {
    format!(
        "{}:{}:{}",
        hook_type.as_str(),
        target_type.unwrap_or("*"),
        target_id.unwrap_or("*")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_context() {
        let mut ctx = ScriptContext::new();
        ctx.set("name", DataType::String("Alice".to_string()));
        ctx.set("age", DataType::Integer(25));

        assert_eq!(ctx.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(ctx.get("age").unwrap().as_integer(), Some(25));
        assert!(ctx.get("nonexist").is_none());
    }

    #[tokio::test]
    async fn test_expression_script() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression(
            "calc_total",
            "price * quantity",
            "business",
        );
        engine.register_script(script.clone());

        let mut ctx = ScriptContext::new();
        ctx.set("price", DataType::Integer(100));
        ctx.set("quantity", DataType::Integer(3));

        let result = engine.execute(&script.id, &ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.return_value.unwrap().as_integer(), Some(300));
    }

    #[tokio::test]
    async fn test_execute_by_name() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("greet", "\"Hello, \" + name", "test");
        engine.register_script(script);

        let mut ctx = ScriptContext::new();
        ctx.set("name", DataType::String("World".to_string()));

        let result = engine.execute_by_name("greet", &ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(
            result.return_value.unwrap().as_str(),
            Some("Hello, World")
        );
    }

    #[test]
    fn test_register_and_get_script() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("test", "1 + 1", "test");
        let id = script.id.clone();

        engine.register_script(script);
        assert_eq!(engine.script_count(), 1);

        let got = engine.get_script(&id).unwrap();
        assert_eq!(got.name, "test");
    }

    #[test]
    fn test_delete_script_with_hooks_fails() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("test", "1", "test");
        let script = engine.register_script(script);

        let hook = ScriptHook::new("test_hook", HookType::BeforeCreate, &script.id);
        engine.register_hook(hook);

        let result = engine.delete_script(&script.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_hook_registration() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("validate", "value > 0", "test");
        let script = engine.register_script(script);

        let hook = ScriptHook::new("validate_before_create", HookType::BeforeCreate, &script.id)
            .with_target("entity", "user");
        engine.register_hook(hook);

        assert_eq!(engine.hook_count(), 1);

        let hooks = engine.get_hooks(HookType::BeforeCreate, Some("entity"), Some("user"));
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].name, "validate_before_create");
    }

    #[test]
    fn test_hook_ordering() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("noop", "1", "test");
        let script = engine.register_script(script);

        let mut hook1 = ScriptHook::new("first", HookType::BeforeCreate, &script.id);
        hook1.order = 10;
        engine.register_hook(hook1);

        let mut hook2 = ScriptHook::new("second", HookType::BeforeCreate, &script.id);
        hook2.order = 5;
        engine.register_hook(hook2);

        let hooks = engine.get_hooks(HookType::BeforeCreate, None, None);
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].order, 5); // 按 order 排序
        assert_eq!(hooks[1].order, 10);
    }

    #[tokio::test]
    async fn test_trigger_hook() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("double", "value * 2", "test");
        let script = engine.register_script(script);

        let hook = ScriptHook::new("double_hook", HookType::BeforeCreate, &script.id);
        engine.register_hook(hook);

        let mut ctx = ScriptContext::new();
        ctx.set("value", DataType::Integer(21));

        let results = engine
            .trigger_hook(HookType::BeforeCreate, None, None, &mut ctx)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].return_value.as_ref().unwrap().as_integer(), Some(42));
    }

    #[test]
    fn test_disabled_hook_not_triggered() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("noop", "1", "test");
        let script = engine.register_script(script);

        let mut hook = ScriptHook::new("disabled_hook", HookType::BeforeCreate, &script.id);
        hook.enabled = false;
        engine.register_hook(hook);

        let hooks = engine.get_hooks(HookType::BeforeCreate, None, None);
        assert_eq!(hooks.len(), 0);
    }

    #[test]
    fn test_list_scripts_by_module() {
        let engine = ScriptEngine::new();
        engine.register_script(ScriptDef::expression("a", "1", "mod1"));
        engine.register_script(ScriptDef::expression("b", "2", "mod1"));
        engine.register_script(ScriptDef::expression("c", "3", "mod2"));

        assert_eq!(engine.list_scripts_by_module("mod1").len(), 2);
        assert_eq!(engine.list_scripts_by_module("mod2").len(), 1);
    }

    #[tokio::test]
    async fn test_execution_count() {
        let engine = ScriptEngine::new();
        let script = ScriptDef::expression("test", "1 + 1", "test");
        engine.register_script(script.clone());

        let ctx = ScriptContext::new();
        engine.execute(&script.id, &ctx).await.unwrap();
        engine.execute(&script.id, &ctx).await.unwrap();

        assert_eq!(engine.total_executions(), 2);
    }
}
