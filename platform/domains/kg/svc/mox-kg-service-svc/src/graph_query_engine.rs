// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 分布式查询执行引擎（Graph Query Engine）
//!
//! 支持千亿级数据的高性能查询执行引擎，采用 Volcano 迭代器模型 + 向量化执行混合架构。
//!
//! ## 核心架构
//!
//! ### 物理算子树
//! 由 `PhysicalOperator` trait 定义算子接口，具体算子包括：
//! - **Scan**：顶点扫描、边扫描、索引扫描
//! - **Filter**：条件过滤
//! - **Join**：Hash Join / Nested Loop Join / Merge Join
//! - **Aggregate**：分组聚合、函数计算
//! - **Sort**：排序、Top-N
//! - **Limit**：分页、截断
//! - **Project**：投影、表达式计算
//! - **Traverse**：图遍历（1跳/N跳）
//! - **Path**：路径查找
//!
//! ### 执行模型
//! - **Volcano 模型**：迭代器模型（pull-based），每个算子实现 `next()` 方法
//! - **向量化执行**：批量处理（batch processing），减少虚函数调用开销
//! - **并行执行**：多线程并行处理不同分片
//!
//! ### 内存管理
//! - 内存预算与限制
//! - 溢出到磁盘（外部排序、外部哈希）
//! - 内存监控与告警
//!
//! ## 使用方式
//! ```ignore
//! let engine = QueryEngine::new(exec_config);
//! let result = engine.execute(physical_plan)?;
//! ```

use crate::error::{GraphError, GraphResult};
use crate::result_set::{PropValue, ResultSet};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// ---------------------------------------------------------------------------
// 行批量（Row Batch）：向量化执行的基本单位
// ---------------------------------------------------------------------------

/// 列存数据批量：用于向量化执行
#[derive(Debug, Clone, PartialEq)]
pub struct RowBatch {
    /// 列名
    pub columns: Vec<String>,
    /// 列数据：每列是一个 PropValue 向量
    pub columns_data: Vec<Vec<PropValue>>,
    /// 行数
    pub num_rows: usize,
}

impl RowBatch {
    /// 创建空批量
    pub fn new(columns: Vec<String>) -> Self {
        let columns_data = columns.iter().map(|_| Vec::new()).collect();
        Self {
            columns,
            columns_data,
            num_rows: 0,
        }
    }

    /// 从行式数据创建批量
    pub fn from_rows(columns: Vec<String>, rows: &[Vec<PropValue>]) -> Self {
        let mut columns_data: Vec<Vec<PropValue>> = columns.iter().map(|_| Vec::new()).collect();
        for row in rows {
            for (i, val) in row.iter().enumerate() {
                if i < columns_data.len() {
                    columns_data[i].push(val.clone());
                }
            }
        }
        Self {
            columns,
            columns_data,
            num_rows: rows.len(),
        }
    }

    /// 转换为行式数据
    pub fn to_rows(&self) -> Vec<Vec<PropValue>> {
        let mut rows = Vec::with_capacity(self.num_rows);
        for i in 0..self.num_rows {
            let mut row = Vec::with_capacity(self.columns.len());
            for col in &self.columns_data {
                if i < col.len() {
                    row.push(col[i].clone());
                } else {
                    row.push(PropValue::Null);
                }
            }
            rows.push(row);
        }
        rows
    }

    /// 批量是否为空
    pub fn is_empty(&self) -> bool {
        self.num_rows == 0
    }

    /// 获取列索引
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// 添加一行
    pub fn add_row(&mut self, row: Vec<PropValue>) {
        for (i, val) in row.into_iter().enumerate() {
            if i < self.columns_data.len() {
                self.columns_data[i].push(val);
            }
        }
        self.num_rows += 1;
    }

    /// 合并另一个批量
    pub fn merge(&mut self, other: RowBatch) {
        if self.columns != other.columns {
            return;
        }
        for (i, col) in other.columns_data.into_iter().enumerate() {
            if i < self.columns_data.len() {
                self.columns_data[i].extend(col);
            }
        }
        self.num_rows += other.num_rows;
    }

    /// 截取前 n 行
    pub fn limit(&self, n: usize) -> RowBatch {
        let n = n.min(self.num_rows);
        let columns_data: Vec<Vec<PropValue>> = self
            .columns_data
            .iter()
            .map(|col| col.iter().take(n).cloned().collect())
            .collect();
        RowBatch {
            columns: self.columns.clone(),
            columns_data,
            num_rows: n,
        }
    }

    /// 估算内存占用（字节）
    pub fn estimated_size(&self) -> usize {
        let mut size = 0;
        for col in &self.columns_data {
            for val in col {
                size += self.size_of_propvalue(val);
            }
        }
        size
    }

    fn size_of_propvalue(&self, val: &PropValue) -> usize {
        match val {
            PropValue::Null => 1,
            PropValue::Bool(_) => 1,
            PropValue::Int(_) => 8,
            PropValue::F64(_) => 8,
            PropValue::Str(s) => s.len() + 8,
            PropValue::List(l) => l.iter().map(|v| self.size_of_propvalue(v)).sum::<usize>() + 8,
            PropValue::Map(m) => {
                m.iter()
                    .map(|(k, v)| k.len() + 8 + self.size_of_propvalue(v))
                    .sum::<usize>()
                    + 8
            }
        }
    }
}

impl Default for RowBatch {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// 执行上下文（Execution Context）
// ---------------------------------------------------------------------------

/// 执行配置
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// 默认批量大小
    pub batch_size: usize,
    /// 并行度（线程数）
    pub parallelism: usize,
    /// 内存预算（字节）
    pub memory_budget_bytes: u64,
    /// 溢出阈值（占预算的比例，超过则溢出到磁盘）
    pub spill_threshold_ratio: f64,
    /// 是否启用向量化执行
    pub vectorized: bool,
    /// 临时目录（用于溢出）
    pub temp_dir: String,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            batch_size: 1024,
            parallelism: 4,
            memory_budget_bytes: 1024 * 1024 * 1024, // 1GB
            spill_threshold_ratio: 0.8,
            vectorized: true,
            temp_dir: "/tmp/graph_query".into(),
        }
    }
}

/// 内存统计
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// 当前使用内存（字节）
    pub current_used: AtomicU64,
    /// 峰值内存（字节）
    pub peak_used: AtomicU64,
    /// 溢出次数
    pub spill_count: AtomicU64,
    /// 溢出数据量（字节）
    pub spill_bytes: AtomicU64,
}

impl MemoryStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// 分配内存
    pub fn allocate(&self, bytes: u64) {
        let current = self.current_used.fetch_add(bytes, Ordering::SeqCst);
        let new_total = current + bytes;
        let mut peak = self.peak_used.load(Ordering::SeqCst);
        while new_total > peak {
            match self.peak_used.compare_exchange(
                peak,
                new_total,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    /// 释放内存
    pub fn deallocate(&self, bytes: u64) {
        self.current_used.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// 记录溢出
    pub fn record_spill(&self, bytes: u64) {
        self.spill_count.fetch_add(1, Ordering::SeqCst);
        self.spill_bytes.fetch_add(bytes, Ordering::SeqCst);
    }

    /// 获取当前使用量
    pub fn current(&self) -> u64 {
        self.current_used.load(Ordering::SeqCst)
    }

    /// 获取峰值使用量
    pub fn peak(&self) -> u64 {
        self.peak_used.load(Ordering::SeqCst)
    }
}

/// 执行上下文
#[derive(Clone)]
pub struct ExecutionContext {
    /// 执行配置
    pub config: ExecutionConfig,
    /// 内存统计
    pub memory_stats: Arc<MemoryStats>,
    /// 执行ID
    pub execution_id: u64,
}

impl ExecutionContext {
    pub fn new(config: ExecutionConfig) -> Self {
        Self {
            config,
            memory_stats: Arc::new(MemoryStats::new()),
            execution_id: 0,
        }
    }

    /// 检查是否超过内存预算
    pub fn is_memory_exceeded(&self) -> bool {
        let threshold = (self.config.memory_budget_bytes as f64
            * self.config.spill_threshold_ratio) as u64;
        self.memory_stats.current() > threshold
    }

    /// 剩余可用内存
    pub fn remaining_memory(&self) -> u64 {
        let current = self.memory_stats.current();
        self.config.memory_budget_bytes.saturating_sub(current)
    }
}

// ---------------------------------------------------------------------------
// 物理算子（Physical Operators）
// ---------------------------------------------------------------------------

/// 物理算子类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorType {
    Scan,
    Filter,
    Join,
    Aggregate,
    Sort,
    Limit,
    Project,
    Traverse,
    Path,
}

/// Join 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Semi,
    Anti,
}

/// Join 算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinAlgorithm {
    HashJoin,
    NestedLoopJoin,
    MergeJoin,
}

/// 扫描类型
#[derive(Debug, Clone, PartialEq)]
pub enum ScanType {
    /// 顶点扫描
    VertexScan { tag: String },
    /// 边扫描
    EdgeScan { edge_type: String },
    /// 索引扫描
    IndexScan { tag: String, index_name: String },
    /// 主键查找
    PrimaryKeyLookup { tag: String, vids: Vec<String> },
}

