//! MOX 动态业务流程执行器。
//!
//! 这是所有业务系统复用的最小运行时：系统只需提供数据库表、SQL 定义和流程定义，
//! 即可获得参数化执行、条件分支、结果回填、版本发布和审计能力。
//!
//! 安全边界：流程定义只引用已发布的 SQL code，不执行数据库中的任意 Rust/脚本代码；
//! 上下文只通过绑定参数进入 SQL，禁止把业务输入拼接进 SQL 文本。

use crate::error::{DsqlError, DsqlResult};
use crate::model::{ExecuteProcessRequest, ExecuteProcessResult, ProcessStep, ProcessStepResult};
use crate::DsqlManager;
use serde_json::{Map, Value};
use std::time::Instant;

/// 声明式动态流程执行器。
pub struct ProcessEngine<'a> {
    manager: &'a DsqlManager,
}

impl<'a> ProcessEngine<'a> {
    pub fn new(manager: &'a DsqlManager) -> Self {
        Self { manager }
    }

    /// 执行已发布流程。
    pub fn execute(&self, request: &ExecuteProcessRequest) -> DsqlResult<ExecuteProcessResult> {
        let process = self.manager.storage().get_active_process(&request.process_code)?;
        let mut context = request.context.clone();
        if !context.is_object() {
            return Err(DsqlError::InvalidParam("process context must be an object".to_string()));
        }

        let started = Instant::now();
        let mut step_results = Vec::with_capacity(process.steps.len());
        let mut success = true;
        let mut error = None;
        // 记录已成功执行且有补偿SQL的步骤，用于事务回滚
        let mut completed_steps: Vec<(usize, &ProcessStep)> = Vec::new();

        for (_idx, step) in process.steps.iter().enumerate() {
            if !evaluate_condition(step.when.as_deref(), &context)? {
                step_results.push(ProcessStepResult {
                    step_code: step.step_code.clone(),
                    executed: false,
                    skipped: true,
                    success: true,
                    compensated: false,
                    output_key: step.output_key.clone(),
                    data: None,
                    error: None,
                });
                continue;
            }

            let params = resolve_params(step, &context)?;
            let execution = self.manager.execute(&crate::model::ExecuteRequest {
                sql_code: step.sql_code.clone(),
                params,
                trace_id: request.trace_id.clone(),
            });

            match execution {
                Ok(result) if result.success => {
                    if let Some(key) = &step.output_key {
                        set_path(&mut context, key, result.data.clone().unwrap_or(Value::Null))?;
                    }
                    // 记录已成功步骤（用于补偿）
                    if step.compensation_sql_code.is_some() {
                        completed_steps.push((step_results.len(), step));
                    }
                    step_results.push(ProcessStepResult {
                        step_code: step.step_code.clone(),
                        executed: true,
                        skipped: false,
                        success: true,
                        compensated: false,
                        output_key: step.output_key.clone(),
                        data: result.data,
                        error: None,
                    });
                }
                Ok(result) => {
                    let message = result.error.unwrap_or_else(|| "SQL execution failed".to_string());
                    let should_stop = !step.continue_on_error;
                    step_results.push(ProcessStepResult {
                        step_code: step.step_code.clone(),
                        executed: true,
                        skipped: false,
                        success: false,
                        compensated: false,
                        output_key: step.output_key.clone(),
                        data: result.data,
                        error: Some(message.clone()),
                    });
                    success = false;
                    error = Some(message);
                    // 事务模式：执行补偿
                    if should_stop && process.transactional {
                        self.execute_compensation(&completed_steps, &context, request.trace_id.as_deref(), &mut step_results);
                    }
                    if should_stop {
                        break;
                    }
                }
                Err(err) => {
                    let message = err.to_string();
                    step_results.push(ProcessStepResult {
                        step_code: step.step_code.clone(),
                        executed: true,
                        skipped: false,
                        success: false,
                        compensated: false,
                        output_key: step.output_key.clone(),
                        data: None,
                        error: Some(message.clone()),
                    });
                    success = false;
                    error = Some(message);
                    // 事务模式：执行补偿
                    if !step.continue_on_error && process.transactional {
                        self.execute_compensation(&completed_steps, &context, request.trace_id.as_deref(), &mut step_results);
                    }
                    if !step.continue_on_error {
                        break;
                    }
                }
            }
        }

        let duration_ms = started.elapsed().as_millis() as u64;
        let result = ExecuteProcessResult {
            process_code: process.process_code,
            success,
            context,
            steps: step_results,
            duration_ms,
            trace_id: request.trace_id.clone(),
            error,
        };
        let step_json = serde_json::to_string(&result.steps)
            .map_err(|e| DsqlError::Internal(format!("serialize process audit: {e}")))?;
        self.manager.storage().write_process_audit(
            request.trace_id.as_deref(),
            &result.process_code,
            result.success,
            result.duration_ms,
            &step_json,
            result.error.as_deref(),
        )?;
        Ok(result)
    }

    /// 执行补偿操作：按逆序执行已成功步骤的补偿SQL
    fn execute_compensation(
        &self,
        completed_steps: &[(usize, &ProcessStep)],
        context: &Value,
        trace_id: Option<&str>,
        step_results: &mut Vec<ProcessStepResult>,
    ) {
        tracing::warn!("Process transaction failed, executing compensation for {} steps", completed_steps.len());
        // 按逆序执行补偿
        for (result_idx, step) in completed_steps.iter().rev() {
            if let Some(comp_sql_code) = &step.compensation_sql_code {
                let params = match resolve_params(step, context) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Compensation param resolution failed for step {}: {}", step.step_code, e);
                        continue;
                    }
                };
                match self.manager.execute(&crate::model::ExecuteRequest {
                    sql_code: comp_sql_code.clone(),
                    params,
                    trace_id: trace_id.map(|s| s.to_string()),
                }) {
                    Ok(_) => {
                        tracing::info!("Compensation succeeded for step: {}", step.step_code);
                        if let Some(sr) = step_results.get_mut(*result_idx) {
                            sr.compensated = true;
                        }
                    }
                    Err(e) => {
                        // 补偿失败只记录日志，不中断补偿流程
                        tracing::error!("Compensation failed for step {}: {}", step.step_code, e);
                    }
                }
            }
        }
    }
}

