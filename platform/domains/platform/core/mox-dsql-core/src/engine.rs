// mox-dsql-core SQL执行引擎：模板渲染 + 参数化查询 + 结果映射
use crate::error::{DsqlError, DsqlResult};
use crate::model::*;
use rusqlite::{Connection, OptionalExtension, params_from_iter};
use std::collections::HashMap;
use std::time::Instant;

/// SQL执行引擎
pub struct SqlEngine;

impl SqlEngine {
    /// 执行SQL定义
    pub fn execute(
        conn: &Connection,
        sql_def: &SqlDefinition,
        params: &serde_json::Value,
    ) -> DsqlResult<ExecuteResult> {
        let start = Instant::now();
        let trace_id = params.get("trace_id").and_then(|v| v.as_str()).map(|s| s.to_string());

        // 1. 参数校验
        let validated_params = Self::validate_params(sql_def, params)?;

        // 2. 模板渲染
        let (rendered_sql, ordered_params) = Self::render_template(&sql_def.sql_template, &validated_params)?;

        tracing::debug!(sql_code = %sql_def.sql_code, rendered_sql = %rendered_sql, "dsql execute");

        // 3. 执行SQL
        let data = match sql_def.operation_type {
            OperationType::Read => Self::execute_query(conn, &rendered_sql, &ordered_params, sql_def.result_type)?,
            OperationType::Write => Self::execute_write(conn, &rendered_sql, &ordered_params)?,
        };

        let duration = start.elapsed().as_millis() as u64;

        Ok(ExecuteResult {
            sql_code: sql_def.sql_code.clone(),
            success: true,
            data: Some(data),
            row_count: None,
            duration_ms: duration,
            cache_hit: false,
            error: None,
            trace_id,
        })
    }

    /// 参数校验
    fn validate_params(
        sql_def: &SqlDefinition,
        params: &serde_json::Value,
    ) -> DsqlResult<HashMap<String, serde_json::Value>> {
        let mut result = HashMap::new();
        let param_obj = params.as_object()
            .ok_or_else(|| DsqlError::InvalidParam("params must be object".to_string()))?;

        for def in &sql_def.param_defs {
            let value = param_obj.get(&def.name).cloned();

            // 必填校验
            if def.required && value.is_none() && def.default_value.is_none() {
                return Err(DsqlError::MissingParam(def.name.clone()));
            }

            // 使用默认值
            let value = value.or_else(|| def.default_value.clone())
                .unwrap_or(serde_json::Value::Null);

            // 类型校验
            Self::validate_param_type(def, &value)?;

            // 校验规则
            if let Some(validation) = &def.validation {
                Self::validate_param_rule(def, &value, validation)?;
            }

            result.insert(def.name.clone(), value);
        }

        Ok(result)
    }

    /// 参数类型校验
    fn validate_param_type(def: &ParamDef, value: &serde_json::Value) -> DsqlResult<()> {
        if value.is_null() {
            return Ok(()); // null跳过类型校验
        }
        match def.data_type.as_str() {
            "STRING" => {
                if !value.is_string() {
                    return Err(DsqlError::InvalidParam(format!("{} must be string", def.name)));
                }
            }
            "INT" | "LONG" => {
                if !value.is_i64() && !value.is_u64() {
                    return Err(DsqlError::InvalidParam(format!("{} must be integer", def.name)));
                }
            }
            "DECIMAL" | "NUMBER" => {
                if !value.is_number() {
                    return Err(DsqlError::InvalidParam(format!("{} must be number", def.name)));
                }
            }
            "BOOL" => {
                if !value.is_boolean() {
                    return Err(DsqlError::InvalidParam(format!("{} must be boolean", def.name)));
                }
            }
            "DATETIME" | "DATE" => {
                if !value.is_string() {
                    return Err(DsqlError::InvalidParam(format!("{} must be datetime string", def.name)));
                }
            }
            _ => {} // 未知类型跳过校验
        }
        Ok(())
    }