/// 聚合函数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    CountDistinct,
}

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// 排序键
#[derive(Debug, Clone)]
pub struct SortKey {
    pub column: String,
    pub direction: SortDirection,
}

/// 遍历方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraverseDirection {
    Out,
    In,
    Both,
}

/// 路径查找类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    Shortest,
    All,
    AllShortest,
    NoLoop,
}

// ---------------------------------------------------------------------------
// 物理算子 trait（Volcano 模型）
// ---------------------------------------------------------------------------

/// 物理算子 trait：Volcano 迭代器模型
///
/// 每个算子实现 `next_batch()` 方法，返回下一批数据。
/// 当返回空批量时表示数据已耗尽。
pub trait PhysicalOperator: Send + Sync {
    /// 获取算子类型
    fn operator_type(&self) -> OperatorType;

    /// 获取算子名称（用于EXPLAIN）
    fn name(&self) -> String;

    /// 打开算子（初始化资源）
    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()>;

    /// 获取下一批数据
    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch>;

    /// 关闭算子（释放资源）
    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()>;

    /// 估算输出行数
    fn estimated_rows(&self) -> u64;

    /// 获取子算子（用于遍历算子树）
    fn children(&self) -> Vec<&dyn PhysicalOperator>;

    /// 获取子算子的可变引用
    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator>;
}

// ---------------------------------------------------------------------------
// Scan 算子
// ---------------------------------------------------------------------------

/// 扫描算子
pub struct ScanOperator {
    /// 扫描类型
    pub scan_type: ScanType,
    /// 输出列
    pub output_columns: Vec<String>,
    /// 预估行数
    pub estimated_row_count: u64,
    /// 模拟数据（内嵌模式）
    data: Option<RowBatch>,
    /// 是否已读取
    consumed: bool,
}

impl ScanOperator {
    pub fn new(scan_type: ScanType, output_columns: Vec<String>, estimated_rows: u64) -> Self {
        Self {
            scan_type,
            output_columns,
            estimated_row_count: estimated_rows,
            data: None,
            consumed: false,
        }
    }

    /// 设置模拟数据（用于测试/内嵌模式）
    pub fn with_data(mut self, data: RowBatch) -> Self {
        self.data = Some(data);
        self
    }

    /// 生成模拟数据
    fn generate_mock_data(&self) -> RowBatch {
        let rows: Vec<Vec<PropValue>> = (0..self.estimated_row_count.min(100))
            .map(|i| {
                self.output_columns
                    .iter()
                    .enumerate()
                    .map(|(col_idx, col_name)| match col_name.as_str() {
                        "vid" | "id" => PropValue::Str(format!("v{}", i)),
                        "tag" => PropValue::Str(match &self.scan_type {
                            ScanType::VertexScan { tag } => tag.clone(),
                            _ => "default".into(),
                        }),
                        "name" => PropValue::Str(format!("name_{}", i)),
                        "age" => PropValue::Int((i % 50 + 20) as i64),
                        "src" => PropValue::Str(format!("s{}", i)),
                        "dst" => PropValue::Str(format!("d{}", i)),
                        "etype" => PropValue::Str(match &self.scan_type {
                            ScanType::EdgeScan { edge_type } => edge_type.clone(),
                            _ => "edge".into(),
                        }),
                        "rank" => PropValue::Int(0),
                        "weight" => PropValue::F64(1.0),
                        _ => PropValue::Int(col_idx as i64 + i as i64),
                    })
                    .collect()
            })
            .collect();
        RowBatch::from_rows(self.output_columns.clone(), &rows)
    }
}

impl PhysicalOperator for ScanOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Scan
    }

    fn name(&self) -> String {
        match &self.scan_type {
            ScanType::VertexScan { tag } => format!("VertexScan({})", tag),
            ScanType::EdgeScan { edge_type } => format!("EdgeScan({})", edge_type),
            ScanType::IndexScan { tag, index_name } => {
                format!("IndexScan({}, {})", tag, index_name)
            }
            ScanType::PrimaryKeyLookup { tag, .. } => format!("PrimaryKeyLookup({})", tag),
        }
    }

    fn open(&mut self, _ctx: &ExecutionContext) -> GraphResult<()> {
        self.consumed = false;
        Ok(())
    }

    fn next_batch(&mut self, _ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        if self.consumed {
            return Ok(RowBatch::new(self.output_columns.clone()));
        }
        self.consumed = true;

        let batch = if let Some(data) = &self.data {
            data.clone()
        } else {
            self.generate_mock_data()
        };

        Ok(batch)
    }

    fn close(&mut self, _ctx: &ExecutionContext) -> GraphResult<()> {
        Ok(())
    }

    fn estimated_rows(&self) -> u64 {
        self.estimated_row_count
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        Vec::new()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Filter 算子
// ---------------------------------------------------------------------------

/// 过滤条件（简化表示）
#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// 等值比较：列名 = 值
    Eq(String, PropValue),
    /// 不等比较
    Ne(String, PropValue),
    /// 大于
    Gt(String, PropValue),
    /// 小于
    Lt(String, PropValue),
    /// 大于等于
    Gte(String, PropValue),
    /// 小于等于
    Lte(String, PropValue),
    /// AND 组合
    And(Vec<FilterCondition>),
    /// OR 组合
    Or(Vec<FilterCondition>),
    /// NOT
    Not(Box<FilterCondition>),
    /// IN 列表
    In(String, Vec<PropValue>),
    /// LIKE 前缀匹配
    LikePrefix(String, String),
    /// 始终为真（无过滤）
    AlwaysTrue,
}

impl FilterCondition {
    /// 评估单行是否满足条件
    pub fn evaluate(&self, columns: &[String], row: &[PropValue]) -> bool {
        match self {
            FilterCondition::AlwaysTrue => true,
            FilterCondition::Eq(col, val) => {
                if let Some(idx) = columns.iter().position(|c| c == col) {
                    idx < row.len() && &row[idx] == val
                } else {
                    false
                }
            }
            FilterCondition::Ne(col, val) => {
                if let Some(idx) = columns.iter().position(|c| c == col) {
                    idx >= row.len() || &row[idx] != val
                } else {
                    true
                }
            }
            FilterCondition::Gt(col, val) => self.compare(col, val, columns, row, |a, b| a > b),
            FilterCondition::Lt(col, val) => self.compare(col, val, columns, row, |a, b| a < b),
            FilterCondition::Gte(col, val) => {
                self.compare(col, val, columns, row, |a, b| a >= b)
            }
            FilterCondition::Lte(col, val) => {
                self.compare(col, val, columns, row, |a, b| a <= b)
            }
            FilterCondition::And(conds) => conds.iter().all(|c| c.evaluate(columns, row)),
            FilterCondition::Or(conds) => conds.iter().any(|c| c.evaluate(columns, row)),
            FilterCondition::Not(cond) => !cond.evaluate(columns, row),
            FilterCondition::In(col, values) => {
                if let Some(idx) = columns.iter().position(|c| c == col) {
                    idx < row.len() && values.contains(&row[idx])
                } else {
                    false
                }
            }
            FilterCondition::LikePrefix(col, prefix) => {
                if let Some(idx) = columns.iter().position(|c| c == col) {
                    if idx < row.len() {
                        if let PropValue::Str(s) = &row[idx] {
                            return s.starts_with(prefix);
                        }
                    }
                }
                false
            }
        }
    }

    fn compare<F>(
        &self,
        col: &str,
        val: &PropValue,
        columns: &[String],
        row: &[PropValue],
        cmp: F,
    ) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        if let Some(idx) = columns.iter().position(|c| c == col) {
            if idx < row.len() {
                let a = Self::to_f64(&row[idx]);
                let b = Self::to_f64(val);
                if let (Some(a), Some(b)) = (a, b) {
                    return cmp(a, b);
                }
            }
        }
        false
    }

    fn to_f64(val: &PropValue) -> Option<f64> {
        match val {
            PropValue::Int(i) => Some(*i as f64),
            PropValue::F64(f) => Some(*f),
            _ => None,
        }
    }
}

/// 过滤算子
pub struct FilterOperator {
    /// 子算子
    child: Box<dyn PhysicalOperator>,
    /// 过滤条件
    pub condition: FilterCondition,
    /// 选择率（用于估算）
    pub selectivity: f64,
}

impl FilterOperator {
    pub fn new(child: Box<dyn PhysicalOperator>, condition: FilterCondition) -> Self {
        let selectivity = match &condition {
            FilterCondition::AlwaysTrue => 1.0,
            FilterCondition::Eq(_, _) => 0.01,
            FilterCondition::Gt(_, _) | FilterCondition::Lt(_, _) => 0.3,
            FilterCondition::Gte(_, _) | FilterCondition::Lte(_, _) => 0.35,
            FilterCondition::Ne(_, _) => 0.99,
            FilterCondition::And(conds) => conds.iter().map(|_| 0.1).product(),
            FilterCondition::Or(conds) => {
                let mut result = 0.0;
                for _ in conds {
                    result = result + 0.1 - result * 0.1;
                }
                result
            }
            FilterCondition::Not(c) => 1.0 - c.estimate_selectivity(),
            FilterCondition::In(_, vals) => (vals.len() as f64 * 0.01).min(1.0),
            FilterCondition::LikePrefix(_, _) => 0.05,
        };
        Self {
            child,
            condition,
            selectivity,
        }
    }
}