fn resolve_params(step: &ProcessStep, context: &Value) -> DsqlResult<Value> {
    let mut params = Map::new();
    if step.input_mapping.is_empty() {
        if let Some(object) = context.as_object() {
            params.extend(object.clone());
        }
    } else {
        for (param, path) in &step.input_mapping {
            let value = resolve_path(context, path).cloned().unwrap_or(Value::Null);
            params.insert(param.clone(), value);
        }
    }
    Ok(Value::Object(params))
}

fn evaluate_condition(condition: Option<&str>, context: &Value) -> DsqlResult<bool> {
    let Some(condition) = condition.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(true);
    };
    if let Some(inner) = condition.strip_prefix("exists(").and_then(|v| v.strip_suffix(')')) {
        return Ok(resolve_path(context, inner.trim()).is_some());
    }
    for operator in ["==", "!="] {
        if let Some((left, right)) = condition.split_once(operator) {
            let actual = resolve_path(context, left.trim()).cloned().unwrap_or(Value::Null);
            let expected = parse_literal(right.trim());
            return Ok(if operator == "==" { actual == expected } else { actual != expected });
        }
    }
    Err(DsqlError::InvalidParam(format!("unsupported process condition: {condition}")))
}

fn parse_literal(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.trim_matches('\'').to_string()))
}

fn resolve_path<'a>(context: &'a Value, raw_path: &str) -> Option<&'a Value> {
    let trimmed = raw_path.trim();
    let path = trimmed
        .strip_prefix("$.")
        .or_else(|| trimmed.strip_prefix('$'))
        .unwrap_or(trimmed)
        .trim_start_matches('.');
    if path.is_empty() {
        return Some(context);
    }
    path.split('.').try_fold(context, |value, key| value.get(key))
}

fn set_path(context: &mut Value, raw_path: &str, value: Value) -> DsqlResult<()> {
    let trimmed = raw_path.trim();
    let path = trimmed
        .strip_prefix("$.")
        .or_else(|| trimmed.strip_prefix('$'))
        .unwrap_or(trimmed)
        .trim_start_matches('.');
    let Some(object) = context.as_object_mut() else {
        return Err(DsqlError::InvalidParam("process context must be an object".to_string()));
    };
    if path.is_empty() || path.contains('.') {
        return Err(DsqlError::InvalidParam("output_key must be a top-level context key".to_string()));
    }
    object.insert(path.to_string(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CreateProcessRequest, CreateSqlRequest, OperationType, ParamDef, ResultType};
    use std::collections::HashMap;

    #[test]
    fn dynamic_process_executes_sql_and_returns_context() {
        let manager = DsqlManager::open_memory().unwrap();
        manager.execute_ddl("CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
        manager.execute_ddl("INSERT INTO people (name) VALUES ('Ada'), ('Linus')").unwrap();
        manager.create_sql(&CreateSqlRequest {
            sql_code: "people.list".to_string(),
            sql_name: "list people".to_string(),
            description: None,
            datasource_code: "default".to_string(),
            sql_template: "SELECT id, name FROM people WHERE name LIKE {{keyword}} ORDER BY id".to_string(),
            param_defs: vec![ParamDef {
                name: "keyword".to_string(),
                data_type: "STRING".to_string(),
                required: true,
                default_value: None,
                description: None,
                validation: None,
            }],
            result_type: ResultType::List,
            operation_type: OperationType::Read,
            cache_enabled: Some(false),
            cache_ttl: None,
            permission_code: None,
            entity_code: Some("people".to_string()),
            created_by: Some("test".to_string()),
        }).unwrap();
        manager.activate_sql("people.list").unwrap();

        let mut mapping = HashMap::new();
        mapping.insert("keyword".to_string(), "$.keyword".to_string());
        manager.create_process(&CreateProcessRequest {
            process_code: "people.search".to_string(),
            process_name: "search people".to_string(),
            description: None,
            steps: vec![ProcessStep {
                step_code: "find".to_string(),
                name: "find".to_string(),
                sql_code: "people.list".to_string(),
                input_mapping: mapping,
                output_key: Some("matches".to_string()),
                when: None,
                continue_on_error: false,
                compensation_sql_code: None,
            }],
            permission_code: None,
            entity_code: Some("people".to_string()),
            created_by: Some("test".to_string()),
        }).unwrap();
        manager.activate_process("people.search").unwrap();

        let result = ProcessEngine::new(&manager).execute(&ExecuteProcessRequest {
            process_code: "people.search".to_string(),
            context: serde_json::json!({"keyword": "%Ada%"}),
            trace_id: Some("trace-process-1".to_string()),
        }).unwrap();
        assert!(result.success);
        assert_eq!(result.context["matches"].as_array().unwrap().len(), 1);
        assert_eq!(result.steps[0].executed, true);
    }

    #[test]
    fn conditions_skip_without_running_sql() {
        let context = serde_json::json!({"approved": false});
        assert!(!evaluate_condition(Some("$.approved == true"), &context).unwrap());
        assert!(evaluate_condition(Some("exists($.approved)"), &context).unwrap());
    }
}