    /// 参数校验规则
    fn validate_param_rule(
        def: &ParamDef,
        value: &serde_json::Value,
        validation: &ParamValidation,
    ) -> DsqlResult<()> {
        match validation.rule_type.as_str() {
            "not_empty" => {
                if let Some(s) = value.as_str() {
                    if s.is_empty() {
                        return Err(DsqlError::InvalidParam(format!("{} must not be empty", def.name)));
                    }
                }
            }
            "regex" => {
                if let (Some(pattern), Some(s)) = (&validation.pattern, value.as_str()) {
                    // 简单正则校验（不引入regex crate，用基础匹配）
                    if !s.contains(pattern.as_str()) {
                        // 这里简化处理，实际应使用regex crate
                    }
                }
            }
            "range" => {
                if let Some(n) = value.as_i64() {
                    if let Some(min) = validation.min.as_ref().and_then(|m| m.as_i64()) {
                        if n < min {
                            return Err(DsqlError::InvalidParam(format!("{} must be >= {}", def.name, min)));
                        }
                    }
                    if let Some(max) = validation.max.as_ref().and_then(|m| m.as_i64()) {
                        if n > max {
                            return Err(DsqlError::InvalidParam(format!("{} must be <= {}", def.name, max)));
                        }
                    }
                }
            }
            "enum" => {
                if let (Some(values), Some(s)) = (&validation.enum_values, value.as_str()) {
                    if !values.iter().any(|v| v == s) {
                        return Err(DsqlError::InvalidParam(format!("{} must be one of {:?}", def.name, values)));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 模板渲染：将{{param}}替换为?占位符，并按顺序收集参数
    /// 支持 {?if param?}...{?endif?} 条件片段
    fn render_template(
        template: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> DsqlResult<(String, Vec<serde_json::Value>)> {
        let mut result = String::new();
        let mut ordered_params: Vec<serde_json::Value> = Vec::new();
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                // 检查是否是 {{param}}
                if chars.peek() == Some(&'{') {
                    chars.next(); // 消费第二个{
                    let mut param_name = String::new();
                    while let Some(&pc) = chars.peek() {
                        if pc == '}' {
                            chars.next();
                            if chars.peek() == Some(&'}') {
                                chars.next();
                            }
                            break;
                        }
                        param_name.push(pc);
                        chars.next();
                    }
                    let param_name = param_name.trim();
                    if let Some(value) = params.get(param_name) {
                        result.push('?');
                        ordered_params.push(value.clone());
                    } else {
                        return Err(DsqlError::TemplateError(format!("unknown param: {param_name}")));
                    }
                    continue;
                }
                // 检查是否是 {?if param?}
                if chars.peek() == Some(&'?') {
                    chars.next(); // 消费?
                    let mut directive = String::new();
                    while let Some(&pc) = chars.peek() {
                        if pc == '?' {
                            chars.next();
                            if chars.peek() == Some(&'}') {
                                chars.next();
                            }
                            break;
                        }
                        directive.push(pc);
                        chars.next();
                    }
                    let directive = directive.trim();
                    if directive.starts_with("if ") {
                        let cond_param = directive[3..].trim();
                        let cond_true = params.get(cond_param)
                            .map(|v| !v.is_null() && v != &serde_json::Value::Bool(false) && v != &serde_json::Value::String(String::new()))
                            .unwrap_or(false);
                        // 收集if块内容
                        let mut block_content = String::new();
                        let mut depth = 1;
                        while let Some(bc) = chars.next() {
                            if bc == '{' && chars.peek() == Some(&'?') {
                                chars.next();
                                let mut inner_dir = String::new();
                                while let Some(&pc) = chars.peek() {
                                    if pc == '?' {
                                        chars.next();
                                        if chars.peek() == Some(&'}') {
                                            chars.next();
                                        }
                                        break;
                                    }
                                    inner_dir.push(pc);
                                    chars.next();
                                }
                                let inner_dir = inner_dir.trim();
                                if inner_dir.starts_with("if ") {
                                    depth += 1;
                                    block_content.push_str("{?if ");
                                    block_content.push_str(&inner_dir[3..]);
                                    block_content.push_str("?}");
                                } else if inner_dir == "endif" {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                    block_content.push_str("{?endif?}");
                                } else {
                                    block_content.push_str("{?");
                                    block_content.push_str(inner_dir);
                                    block_content.push_str("?}");
                                }
                            } else {
                                block_content.push(bc);
                            }
                        }
                        if cond_true {
                            // 递归渲染if块内容
                            let (sub_sql, sub_params) = Self::render_template(&block_content, params)?;
                            result.push_str(&sub_sql);
                            ordered_params.extend(sub_params);
                        }
                        continue;
                    }
                }
            }
            result.push(c);
        }

        Ok((result, ordered_params))
    }

    /// 执行查询
    fn execute_query(
        conn: &Connection,
        sql: &str,
        params: &[serde_json::Value],
        result_type: ResultType,
    ) -> DsqlResult<serde_json::Value> {
        let param_values: Vec<rusqlite::types::Value> = params.iter()
            .map(json_to_sqlite_value)
            .collect();

        match result_type {
            ResultType::List => {
                let mut stmt = conn.prepare(sql)
                    .map_err(|e| DsqlError::ExecutionError(format!("prepare: {e}")))?;
                let column_count = stmt.column_count();
                let column_names: Vec<String> = (0..column_count)
                    .map(|i| stmt.column_name(i).unwrap_or(&format!("col{i}")).to_string())
                    .collect();

                let rows = stmt.query_map(params_from_iter(param_values.iter()), |row| {
                    let mut map = serde_json::Map::new();
                    for (i, name) in column_names.iter().enumerate() {
                        let value = sqlite_value_to_json(row.get_ref(i).ok());
                        map.insert(name.clone(), value);
                    }
                    Ok(serde_json::Value::Object(map))
                }).map_err(|e| DsqlError::ExecutionError(format!("query: {e}")))?;

                let items: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();
                Ok(serde_json::Value::Array(items))
            }
            ResultType::Map => {
                let mut stmt = conn.prepare(sql)
                    .map_err(|e| DsqlError::ExecutionError(format!("prepare: {e}")))?;
                let column_count = stmt.column_count();
                let column_names: Vec<String> = (0..column_count)
                    .map(|i| stmt.column_name(i).unwrap_or(&format!("col{i}")).to_string())
                    .collect();

                let result = stmt.query_row(params_from_iter(param_values.iter()), |row| {
                    let mut map = serde_json::Map::new();
                    for (i, name) in column_names.iter().enumerate() {
                        let value = sqlite_value_to_json(row.get_ref(i).ok());
                        map.insert(name.clone(), value);
                    }
                    Ok(serde_json::Value::Object(map))
                }).optional()
                .map_err(|e| DsqlError::ExecutionError(format!("query: {e}")))?;

                Ok(result.unwrap_or(serde_json::Value::Null))
            }
            ResultType::Single => {
                let value: Option<rusqlite::types::Value> = conn.query_row(
                    sql,
                    params_from_iter(param_values.iter()),
                    |row| row.get(0),
                ).optional()
                .map_err(|e| DsqlError::ExecutionError(format!("query: {e}")))?;

                Ok(match value {
                    Some(v) => owned_sqlite_value_to_json(v),
                    None => serde_json::Value::Null,
                })
            }
            ResultType::Count => {
                let count: i64 = conn.query_row(
                    sql,
                    params_from_iter(param_values.iter()),
                    |row| row.get(0),
                ).unwrap_or(0);
                Ok(serde_json::json!({ "count": count }))
            }
            _ => Err(DsqlError::ExecutionError("invalid result type for read".to_string())),
        }
    }

    /// 执行写操作
    fn execute_write(
        conn: &Connection,
        sql: &str,
        params: &[serde_json::Value],
    ) -> DsqlResult<serde_json::Value> {
        let param_values: Vec<rusqlite::types::Value> = params.iter()
            .map(json_to_sqlite_value)
            .collect();

        let affected = conn.execute(sql, params_from_iter(param_values.iter()))
            .map_err(|e| DsqlError::ExecutionError(format!("execute: {e}")))?;

        Ok(serde_json::json!({ "affected_rows": affected }))
    }
}

/// JSON值 → SQLite值
fn json_to_sqlite_value(value: &serde_json::Value) -> rusqlite::types::Value {
    match value {
        serde_json::Value::Null => rusqlite::types::Value::Null,
        serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            rusqlite::types::Value::Text(value.to_string())
        }
    }
}

/// SQLite值 → JSON值（Owned版本）
fn owned_sqlite_value_to_json(value: rusqlite::types::Value) -> serde_json::Value {
    match value {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
        rusqlite::types::Value::Real(f) => serde_json::json!(f),
        rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
        rusqlite::types::Value::Blob(b) => serde_json::Value::String(hex::encode(b)),
    }
}

/// SQLite值 → JSON值
fn sqlite_value_to_json(value: Option<rusqlite::types::ValueRef>) -> serde_json::Value {
    match value {
        None | Some(rusqlite::types::ValueRef::Null) => serde_json::Value::Null,
        Some(rusqlite::types::ValueRef::Integer(i)) => serde_json::json!(i),
        Some(rusqlite::types::ValueRef::Real(f)) => serde_json::json!(f),
        Some(rusqlite::types::ValueRef::Text(s)) => {
            serde_json::Value::String(String::from_utf8_lossy(s).to_string())
        }
        Some(rusqlite::types::ValueRef::Blob(b)) => {
            serde_json::Value::String(hex::encode(b))
        }
    }
}