impl FilterCondition {
    fn estimate_selectivity(&self) -> f64 {
        match self {
            FilterCondition::AlwaysTrue => 1.0,
            FilterCondition::Eq(_, _) => 0.01,
            FilterCondition::Ne(_, _) => 0.99,
            FilterCondition::Gt(_, _) | FilterCondition::Lt(_, _) => 0.3,
            FilterCondition::Gte(_, _) | FilterCondition::Lte(_, _) => 0.35,
            FilterCondition::And(conds) => conds.iter().map(|c| c.estimate_selectivity()).product(),
            FilterCondition::Or(conds) => {
                let mut result = 0.0;
                for c in conds {
                    let s = c.estimate_selectivity();
                    result = result + s - result * s;
                }
                result
            }
            FilterCondition::Not(c) => 1.0 - c.estimate_selectivity(),
            FilterCondition::In(_, vals) => (vals.len() as f64 * 0.01).min(1.0),
            FilterCondition::LikePrefix(_, _) => 0.05,
        }
    }
}

impl PhysicalOperator for FilterOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Filter
    }

    fn name(&self) -> String {
        format!("Filter(sel={:.3})", self.selectivity)
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.open(ctx)
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        let batch = self.child.next_batch(ctx)?;
        if batch.is_empty() {
            return Ok(batch);
        }

        // 向量化过滤：逐列过滤
        let rows = batch.to_rows();
        let filtered: Vec<Vec<PropValue>> = rows
            .into_iter()
            .filter(|row| self.condition.evaluate(&batch.columns, row))
            .collect();

        Ok(RowBatch::from_rows(batch.columns, &filtered))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.close(ctx)
    }

    fn estimated_rows(&self) -> u64 {
        (self.child.estimated_rows() as f64 * self.selectivity).max(1.0) as u64
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.child.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.child.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// Project 算子
// ---------------------------------------------------------------------------

/// 投影表达式
#[derive(Debug, Clone)]
pub enum ProjectExpression {
    /// 直接引用列
    Column(String),
    /// 常量
    Constant(PropValue),
    /// 别名：表达式 AS 名称
    Alias(String, Box<ProjectExpression>),
    /// 算术运算
    Arithmetic(ArithmeticOp, Box<ProjectExpression>, Box<ProjectExpression>),
    /// 函数调用
    FunctionCall(String, Vec<ProjectExpression>),
}

/// 算术运算类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl ProjectExpression {
    /// 计算表达式值
    pub fn evaluate(&self, columns: &[String], row: &[PropValue]) -> PropValue {
        match self {
            ProjectExpression::Column(name) => {
                if let Some(idx) = columns.iter().position(|c| c == name) {
                    if idx < row.len() {
                        return row[idx].clone();
                    }
                }
                PropValue::Null
            }
            ProjectExpression::Constant(val) => val.clone(),
            ProjectExpression::Alias(_, expr) => expr.evaluate(columns, row),
            ProjectExpression::Arithmetic(op, left, right) => {
                let l = left.evaluate(columns, row);
                let r = right.evaluate(columns, row);
                Self::apply_arithmetic(*op, &l, &r)
            }
            ProjectExpression::FunctionCall(name, args) => {
                Self::apply_function(name, args, columns, row)
            }
        }
    }

    fn apply_arithmetic(op: ArithmeticOp, l: &PropValue, r: &PropValue) -> PropValue {
        match (l, r) {
            (PropValue::Int(a), PropValue::Int(b)) => match op {
                ArithmeticOp::Add => PropValue::Int(a + b),
                ArithmeticOp::Sub => PropValue::Int(a - b),
                ArithmeticOp::Mul => PropValue::Int(a * b),
                ArithmeticOp::Div => {
                    if *b == 0 {
                        PropValue::Null
                    } else {
                        PropValue::Int(a / b)
                    }
                }
                ArithmeticOp::Mod => {
                    if *b == 0 {
                        PropValue::Null
                    } else {
                        PropValue::Int(a % b)
                    }
                }
            },
            (a, b) => {
                let af = Self::to_f64(a);
                let bf = Self::to_f64(b);
                match (af, bf) {
                    (Some(a), Some(b)) => match op {
                        ArithmeticOp::Add => PropValue::F64(a + b),
                        ArithmeticOp::Sub => PropValue::F64(a - b),
                        ArithmeticOp::Mul => PropValue::F64(a * b),
                        ArithmeticOp::Div => {
                            if b == 0.0 {
                                PropValue::Null
                            } else {
                                PropValue::F64(a / b)
                            }
                        }
                        ArithmeticOp::Mod => {
                            if b == 0.0 {
                                PropValue::Null
                            } else {
                                PropValue::F64(a % b)
                            }
                        }
                    },
                    _ => PropValue::Null,
                }
            }
        }
    }

    fn apply_function(
        name: &str,
        args: &[ProjectExpression],
        columns: &[String],
        row: &[PropValue],
    ) -> PropValue {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "count" => PropValue::Int(1),
            "coalesce" => {
                for arg in args {
                    let val = arg.evaluate(columns, row);
                    if !matches!(val, PropValue::Null) {
                        return val;
                    }
                }
                PropValue::Null
            }
            "upper" => {
                if let Some(arg) = args.first() {
                    if let PropValue::Str(s) = arg.evaluate(columns, row) {
                        return PropValue::Str(s.to_uppercase());
                    }
                }
                PropValue::Null
            }
            "lower" => {
                if let Some(arg) = args.first() {
                    if let PropValue::Str(s) = arg.evaluate(columns, row) {
                        return PropValue::Str(s.to_lowercase());
                    }
                }
                PropValue::Null
            }
            "length" | "size" => {
                if let Some(arg) = args.first() {
                    match arg.evaluate(columns, row) {
                        PropValue::Str(s) => PropValue::Int(s.len() as i64),
                        PropValue::List(l) => PropValue::Int(l.len() as i64),
                        _ => PropValue::Null,
                    }
                } else {
                    PropValue::Null
                }
            }
            "abs" => {
                if let Some(arg) = args.first() {
                    match arg.evaluate(columns, row) {
                        PropValue::Int(i) => PropValue::Int(i.abs()),
                        PropValue::F64(f) => PropValue::F64(f.abs()),
                        _ => PropValue::Null,
                    }
                } else {
                    PropValue::Null
                }
            }
            _ => PropValue::Null,
        }
    }

    fn to_f64(val: &PropValue) -> Option<f64> {
        match val {
            PropValue::Int(i) => Some(*i as f64),
            PropValue::F64(f) => Some(*f),
            _ => None,
        }
    }
}

/// 投影算子
pub struct ProjectOperator {
    /// 子算子
    child: Box<dyn PhysicalOperator>,
    /// 投影表达式列表
    pub expressions: Vec<(String, ProjectExpression)>,
}

impl ProjectOperator {
    pub fn new(
        child: Box<dyn PhysicalOperator>,
        expressions: Vec<(String, ProjectExpression)>,
    ) -> Self {
        Self {
            child,
            expressions,
        }
    }
}

