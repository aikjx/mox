// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # Mox R3 Graph Service
//!
//! Zero third-party成品 graph DB embedded; Parser 100% Mox self-dev.
//!
//! License: MIT OR Apache-2.0
//!
//! 包含：
//! - GraphServer：nGQL / openCypher 执行入口，使用内嵌 StorageEngine 访问存储
//! - NgqlParser：支持 100+ 条 nGQL 语句解析（含索引管理、EXPLAIN/PROFILE、异步任务、快照、均衡、配置等）
//! - CypherParser：支持 20 条 openCypher 语句解析
//! - Optimizer：规则 + 代价的查询优化器（CBO + RBO + 执行计划缓存）
//! - GraphQueryEngine：分布式查询执行引擎（Volcano 迭代器模型 + 向量化执行）
//! - GraphIndex：图索引管理（主键/类型/属性/全文/向量索引）
//! - AlgoBridge：Rust 7 算法纯内联实现，精度护栏与 T5 single-source 一致
//! - ResultSet：标准化行列结果集；PropValue 原子类型枚举；LPA 弃用 helper
//!
//! 本 crate 零引用第三方成品图数据库 crate。

pub mod ac15_faults;
pub mod algo_bridge;
pub mod community_cnm;
pub mod cypher_parser;
pub mod kg_graph;
pub mod error;
pub mod graph_index;
pub mod graph_query_engine;
pub mod graph_server;
pub mod ngql_parser;
pub mod optimizer;
pub mod projection_20;
pub mod result_set;
pub mod trace_8stages;

/// KG/AI HTTP 适配层：6 KG 真实接口 + 4 AI 引擎接口桩
/// （feature = "http-adapter" 时构建，见 [`http_adapter::build_kg_ai_router`]）
#[cfg(feature = "http-adapter")]
pub mod http_adapter;

// ---------- 核心类型重导出 ----------

pub use ac15_faults::{Ac15Fault, FaultInjector, FaultPoint, FaultReport, QualityGate};
pub use trace_8stages::*;
pub use algo_bridge::{AlgoBridge, Communities, Graph as AlgoGraph};
pub use cypher_parser::CypherParser;
pub use error::{GraphError, GraphResult};
pub use graph_server::{Direction, EdgeRow, GraphServer, Neighbor, StorageEngine};
pub use result_set::{PropValue, ResultSet};

// ---------- nGQL 解析器重导出 ----------

pub use ngql_parser::{
    NgqlParser, PlanNode,
    // 表达式系统
    Expression, ExpressionParser, UnaryOp, BinaryOp, CastType,
    // 函数注册表
    FunctionCategory, FunctionMeta, FunctionRegistry,
};

// ---------- 查询优化器重导出 ----------

pub use optimizer::{
    Optimizer, PlanOutput, DetailedPlanOutput,
    // 统计信息
    StatisticsManager, TagStatistics, EdgeStatistics, Histogram,
    // 代价模型
    CostModel, CostModelConfig, CostEstimate,
    // 连接顺序优化
    JoinOrderOptimizer, JoinRelation, JoinEdge,
    // 选择率估算
    SelectivityEstimator, CompareOp,
    // 优化规则
    CboOptimizer, OptimizationRule, RuleApplication,
    // 执行计划缓存
    PlanCache,
};

// ---------- 分布式查询执行引擎重导出 ----------

pub use graph_query_engine::{
    QueryEngine,
    // 执行配置与上下文
    ExecutionConfig, ExecutionContext,
    // 内存管理
    MemoryStats, MemoryStatsSnapshot,
    // 行批量（向量化执行）
    RowBatch,
    // 算子类型
    OperatorType,
    // 物理算子 trait
    PhysicalOperator,
    // Scan 算子
    ScanOperator, ScanType,
    // Filter 算子
    FilterOperator, FilterCondition,
    // Project 算子
    ProjectOperator, ProjectExpression, ArithmeticOp,
    // Join 算子
    HashJoinOperator, JoinType, JoinAlgorithm,
    // Aggregate 算子
    AggregateOperator, AggregateFunction, AggregateExpression,
    // Sort 算子
    SortOperator, SortDirection, SortKey,
    // Limit 算子
    LimitOperator,
    // Traverse 算子
    TraverseOperator, TraverseDirection,
    // Path 算子
    PathOperator, PathType,
};

// ---------- 图索引管理重导出 ----------

pub use graph_index::{
    // 索引管理核心
    IndexManager,
    // 索引类型与状态
    IndexType, IndexStatus,
    // 索引定义
    IndexDefinition, IndexColumn,
    // 索引使用统计
    IndexUsageStats, IndexUsageSnapshot,
    // 索引选择性
    IndexSelectivity, SelectivityGrade,
    // 索引选择器
    IndexSelector, IndexSelection,
    // B+ 树索引
    BPlusTreeIndex,
    // 全文索引
    InvertedIndex, PostingItem,
    // 向量索引
    VectorIndex, VectorIndexType, VectorIndexConfig, VectorSearchResult,
};

// ---------- 投影算子重导出 ----------

pub use projection_20::{
    projection_20_matrix, PROJECTION_OPERATORS, ProjectionContext, ProjectionOperator,
    ProjectionResult,
};

/// LPA helper（公共 API 已弃用）。
#[allow(deprecated)]
#[deprecated(
    since = "3.0.0",
    note = "LPA public API deprecated; use AlgoBridge::cnm for community detection."
)]
pub fn lpa_communities(_graph: &algo_bridge::Graph) -> Communities {
    Vec::new()
}