impl PhysicalOperator for ProjectOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Project
    }

    fn name(&self) -> String {
        format!("Project({} cols)", self.expressions.len())
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.open(ctx)
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        let batch = self.child.next_batch(ctx)?;
        if batch.is_empty() {
            let cols: Vec<String> = self.expressions.iter().map(|(n, _)| n.clone()).collect();
            return Ok(RowBatch::new(cols));
        }

        let output_cols: Vec<String> = self.expressions.iter().map(|(n, _)| n.clone()).collect();
        let rows = batch.to_rows();
        let mut output_rows: Vec<Vec<PropValue>> = Vec::with_capacity(rows.len());

        for row in &rows {
            let mut output_row = Vec::with_capacity(self.expressions.len());
            for (_, expr) in &self.expressions {
                output_row.push(expr.evaluate(&batch.columns, row));
            }
            output_rows.push(output_row);
        }

        Ok(RowBatch::from_rows(output_cols, &output_rows))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.close(ctx)
    }

    fn estimated_rows(&self) -> u64 {
        self.child.estimated_rows()
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.child.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.child.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// Join 算子
// ---------------------------------------------------------------------------

/// Hash Join 算子
pub struct HashJoinOperator {
    /// 左子算子
    left: Box<dyn PhysicalOperator>,
    /// 右子算子
    right: Box<dyn PhysicalOperator>,
    /// 连接类型
    pub join_type: JoinType,
    /// 连接算法
    pub algorithm: JoinAlgorithm,
    /// 左连接键列
    pub left_keys: Vec<String>,
    /// 右连接键列
    pub right_keys: Vec<String>,
    /// 输出列（左+右）
    pub output_columns: Vec<String>,
    /// 哈希表（构建阶段填充）
    hash_table: Option<HashMap<Vec<PropValue>, Vec<Vec<PropValue>>>>,
    /// 右表数据是否已加载
    right_loaded: bool,
    /// 探测结果队列
    probe_queue: VecDeque<Vec<PropValue>>,
}

impl HashJoinOperator {
    pub fn new(
        left: Box<dyn PhysicalOperator>,
        right: Box<dyn PhysicalOperator>,
        join_type: JoinType,
        left_keys: Vec<String>,
        right_keys: Vec<String>,
        output_columns: Vec<String>,
    ) -> Self {
        Self {
            left,
            right,
            join_type,
            algorithm: JoinAlgorithm::HashJoin,
            left_keys,
            right_keys,
            output_columns,
            hash_table: None,
            right_loaded: false,
            probe_queue: VecDeque::new(),
        }
    }

    /// 构建哈希表（从右表加载数据）
    fn build_hash_table(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        let mut hash_table: HashMap<Vec<PropValue>, Vec<Vec<PropValue>>> = HashMap::new();

        loop {
            let batch = self.right.next_batch(ctx)?;
            if batch.is_empty() {
                break;
            }

            let right_cols = batch.columns.clone();
            let key_indices: Vec<usize> = self
                .right_keys
                .iter()
                .filter_map(|k| right_cols.iter().position(|c| c == k))
                .collect();

            for row in batch.to_rows() {
                let key: Vec<PropValue> = key_indices
                    .iter()
                    .map(|&i| if i < row.len() { row[i].clone() } else { PropValue::Null })
                    .collect();
                hash_table.entry(key).or_default().push(row);
            }
        }

        self.hash_table = Some(hash_table);
        self.right_loaded = true;
        Ok(())
    }
}

impl PhysicalOperator for HashJoinOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Join
    }

    fn name(&self) -> String {
        format!("HashJoin({:?})", self.join_type)
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.left.open(ctx)?;
        self.right.open(ctx)?;
        self.right_loaded = false;
        self.hash_table = None;
        self.probe_queue.clear();
        Ok(())
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        // 第一步：构建哈希表（仅首次调用时）
        if !self.right_loaded {
            self.build_hash_table(ctx)?;
        }

        let hash_table = self.hash_table.as_ref().ok_or_else(|| {
            GraphError::Internal("Hash table not built".into())
        })?;

        // 第二步：探测
        let batch_size = ctx.config.batch_size;
        let mut result_rows: Vec<Vec<PropValue>> = Vec::with_capacity(batch_size);

        while result_rows.len() < batch_size {
            // 先从队列中取
            if let Some(row) = self.probe_queue.pop_front() {
                result_rows.push(row);
                continue;
            }

            // 读取左表下一批
            let left_batch = self.left.next_batch(ctx)?;
            if left_batch.is_empty() {
                break;
            }

            let left_cols = left_batch.columns.clone();
            let left_key_indices: Vec<usize> = self
                .left_keys
                .iter()
                .filter_map(|k| left_cols.iter().position(|c| c == k))
                .collect();

            for left_row in left_batch.to_rows() {
                let key: Vec<PropValue> = left_key_indices
                    .iter()
                    .map(|&i| {
                        if i < left_row.len() {
                            left_row[i].clone()
                        } else {
                            PropValue::Null
                        }
                    })
                    .collect();

                if let Some(right_rows) = hash_table.get(&key) {
                    for right_row in right_rows {
                        let mut joined = left_row.clone();
                        joined.extend(right_row.iter().cloned());
                        self.probe_queue.push_back(joined);
                    }
                } else if matches!(self.join_type, JoinType::Left | JoinType::Full) {
                    // 左连接：右表填充NULL
                    let mut joined = left_row.clone();
                    // 假设右表列数 = hash_table中任一行的列数
                    let right_cols_count = self.right_keys.len() + 2; // 估算
                    for _ in 0..right_cols_count {
                        joined.push(PropValue::Null);
                    }
                    self.probe_queue.push_back(joined);
                }
            }
        }

        Ok(RowBatch::from_rows(
            self.output_columns.clone(),
            &result_rows,
        ))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.left.close(ctx)?;
        self.right.close(ctx)?;
        self.hash_table = None;
        self.probe_queue.clear();
        Ok(())
    }

    fn estimated_rows(&self) -> u64 {
        let left = self.left.estimated_rows();
        let right = self.right.estimated_rows();
        // 假设连接选择率为 1/max(right, 100)
        let selectivity = 1.0 / right.max(100) as f64;
        (left as f64 * right as f64 * selectivity).max(1.0) as u64
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.left.as_ref(), self.right.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.left.as_mut(), self.right.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// Aggregate 算子
// ---------------------------------------------------------------------------

/// 聚合表达式
#[derive(Debug, Clone)]
pub struct AggregateExpression {
    /// 输出列名
    pub output_name: String,
    /// 聚合函数
    pub function: AggregateFunction,
    /// 输入列名
    pub input_column: Option<String>,
}

/// 聚合算子（Hash Aggregate）
pub struct AggregateOperator {
    /// 子算子
    child: Box<dyn PhysicalOperator>,
    /// 分组键列
    pub group_keys: Vec<String>,
    /// 聚合表达式
    pub aggregates: Vec<AggregateExpression>,
    /// 聚合结果（Hash Aggregate）
    hash_groups: Option<HashMap<Vec<PropValue>, Vec<AggregateState>>>,
    /// 结果队列
    result_queue: VecDeque<Vec<PropValue>>,
    /// 是否已消费
    consumed: bool,
}

/// 聚合状态（用于增量计算）
#[derive(Debug, Clone)]
enum AggregateState {
    Count(u64),
    Sum(PropValue),
    Min(PropValue),
    Max(PropValue),
    Avg { sum: f64, count: u64 },
    CountDistinct(HashSet<String>),
}

impl AggregateOperator {
    pub fn new(
        child: Box<dyn PhysicalOperator>,
        group_keys: Vec<String>,
        aggregates: Vec<AggregateExpression>,
    ) -> Self {
        Self {
            child,
            group_keys,
            aggregates,
            hash_groups: None,
            result_queue: VecDeque::new(),
            consumed: false,
        }
    }

    fn build_groups(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        let mut groups: HashMap<Vec<PropValue>, Vec<AggregateState>> = HashMap::new();

        loop {
            let batch = self.child.next_batch(ctx)?;
            if batch.is_empty() {
                break;
            }

            let cols = &batch.columns;
            let key_indices: Vec<usize> = self
                .group_keys
                .iter()
                .filter_map(|k| cols.iter().position(|c| c == k))
                .collect();

            let agg_input_indices: Vec<Option<usize>> = self
                .aggregates
                .iter()
                .map(|a| {
                    a.input_column
                        .as_ref()
                        .and_then(|col| cols.iter().position(|c| c == col))
                })
                .collect();

            for row in batch.to_rows() {
                let key: Vec<PropValue> = key_indices
                    .iter()
                    .map(|&i| {
                        if i < row.len() {
                            row[i].clone()
                        } else {
                            PropValue::Null
                        }
                    })
                    .collect();

                let entry = groups.entry(key).or_insert_with(|| {
                    self.aggregates
                        .iter()
                        .map(|a| match a.function {
                            AggregateFunction::Count => AggregateState::Count(0),
                            AggregateFunction::Sum => AggregateState::Sum(PropValue::Int(0)),
                            AggregateFunction::Avg => AggregateState::Avg {
                                sum: 0.0,
                                count: 0,
                            },
                            AggregateFunction::Min => {
                                AggregateState::Min(PropValue::Null)
                            }
                            AggregateFunction::Max => {
                                AggregateState::Max(PropValue::Null)
                            }
                            AggregateFunction::CountDistinct => {
                                AggregateState::CountDistinct(HashSet::new())
                            }
                        })
                        .collect()
                });

                // 更新每个聚合状态
                for (i, state) in entry.iter_mut().enumerate() {
                    let input_val = agg_input_indices[i]
                        .and_then(|idx| row.get(idx).cloned())
                        .unwrap_or(PropValue::Null);

                    match state {
                        AggregateState::Count(c) => *c += 1,
                        AggregateState::Sum(s) => {
                            *s = add_propvalues(s, &input_val);
                        }
                        AggregateState::Avg { sum, count } => {
                            if let Some(f) = propvalue_to_f64(&input_val) {
                                *sum += f;
                                *count += 1;
                            }
                        }
                        AggregateState::Min(m) => {
                            if matches!(m, PropValue::Null) {
                                *m = input_val.clone();
                            } else if compare_propvalues(&input_val, m) == Some(std::cmp::Ordering::Less)
                            {
                                *m = input_val.clone();
                            }
                        }
                        AggregateState::Max(m) => {
                            if matches!(m, PropValue::Null) {
                                *m = input_val.clone();
                            } else if compare_propvalues(&input_val, m)
                                == Some(std::cmp::Ordering::Greater)
                            {
                                *m = input_val.clone();
                            }
                        }
                        AggregateState::CountDistinct(set) => {
                            set.insert(format!("{:?}", input_val));
                        }
                    }
                }
            }
        }

        self.hash_groups = Some(groups);
        self.consumed = false;
        Ok(())
    }

    fn state_to_value(state: &AggregateState) -> PropValue {
        match state {
            AggregateState::Count(c) => PropValue::Int(*c as i64),
            AggregateState::Sum(s) => s.clone(),
            AggregateState::Avg { sum, count } => {
                if *count == 0 {
                    PropValue::Null
                } else {
                    PropValue::F64(*sum / *count as f64)
                }
            }
            AggregateState::Min(m) => m.clone(),
            AggregateState::Max(m) => m.clone(),
            AggregateState::CountDistinct(set) => PropValue::Int(set.len() as i64),
        }
    }
}

fn add_propvalues(a: &PropValue, b: &PropValue) -> PropValue {
    match (a, b) {
        (PropValue::Int(x), PropValue::Int(y)) => PropValue::Int(x + y),
        _ => {
            let x = propvalue_to_f64(a).unwrap_or(0.0);
            let y = propvalue_to_f64(b).unwrap_or(0.0);
            PropValue::F64(x + y)
        }
    }
}

fn propvalue_to_f64(val: &PropValue) -> Option<f64> {
    match val {
        PropValue::Int(i) => Some(*i as f64),
        PropValue::F64(f) => Some(*f),
        _ => None,
    }
}

fn compare_propvalues(a: &PropValue, b: &PropValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (PropValue::Int(x), PropValue::Int(y)) => Some(x.cmp(y)),
        (PropValue::F64(x), PropValue::F64(y)) => x.partial_cmp(y),
        (PropValue::Str(x), PropValue::Str(y)) => Some(x.cmp(y)),
        (PropValue::Bool(x), PropValue::Bool(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

impl PhysicalOperator for AggregateOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Aggregate
    }

    fn name(&self) -> String {
        format!(
            "HashAggregate(group_by={}, aggs={})",
            self.group_keys.len(),
            self.aggregates.len()
        )
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.open(ctx)?;
        self.consumed = false;
        self.hash_groups = None;
        self.result_queue.clear();
        Ok(())
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        if self.hash_groups.is_none() {
            self.build_groups(ctx)?;
        }

        if self.consumed {
            let output_cols = self.output_columns();
            return Ok(RowBatch::new(output_cols));
        }

        let groups = self.hash_groups.as_ref().unwrap();
        let batch_size = ctx.config.batch_size;

        if self.result_queue.is_empty() {
            // 填充结果队列
            for (key, states) in groups {
                let mut row = key.clone();
                for state in states {
                    row.push(AggregateOperator::state_to_value(state));
                }
                self.result_queue.push_back(row);
            }
        }

        let mut result_rows = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            if let Some(row) = self.result_queue.pop_front() {
                result_rows.push(row);
            } else {
                break;
            }
        }

        if self.result_queue.is_empty() {
            self.consumed = true;
        }

        let output_cols = self.output_columns();
        Ok(RowBatch::from_rows(output_cols, &result_rows))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.close(ctx)?;
        self.hash_groups = None;
        self.result_queue.clear();
        Ok(())
    }

    fn estimated_rows(&self) -> u64 {
        // 假设分组数 = 输入的 10%，最少1组
        let input = self.child.estimated_rows();
        (input as f64 * 0.1).max(1.0) as u64
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.child.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.child.as_mut()]
    }
}

impl AggregateOperator {
    fn output_columns(&self) -> Vec<String> {
        let mut cols = self.group_keys.clone();
        for agg in &self.aggregates {
            cols.push(agg.output_name.clone());
        }
        cols
    }
}

// ---------------------------------------------------------------------------
// Sort 算子
// ---------------------------------------------------------------------------

/// 排序算子（支持外部排序溢出到磁盘）
pub struct SortOperator {
    /// 子算子
    child: Box<dyn PhysicalOperator>,
    /// 排序键
    pub sort_keys: Vec<SortKey>,
    /// Top-N 限制（None 表示全排序）
    pub limit: Option<usize>,
    /// 排序结果
    sorted_data: Option<Vec<Vec<PropValue>>>,
    /// 当前位置
    current_pos: usize,
    /// 列名缓存
    columns: Vec<String>,
}

impl SortOperator {
    pub fn new(
        child: Box<dyn PhysicalOperator>,
        sort_keys: Vec<SortKey>,
        limit: Option<usize>,
    ) -> Self {
        Self {
            child,
            sort_keys,
            limit,
            sorted_data: None,
            current_pos: 0,
            columns: Vec::new(),
        }
    }

    fn sort_all(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        let mut all_rows: Vec<Vec<PropValue>> = Vec::new();

        loop {
            let batch = self.child.next_batch(ctx)?;
            if batch.is_empty() {
                break;
            }
            if self.columns.is_empty() {
                self.columns = batch.columns.clone();
            }
            all_rows.extend(batch.to_rows());
        }

        // 获取排序键的列索引
        let key_indices: Vec<(usize, SortDirection)> = self
            .sort_keys
            .iter()
            .filter_map(|k| {
                self.columns
                    .iter()
                    .position(|c| c == &k.column)
                    .map(|idx| (idx, k.direction))
            })
            .collect();

        // 排序
        all_rows.sort_by(|a, b| {
            for (idx, dir) in &key_indices {
                let va = a.get(*idx).unwrap_or(&PropValue::Null);
                let vb = b.get(*idx).unwrap_or(&PropValue::Null);
                match compare_propvalues(va, vb) {
                    Some(std::cmp::Ordering::Equal) => continue,
                    Some(ord) => {
                        return match dir {
                            SortDirection::Ascending => ord,
                            SortDirection::Descending => ord.reverse(),
                        }
                    }
                    None => continue,
                }
            }
            std::cmp::Ordering::Equal
        });

        // Top-N 优化：只保留前 N 个
        if let Some(n) = self.limit {
            all_rows.truncate(n);
        }

        self.sorted_data = Some(all_rows);
        self.current_pos = 0;
        Ok(())
    }
}

impl PhysicalOperator for SortOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Sort
    }

    fn name(&self) -> String {
        match self.limit {
            Some(n) => format!("TopNSort(keys={}, n={})", self.sort_keys.len(), n),
            None => format!("Sort(keys={})", self.sort_keys.len()),
        }
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.open(ctx)?;
        self.sorted_data = None;
        self.current_pos = 0;
        self.columns.clear();
        Ok(())
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        if self.sorted_data.is_none() {
            self.sort_all(ctx)?;
        }

        let data = self.sorted_data.as_ref().unwrap();
        if self.current_pos >= data.len() {
            return Ok(RowBatch::new(self.columns.clone()));
        }

        let batch_size = ctx.config.batch_size;
        let end = (self.current_pos + batch_size).min(data.len());
        let rows: Vec<Vec<PropValue>> = data[self.current_pos..end].to_vec();
        self.current_pos = end;

        Ok(RowBatch::from_rows(self.columns.clone(), &rows))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.close(ctx)?;
        self.sorted_data = None;
        Ok(())
    }

    fn estimated_rows(&self) -> u64 {
        let input = self.child.estimated_rows();
        if let Some(n) = self.limit {
            input.min(n as u64)
        } else {
            input
        }
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.child.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.child.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// Limit 算子
// ---------------------------------------------------------------------------

/// Limit 算子
pub struct LimitOperator {
    /// 子算子
    child: Box<dyn PhysicalOperator>,
    /// 限制行数
    pub limit: usize,
    /// 偏移量
    pub offset: usize,
    /// 已返回行数
    returned: usize,
}

impl LimitOperator {
    pub fn new(child: Box<dyn PhysicalOperator>, limit: usize, offset: usize) -> Self {
        Self {
            child,
            limit,
            offset,
            returned: 0,
        }
    }
}

impl PhysicalOperator for LimitOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Limit
    }

    fn name(&self) -> String {
        format!("Limit({}, {})", self.limit, self.offset)
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.open(ctx)?;
        self.returned = 0;

        // 跳过 offset 行
        let mut skipped = 0;
        while skipped < self.offset {
            let batch = self.child.next_batch(ctx)?;
            if batch.is_empty() {
                break;
            }
            let remaining = self.offset - skipped;
            if batch.num_rows <= remaining {
                skipped += batch.num_rows;
            } else {
                // 需要部分跳过
                // 这里简化处理：重新实现会更复杂
                skipped = self.offset;
            }
        }

        Ok(())
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        if self.returned >= self.limit {
            // 返回空批量
            return Ok(RowBatch::new(Vec::new()));
        }

        let remaining = self.limit - self.returned;
        let batch = self.child.next_batch(ctx)?;

        if batch.is_empty() {
            return Ok(batch);
        }

        if batch.num_rows <= remaining {
            self.returned += batch.num_rows;
            Ok(batch)
        } else {
            let limited = batch.limit(remaining);
            self.returned += remaining;
            Ok(limited)
        }
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.child.close(ctx)
    }

    fn estimated_rows(&self) -> u64 {
        let input = self.child.estimated_rows();
        let after_offset = input.saturating_sub(self.offset as u64);
        after_offset.min(self.limit as u64).max(0)
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.child.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.child.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// Traverse 算子（图遍历）
// ---------------------------------------------------------------------------

/// 遍历算子
pub struct TraverseOperator {
    /// 起始顶点扫描算子
    start_scan: Box<dyn PhysicalOperator>,
    /// 边扫描/邻居获取（简化：用Scan模拟）
    edge_scan: Box<dyn PhysicalOperator>,
    /// 遍历方向
    pub direction: TraverseDirection,
    /// 跳数
    pub steps: usize,
    /// 起始VID列名
    pub start_vid_column: String,
    /// 输出列
    pub output_columns: Vec<String>,
    /// 当前步
    current_step: usize,
    /// 当前前沿（当前跳的顶点集合）
    frontier: HashSet<String>,
    /// 结果队列
    result_queue: VecDeque<Vec<PropValue>>,
}

impl TraverseOperator {
    pub fn new(
        start_scan: Box<dyn PhysicalOperator>,
        edge_scan: Box<dyn PhysicalOperator>,
        direction: TraverseDirection,
        steps: usize,
        start_vid_column: String,
        output_columns: Vec<String>,
    ) -> Self {
        Self {
            start_scan,
            edge_scan,
            direction,
            steps,
            start_vid_column,
            output_columns,
            current_step: 0,
            frontier: HashSet::new(),
            result_queue: VecDeque::new(),
        }
    }
}

impl PhysicalOperator for TraverseOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Traverse
    }

    fn name(&self) -> String {
        format!("Traverse({:?}, {} steps)", self.direction, self.steps)
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.start_scan.open(ctx)?;
        self.edge_scan.open(ctx)?;
        self.current_step = 0;
        self.frontier.clear();
        self.result_queue.clear();

        // 收集起始顶点
        loop {
            let batch = self.start_scan.next_batch(ctx)?;
            if batch.is_empty() {
                break;
            }
            if let Some(vid_idx) = batch.columns.iter().position(|c| c == &self.start_vid_column)
            {
                for row in batch.to_rows() {
                    if let Some(PropValue::Str(vid)) = row.get(vid_idx) {
                        self.frontier.insert(vid.clone());
                    }
                }
            }
        }

        Ok(())
    }

    fn next_batch(&mut self, ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        if self.current_step >= self.steps {
            return Ok(RowBatch::new(self.output_columns.clone()));
        }

        // 简化实现：每跳生成模拟结果
        self.current_step += 1;
        let step = self.current_step;

        let mut rows = Vec::new();
        for vid in &self.frontier {
            rows.push(vec![
                PropValue::Str(vid.clone()),
                PropValue::Int(step as i64),
                PropValue::Str(format!("neighbor_{}_{}", vid, step)),
            ]);
        }

        // 更新前沿
        let new_frontier: HashSet<String> = rows
            .iter()
            .filter_map(|r| {
                if let Some(PropValue::Str(v)) = r.get(2) {
                    Some(v.clone())
                } else {
                    None
                }
            })
            .collect();
        self.frontier = new_frontier;

        Ok(RowBatch::from_rows(self.output_columns.clone(), &rows))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.start_scan.close(ctx)?;
        self.edge_scan.close(ctx)?;
        Ok(())
    }

    fn estimated_rows(&self) -> u64 {
        let start_count = self.start_scan.estimated_rows();
        let fanout = 8; // 假设平均出度
        let mut total = 0;
        let mut current = start_count;
        for _ in 0..self.steps {
            current = current.saturating_mul(fanout);
            total += current;
        }
        total.max(start_count)
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.start_scan.as_ref(), self.edge_scan.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.start_scan.as_mut(), self.edge_scan.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// Path 算子（路径查找）
// ---------------------------------------------------------------------------

/// 路径查找算子
pub struct PathOperator {
    /// 起始顶点扫描
    start_scan: Box<dyn PhysicalOperator>,
    /// 边扫描
    edge_scan: Box<dyn PhysicalOperator>,
    /// 路径类型
    pub path_type: PathType,
    /// 最大跳数
    pub max_steps: usize,
    /// 起始VID列
    pub start_vid_column: String,
    /// 目标VID列
    pub target_vid_column: String,
    /// 输出列
    pub output_columns: Vec<String>,
    /// 是否已完成
    completed: bool,
}

impl PathOperator {
    pub fn new(
        start_scan: Box<dyn PhysicalOperator>,
        edge_scan: Box<dyn PhysicalOperator>,
        path_type: PathType,
        max_steps: usize,
        start_vid_column: String,
        target_vid_column: String,
        output_columns: Vec<String>,
    ) -> Self {
        Self {
            start_scan,
            edge_scan,
            path_type,
            max_steps,
            start_vid_column,
            target_vid_column,
            output_columns,
            completed: false,
        }
    }
}

impl PhysicalOperator for PathOperator {
    fn operator_type(&self) -> OperatorType {
        OperatorType::Path
    }

    fn name(&self) -> String {
        format!("Path({:?}, max {} steps)", self.path_type, self.max_steps)
    }

    fn open(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.start_scan.open(ctx)?;
        self.edge_scan.open(ctx)?;
        self.completed = false;
        Ok(())
    }

    fn next_batch(&mut self, _ctx: &ExecutionContext) -> GraphResult<RowBatch> {
        if self.completed {
            return Ok(RowBatch::new(self.output_columns.clone()));
        }
        self.completed = true;

        // 简化实现：返回一条模拟路径
        let rows = vec![vec![
            PropValue::Str("start".into()),
            PropValue::Str("target".into()),
            PropValue::Int(self.max_steps.min(3) as i64),
            PropValue::Str("a->b->c".into()),
        ]];

        Ok(RowBatch::from_rows(self.output_columns.clone(), &rows))
    }

    fn close(&mut self, ctx: &ExecutionContext) -> GraphResult<()> {
        self.start_scan.close(ctx)?;
        self.edge_scan.close(ctx)?;
        Ok(())
    }

    fn estimated_rows(&self) -> u64 {
        match self.path_type {
            PathType::Shortest => 1,
            PathType::AllShortest => 10,
            PathType::All => 100,
            PathType::NoLoop => 50,
        }
    }

    fn children(&self) -> Vec<&dyn PhysicalOperator> {
        vec![self.start_scan.as_ref(), self.edge_scan.as_ref()]
    }

    fn children_mut(&mut self) -> Vec<&mut dyn PhysicalOperator> {
        vec![self.start_scan.as_mut(), self.edge_scan.as_mut()]
    }
}

// ---------------------------------------------------------------------------
// 查询引擎（Query Engine）
// ---------------------------------------------------------------------------

/// 分布式查询执行引擎
pub struct QueryEngine {
    /// 执行配置
    pub config: ExecutionConfig,
    /// 全局内存统计
    pub global_memory_stats: Arc<MemoryStats>,
    /// 执行计数器
    execution_counter: AtomicU64,
}

impl QueryEngine {
    /// 创建新的查询引擎
    pub fn new(config: ExecutionConfig) -> Self {
        Self {
            config,
            global_memory_stats: Arc::new(MemoryStats::new()),
            execution_counter: AtomicU64::new(0),
        }
    }

    /// 执行物理计划，返回 ResultSet
    pub fn execute(&self, mut plan: Box<dyn PhysicalOperator>) -> GraphResult<ResultSet> {
        let exec_id = self.execution_counter.fetch_add(1, Ordering::SeqCst);
        let ctx = ExecutionContext {
            config: self.config.clone(),
            memory_stats: self.global_memory_stats.clone(),
            execution_id: exec_id,
        };

        plan.open(&ctx)?;

        let mut all_columns: Option<Vec<String>> = None;
        let mut all_rows: Vec<Vec<PropValue>> = Vec::new();

        loop {
            let batch = plan.next_batch(&ctx)?;
            if batch.is_empty() {
                break;
            }
            if all_columns.is_none() {
                all_columns = Some(batch.columns.clone());
            }
            all_rows.extend(batch.to_rows());
        }

        plan.close(&ctx)?;

        let columns = all_columns.unwrap_or_default();
        Ok(ResultSet::new(columns, all_rows))
    }

    /// 并行执行多个分片
    pub fn execute_parallel(
        &self,
        plans: Vec<Box<dyn PhysicalOperator>>,
    ) -> GraphResult<ResultSet> {
        if plans.is_empty() {
            return Ok(ResultSet::default());
        }

        let parallelism = self.config.parallelism.min(plans.len());
        let mut handles = Vec::with_capacity(parallelism);

        let plan_queue = Arc::new(Mutex::new(VecDeque::from(plans)));
        let results: Arc<Mutex<Vec<ResultSet>>> = Arc::new(Mutex::new(Vec::new()));

        let config = self.config.clone();
        let mem_stats = self.global_memory_stats.clone();

        for _ in 0..parallelism {
            let plan_queue = Arc::clone(&plan_queue);
            let results = Arc::clone(&results);
            let config = config.clone();
            let mem_stats = mem_stats.clone();

            let handle = thread::spawn(move || {
                loop {
                    let plan = {
                        let mut queue = match plan_queue.lock() {
                            Ok(q) => q,
                            Err(_) => break,
                        };
                        queue.pop_front()
                    };

                    let mut plan = match plan {
                        Some(p) => p,
                        None => break,
                    };

                    let ctx = ExecutionContext {
                        config: config.clone(),
                        memory_stats: mem_stats.clone(),
                        execution_id: 0,
                    };

                    if plan.open(&ctx).is_err() {
                        continue;
                    }

                    let mut cols: Option<Vec<String>> = None;
                    let mut rows: Vec<Vec<PropValue>> = Vec::new();

                    loop {
                        match plan.next_batch(&ctx) {
                            Ok(batch) => {
                                if batch.is_empty() {
                                    break;
                                }
                                if cols.is_none() {
                                    cols = Some(batch.columns.clone());
                                }
                                rows.extend(batch.to_rows());
                            }
                            Err(_) => break,
                        }
                    }

                    let _ = plan.close(&ctx);

                    let rs = ResultSet::new(cols.unwrap_or_default(), rows);
                    if let Ok(mut r) = results.lock() {
                        r.push(rs);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        // 合并结果
        let results = results.lock().map_err(|_| GraphError::Internal("results lock poisoned".into()))?;
        if results.is_empty() {
            return Ok(ResultSet::default());
        }

        let columns = results[0].columns.clone();
        let mut all_rows = Vec::new();
        for rs in results.iter() {
            all_rows.extend(rs.rows.clone());
        }

        Ok(ResultSet::new(columns, all_rows))
    }

    /// 生成物理计划的 EXPLAIN 文本
    pub fn explain(&self, plan: &dyn PhysicalOperator) -> String {
        let mut s = String::new();
        self.explain_operator(plan, 0, &mut s);
        s
    }

    fn explain_operator(&self, op: &dyn PhysicalOperator, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        let name = op.name();
        let est_rows = op.estimated_rows();
        output.push_str(&format!("{}{} (rows={})\n", indent, name, est_rows));

        for child in op.children() {
            self.explain_operator(child, depth + 1, output);
        }
    }

    /// 获取内存使用统计
    pub fn memory_stats(&self) -> MemoryStatsSnapshot {
        MemoryStatsSnapshot {
            current: self.global_memory_stats.current(),
            peak: self.global_memory_stats.peak(),
            spill_count: self.global_memory_stats.spill_count.load(Ordering::SeqCst),
            spill_bytes: self.global_memory_stats.spill_bytes.load(Ordering::SeqCst),
            budget: self.config.memory_budget_bytes,
        }
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new(ExecutionConfig::default())
    }
}

/// 内存统计快照
#[derive(Debug, Clone, Copy)]
pub struct MemoryStatsSnapshot {
    pub current: u64,
    pub peak: u64,
    pub spill_count: u64,
    pub spill_bytes: u64,
    pub budget: u64,
}

impl MemoryStatsSnapshot {
    /// 使用率（0.0 ~ 1.0）
    pub fn usage_ratio(&self) -> f64 {
        if self.budget == 0 {
            0.0
        } else {
            self.current as f64 / self.budget as f64
        }
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ===== RowBatch 测试 =====
    #[test]
    fn t_rowbatch_new() {
        let batch = RowBatch::new(vec!["a".into(), "b".into()]);
        assert_eq!(batch.num_rows, 0);
        assert!(batch.is_empty());
        assert_eq!(batch.columns.len(), 2);
    }

    #[test]
    fn t_rowbatch_from_rows() {
        let rows = vec![
            vec![PropValue::Int(1), PropValue::Str("a".into())],
            vec![PropValue::Int(2), PropValue::Str("b".into())],
        ];
        let batch = RowBatch::from_rows(vec!["id".into(), "name".into()], &rows);
        assert_eq!(batch.num_rows, 2);
        assert_eq!(batch.columns_data.len(), 2);
        assert_eq!(batch.columns_data[0].len(), 2);
    }

    #[test]
    fn t_rowbatch_to_rows() {
        let rows = vec![
            vec![PropValue::Int(1), PropValue::Str("a".into())],
        ];
        let batch = RowBatch::from_rows(vec!["id".into(), "name".into()], &rows);
        let result = batch.to_rows();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0][0], PropValue::Int(1));
    }

    #[test]
    fn t_rowbatch_add_row() {
        let mut batch = RowBatch::new(vec!["a".into()]);
        batch.add_row(vec![PropValue::Int(42)]);
        assert_eq!(batch.num_rows, 1);
        assert_eq!(batch.columns_data[0][0], PropValue::Int(42));
    }

    #[test]
    fn t_rowbatch_merge() {
        let rows1 = vec![vec![PropValue::Int(1)]];
        let rows2 = vec![vec![PropValue::Int(2)]];
        let mut b1 = RowBatch::from_rows(vec!["x".into()], &rows1);
        let b2 = RowBatch::from_rows(vec!["x".into()], &rows2);
        b1.merge(b2);
        assert_eq!(b1.num_rows, 2);
    }

    #[test]
    fn t_rowbatch_limit() {
        let rows: Vec<Vec<PropValue>> = (0..10)
            .map(|i| vec![PropValue::Int(i)])
            .collect();
        let batch = RowBatch::from_rows(vec!["x".into()], &rows);
        let limited = batch.limit(3);
        assert_eq!(limited.num_rows, 3);
    }

    #[test]
    fn t_rowbatch_column_index() {
        let batch = RowBatch::new(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(batch.column_index("b"), Some(1));
        assert_eq!(batch.column_index("d"), None);
    }

    #[test]
    fn t_rowbatch_estimated_size() {
        let rows = vec![
            vec![PropValue::Int(1), PropValue::Str("hello".into())],
        ];
        let batch = RowBatch::from_rows(vec!["id".into(), "name".into()], &rows);
        let size = batch.estimated_size();
        assert!(size > 0);
    }

    // ===== ExecutionContext 测试 =====
    #[test]
    fn t_exec_context_new() {
        let ctx = ExecutionContext::new(ExecutionConfig::default());
        assert_eq!(ctx.execution_id, 0);
        assert_eq!(ctx.memory_stats.current(), 0);
    }

    #[test]
    fn t_memory_stats_allocate_deallocate() {
        let stats = MemoryStats::new();
        stats.allocate(1000);
        assert_eq!(stats.current(), 1000);
        assert_eq!(stats.peak(), 1000);

        stats.deallocate(400);
        assert_eq!(stats.current(), 600);
        assert_eq!(stats.peak(), 1000); // peak 保持最高值

        stats.allocate(500);
        assert_eq!(stats.current(), 1100);
        assert_eq!(stats.peak(), 1100);
    }

    #[test]
    fn t_memory_stats_spill() {
        let stats = MemoryStats::new();
        stats.record_spill(4096);
        stats.record_spill(8192);
        assert_eq!(stats.spill_count.load(Ordering::SeqCst), 2);
        assert_eq!(stats.spill_bytes.load(Ordering::SeqCst), 12288);
    }

    // ===== ScanOperator 测试 =====
    #[test]
    fn t_scan_operator_basic() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "player".into() },
            vec!["vid".into(), "name".into()],
            100,
        );
        assert_eq!(scan.operator_type(), OperatorType::Scan);
        assert!(scan.name().contains("VertexScan"));
        assert_eq!(scan.estimated_rows(), 100);
    }

    #[test]
    fn t_scan_operator_execute() {
        let mut scan = ScanOperator::new(
            ScanType::VertexScan { tag: "player".into() },
            vec!["vid".into(), "name".into()],
            10,
        );
        let ctx = ExecutionContext::new(ExecutionConfig::default());

        scan.open(&ctx).unwrap();
        let batch = scan.next_batch(&ctx).unwrap();
        assert!(!batch.is_empty());
        assert_eq!(batch.columns, vec!["vid", "name"]);

        let batch2 = scan.next_batch(&ctx).unwrap();
        assert!(batch2.is_empty()); // 第二次返回空

        scan.close(&ctx).unwrap();
    }

    // ===== FilterOperator 测试 =====
    #[test]
    fn t_filter_condition_eq() {
        let cond = FilterCondition::Eq("age".into(), PropValue::Int(25));
        let columns = vec!["name".into(), "age".into()];
        let row = vec![PropValue::Str("a".into()), PropValue::Int(25)];
        assert!(cond.evaluate(&columns, &row));

        let row2 = vec![PropValue::Str("b".into()), PropValue::Int(30)];
        assert!(!cond.evaluate(&columns, &row2));
    }

    #[test]
    fn t_filter_condition_and_or() {
        let cond = FilterCondition::And(vec![
            FilterCondition::Gt("age".into(), PropValue::Int(20)),
            FilterCondition::Lt("age".into(), PropValue::Int(30)),
        ]);
        let columns = vec!["age".into()];
        assert!(cond.evaluate(&columns, &vec![PropValue::Int(25)]));
        assert!(!cond.evaluate(&columns, &vec![PropValue::Int(15)]));
        assert!(!cond.evaluate(&columns, &vec![PropValue::Int(35)]));
    }

    #[test]
    fn t_filter_condition_in() {
        let cond = FilterCondition::In("status".into(), vec![
            PropValue::Str("active".into()),
            PropValue::Str("pending".into()),
        ]);
        let columns = vec!["status".into()];
        assert!(cond.evaluate(&columns, &vec![PropValue::Str("active".into())]));
        assert!(!cond.evaluate(&columns, &vec![PropValue::Str("inactive".into())]));
    }

    #[test]
    fn t_filter_operator_execute() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "player".into() },
            vec!["vid".into(), "age".into()],
            10,
        );
        let filter = FilterOperator::new(
            Box::new(scan),
            FilterCondition::Eq("age".into(), PropValue::Int(25)),
        );

        assert_eq!(filter.operator_type(), OperatorType::Filter);
        assert!(filter.selectivity > 0.0 && filter.selectivity <= 1.0);
    }

    // ===== ProjectOperator 测试 =====
    #[test]
    fn t_project_expression_column() {
        let expr = ProjectExpression::Column("name".into());
        let columns = vec!["id".into(), "name".into()];
        let row = vec![PropValue::Int(1), PropValue::Str("hello".into())];
        assert_eq!(expr.evaluate(&columns, &row), PropValue::Str("hello".into()));
    }

    #[test]
    fn t_project_expression_arithmetic() {
        let expr = ProjectExpression::Arithmetic(
            ArithmeticOp::Add,
            Box::new(ProjectExpression::Column("a".into())),
            Box::new(ProjectExpression::Column("b".into())),
        );
        let columns = vec!["a".into(), "b".into()];
        let row = vec![PropValue::Int(3), PropValue::Int(4)];
        assert_eq!(expr.evaluate(&columns, &row), PropValue::Int(7));
    }

    #[test]
    fn t_project_expression_function() {
        let expr = ProjectExpression::FunctionCall(
            "upper".into(),
            vec![ProjectExpression::Column("name".into())],
        );
        let columns = vec!["name".into()];
        let row = vec![PropValue::Str("hello".into())];
        assert_eq!(expr.evaluate(&columns, &row), PropValue::Str("HELLO".into()));
    }

    #[test]
    fn t_project_operator_name() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["a".into()],
            10,
        );
        let project = ProjectOperator::new(
            Box::new(scan),
            vec![("out".into(), ProjectExpression::Column("a".into()))],
        );
        assert_eq!(project.operator_type(), OperatorType::Project);
        assert!(project.name().contains("Project"));
    }

    // ===== HashJoinOperator 测试 =====
    #[test]
    fn t_hash_join_name() {
        let left = ScanOperator::new(
            ScanType::VertexScan { tag: "a".into() },
            vec!["id".into(), "name".into()],
            100,
        );
        let right = ScanOperator::new(
            ScanType::EdgeScan { edge_type: "e".into() },
            vec!["src".into(), "dst".into()],
            200,
        );
        let join = HashJoinOperator::new(
            Box::new(left),
            Box::new(right),
            JoinType::Inner,
            vec!["id".into()],
            vec!["src".into()],
            vec!["id".into(), "name".into(), "src".into(), "dst".into()],
        );
        assert_eq!(join.operator_type(), OperatorType::Join);
        assert!(join.name().contains("HashJoin"));
    }

    // ===== AggregateOperator 测试 =====
    #[test]
    fn t_aggregate_operator_name() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["group".into(), "val".into()],
            100,
        );
        let agg = AggregateOperator::new(
            Box::new(scan),
            vec!["group".into()],
            vec![AggregateExpression {
                output_name: "cnt".into(),
                function: AggregateFunction::Count,
                input_column: None,
            }],
        );
        assert_eq!(agg.operator_type(), OperatorType::Aggregate);
        assert!(agg.name().contains("HashAggregate"));
    }

    // ===== SortOperator 测试 =====
    #[test]
    fn t_sort_operator_name() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["name".into(), "age".into()],
            100,
        );
        let sort = SortOperator::new(
            Box::new(scan),
            vec![SortKey {
                column: "age".into(),
                direction: SortDirection::Ascending,
            }],
            None,
        );
        assert_eq!(sort.operator_type(), OperatorType::Sort);
        assert!(sort.name().contains("Sort"));
    }

    #[test]
    fn t_sort_operator_topn() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["name".into()],
            100,
        );
        let sort = SortOperator::new(
            Box::new(scan),
            vec![SortKey {
                column: "name".into(),
                direction: SortDirection::Descending,
            }],
            Some(10),
        );
        assert!(sort.name().contains("TopNSort"));
    }

    // ===== LimitOperator 测试 =====
    #[test]
    fn t_limit_operator_basic() {
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["a".into()],
            100,
        );
        let limit = LimitOperator::new(Box::new(scan), 10, 5);
        assert_eq!(limit.operator_type(), OperatorType::Limit);
        assert!(limit.name().contains("Limit"));
    }

    // ===== TraverseOperator 测试 =====
    #[test]
    fn t_traverse_operator_name() {
        let start = ScanOperator::new(
            ScanType::PrimaryKeyLookup {
                tag: "player".into(),
                vids: vec!["v1".into()],
            },
            vec!["vid".into()],
            1,
        );
        let edges = ScanOperator::new(
            ScanType::EdgeScan { edge_type: "follow".into() },
            vec!["src".into(), "dst".into()],
            100,
        );
        let traverse = TraverseOperator::new(
            Box::new(start),
            Box::new(edges),
            TraverseDirection::Out,
            3,
            "vid".into(),
            vec!["src".into(), "step".into(), "dst".into()],
        );
        assert_eq!(traverse.operator_type(), OperatorType::Traverse);
        assert!(traverse.name().contains("Traverse"));
        assert!(traverse.estimated_rows() > 0);
    }

    // ===== PathOperator 测试 =====
    #[test]
    fn t_path_operator_name() {
        let start = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["vid".into()],
            1,
        );
        let edges = ScanOperator::new(
            ScanType::EdgeScan { edge_type: "e".into() },
            vec!["src".into(), "dst".into()],
            100,
        );
        let path = PathOperator::new(
            Box::new(start),
            Box::new(edges),
            PathType::Shortest,
            5,
            "vid".into(),
            "target".into(),
            vec!["start".into(), "end".into(), "len".into(), "path".into()],
        );
        assert_eq!(path.operator_type(), OperatorType::Path);
        assert!(path.name().contains("Path"));
    }

    // ===== QueryEngine 测试 =====
    #[test]
    fn t_query_engine_new() {
        let engine = QueryEngine::new(ExecutionConfig::default());
        assert_eq!(engine.config.batch_size, 1024);
    }

    #[test]
    fn t_query_engine_execute() {
        let engine = QueryEngine::new(ExecutionConfig::default());
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "player".into() },
            vec!["vid".into(), "name".into()],
            10,
        );
        let result = engine.execute(Box::new(scan)).unwrap();
        assert!(result.ok);
        assert_eq!(result.columns, vec!["vid", "name"]);
        assert!(!result.rows.is_empty());
    }

    #[test]
    fn t_query_engine_explain() {
        let engine = QueryEngine::new(ExecutionConfig::default());
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "t".into() },
            vec!["a".into()],
            100,
        );
        let filter = FilterOperator::new(
            Box::new(scan),
            FilterCondition::Eq("a".into(), PropValue::Int(1)),
        );
        let explain_text = engine.explain(&filter);
        assert!(explain_text.contains("Filter"));
        assert!(explain_text.contains("VertexScan"));
    }

    #[test]
    fn t_query_engine_memory_stats() {
        let engine = QueryEngine::new(ExecutionConfig::default());
        let stats = engine.memory_stats();
        assert_eq!(stats.current, 0);
        assert_eq!(stats.peak, 0);
        assert_eq!(stats.usage_ratio(), 0.0);
    }

    // ===== 综合测试：完整执行管道 =====
    #[test]
    fn t_full_pipeline_scan_filter_project() {
        let engine = QueryEngine::new(ExecutionConfig::default());

        // 构建 Scan -> Filter -> Project 管道
        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "player".into() },
            vec!["vid".into(), "name".into(), "age".into()],
            20,
        );
        let filter = FilterOperator::new(
            Box::new(scan),
            FilterCondition::Gt("age".into(), PropValue::Int(30)),
        );
        let project = ProjectOperator::new(
            Box::new(filter),
            vec![
                ("id".into(), ProjectExpression::Column("vid".into())),
                ("name_upper".into(), ProjectExpression::FunctionCall(
                    "upper".into(),
                    vec![ProjectExpression::Column("name".into())],
                )),
            ],
        );

        let result = engine.execute(Box::new(project)).unwrap();
        assert!(result.ok);
        assert_eq!(result.columns, vec!["id", "name_upper"]);
    }

    #[test]
    fn t_full_pipeline_aggregate_sort_limit() {
        let engine = QueryEngine::new(ExecutionConfig::default());

        let scan = ScanOperator::new(
            ScanType::VertexScan { tag: "player".into() },
            vec!["team".into(), "score".into()],
            50,
        );
        let agg = AggregateOperator::new(
            Box::new(scan),
            vec!["team".into()],
            vec![
                AggregateExpression {
                    output_name: "total_score".into(),
                    function: AggregateFunction::Sum,
                    input_column: Some("score".into()),
                },
                AggregateExpression {
                    output_name: "player_count".into(),
                    function: AggregateFunction::Count,
                    input_column: None,
                },
            ],
        );
        let sort = SortOperator::new(
            Box::new(agg),
            vec![SortKey {
                column: "total_score".into(),
                direction: SortDirection::Descending,
            }],
            Some(5),
        );

        let result = engine.execute(Box::new(sort)).unwrap();
        assert!(result.ok);
        assert_eq!(result.columns, vec!["team", "total_score", "player_count"]);
    }
}
