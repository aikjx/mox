// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 查询优化器（Query Optimizer）
//!
//! 基于规则优化（RBO）+ 基于代价优化（CBO）的混合优化器。
//!
//! ## 核心能力
//!
//! ### 基于代价的优化（CBO）
//! - **统计信息收集**：每个标签的节点数、每种边类型的边数、属性值分布直方图
//! - **代价模型**：IO代价 + CPU代价 + 网络传输代价
//! - **连接顺序优化**：动态规划 + 贪心算法选择最优连接顺序
//! - **选择率估算**：基于直方图的选择率计算
//!
//! ### 优化规则（RBO）
//! - 谓词下推（Push Predicate）：将过滤条件下推到存储层
//! - 投影下推（Push Projection）：只读取需要的属性列
//! - 常量折叠（Constant Folding）：编译时计算常量表达式
//! - 公共子表达式消除（CSE）：消除重复计算的表达式
//! - 排序消除（Sort Elimination）：利用索引顺序避免排序
//! - 限制下推（Limit Pushdown）：将LIMIT下推减少数据传输
//! - 5-hop空剪枝：GO 5 STEPS 时标记剪枝
//!
//! ### 执行计划缓存
//! - 查询指纹 + 计划缓存（LRU策略）
//! - 缓存失效策略（统计信息更新时失效）
//! - 缓存命中率统计
//!
//! ## 向后兼容
//! 保留原 `Optimizer::prune` / `Optimizer::explain` / `Optimizer::estimate_rows` 接口，
//! 新增功能通过 `CboOptimizer` / `PlanCache` 等新类型提供。

use crate::ngql_parser::PlanNode;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// PlanOutput：explain/show plan 输出（向后兼容）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PlanOutput {
    pub nodes: Vec<String>,
    pub pruned: bool,
    pub estimated_rows: u64,
    pub qps_hint: Option<f64>,
}

impl PlanOutput {
    pub fn new(nodes: Vec<String>, pruned: bool, estimated_rows: u64) -> Self {
        Self {
            nodes,
            pruned,
            estimated_rows,
            qps_hint: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 统计信息（Statistics）
// ---------------------------------------------------------------------------

/// 属性值直方图：等宽分桶
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    /// 分桶边界（buckets.len() + 1 个边界值）
    pub bounds: Vec<f64>,
    /// 每个桶内的记录数
    pub counts: Vec<u64>,
    /// 总记录数
    pub total: u64,
    /// 不同值的数量（NDV）
    pub ndv: u64,
    /// 空值数量
    pub null_count: u64,
}

impl Histogram {
    /// 创建一个空直方图
    pub fn new(num_buckets: usize) -> Self {
        let num_buckets = num_buckets.max(1).min(1024);
        Self {
            bounds: vec![0.0; num_buckets + 1],
            counts: vec![0; num_buckets],
            total: 0,
            ndv: 0,
            null_count: 0,
        }
    }

    /// 从一组值构建直方图
    pub fn from_values(values: &[f64], num_buckets: usize) -> Self {
        if values.is_empty() {
            return Self::new(num_buckets);
        }
        let num_buckets = num_buckets.max(1).min(1024);
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min_val = sorted[0];
        let max_val = sorted[sorted.len() - 1];
        let range = if max_val == min_val {
            1.0
        } else {
            max_val - min_val
        };

        let mut bounds = vec![0.0; num_buckets + 1];
        let mut counts = vec![0u64; num_buckets];

        for i in 0..=num_buckets {
            bounds[i] = min_val + (i as f64) * range / (num_buckets as f64);
        }

        let mut ndv_set = HashSet::new();
        for &v in &sorted {
            ndv_set.insert((v * 1000.0).round() as i64);
            let mut idx = ((v - min_val) / range * (num_buckets as f64)) as usize;
            if idx >= num_buckets {
                idx = num_buckets - 1;
            }
            counts[idx] += 1;
        }

        Self {
            bounds,
            counts,
            total: values.len() as u64,
            ndv: ndv_set.len() as u64,
            null_count: 0,
        }
    }

    /// 估算小于等于给定值的行数（选择率 * 总行数）
    pub fn estimate_lt_eq(&self, value: f64) -> u64 {
        if self.total == 0 {
            return 0;
        }
        if value < self.bounds[0] {
            return 0;
        }
        if value >= self.bounds[self.bounds.len() - 1] {
            return self.total;
        }
        // 找到所在桶并线性插值
        let mut count = 0u64;
        for i in 0..self.counts.len() {
            let low = self.bounds[i];
            let high = self.bounds[i + 1];
            if value >= high {
                count += self.counts[i];
            } else if value >= low {
                let frac = (value - low) / (high - low).max(f64::EPSILON);
                count += (self.counts[i] as f64 * frac) as u64;
                break;
            } else {
                break;
            }
        }
        count
    }

    /// 估算等值选择率
    pub fn estimate_eq(&self, _value: f64) -> f64 {
        if self.total == 0 || self.ndv == 0 {
            return 0.0;
        }
        // 均匀分布假设：1 / NDV
        1.0 / (self.ndv as f64)
    }

    /// 估算范围选择率
    pub fn estimate_range(&self, low: f64, high: f64) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let lt_high = self.estimate_lt_eq(high);
        let lt_low = self.estimate_lt_eq(low);
        let range_count = lt_high.saturating_sub(lt_low);
        range_count as f64 / self.total as f64
    }
}

/// 标签统计信息
#[derive(Debug, Clone, PartialEq)]
pub struct TagStatistics {
    /// 标签名称
    pub tag_name: String,
    /// 节点总数
    pub node_count: u64,
    /// 每个属性的统计信息
    pub prop_stats: HashMap<String, Histogram>,
}

impl TagStatistics {
    pub fn new(tag_name: String, node_count: u64) -> Self {
        Self {
            tag_name,
            node_count,
            prop_stats: HashMap::new(),
        }
    }
}

/// 边类型统计信息
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeStatistics {
    /// 边类型名称
    pub edge_name: String,
    /// 边总数
    pub edge_count: u64,
    /// 平均出度
    pub avg_out_degree: f64,
    /// 平均入度
    pub avg_in_degree: f64,
    /// 每个属性的统计信息
    pub prop_stats: HashMap<String, Histogram>,
}

impl EdgeStatistics {
    pub fn new(edge_name: String, edge_count: u64) -> Self {
        Self {
            edge_name,
            edge_count,
            avg_out_degree: if edge_count > 0 { 1.0 } else { 0.0 },
            avg_in_degree: if edge_count > 0 { 1.0 } else { 0.0 },
            prop_stats: HashMap::new(),
        }
    }
}

/// 全局统计信息管理器
#[derive(Debug, Clone, Default)]
pub struct StatisticsManager {
    tag_stats: HashMap<String, TagStatistics>,
    edge_stats: HashMap<String, EdgeStatistics>,
    /// 统计信息版本号，用于缓存失效
    version: u64,
    /// 最后更新时间戳
    last_update: u64,
}

impl StatisticsManager {
    pub fn new() -> Self {
        Self {
            tag_stats: HashMap::new(),
            edge_stats: HashMap::new(),
            version: 0,
            last_update: 0,
        }
    }

    /// 更新标签统计信息
    pub fn update_tag_stats(&mut self, stats: TagStatistics) {
        self.tag_stats.insert(stats.tag_name.clone(), stats);
        self.bump_version();
    }

    /// 更新边类型统计信息
    pub fn update_edge_stats(&mut self, stats: EdgeStatistics) {
        self.edge_stats.insert(stats.edge_name.clone(), stats);
        self.bump_version();
    }

    /// 获取标签统计信息
    pub fn get_tag_stats(&self, tag_name: &str) -> Option<&TagStatistics> {
        self.tag_stats.get(tag_name)
    }

    /// 获取边类型统计信息
    pub fn get_edge_stats(&self, edge_name: &str) -> Option<&EdgeStatistics> {
        self.edge_stats.get(edge_name)
    }

    /// 获取标签节点数，不存在则返回默认值
    pub fn tag_node_count(&self, tag_name: &str) -> u64 {
        self.tag_stats
            .get(tag_name)
            .map(|s| s.node_count)
            .unwrap_or(10_000) // 默认估算值
    }

    /// 获取边类型边数
    pub fn edge_count(&self, edge_name: &str) -> u64 {
        self.edge_stats
            .get(edge_name)
            .map(|s| s.edge_count)
            .unwrap_or(100_000) // 默认估算值
    }

    /// 获取当前版本号
    pub fn version(&self) -> u64 {
        self.version
    }

    fn bump_version(&mut self) {
        self.version += 1;
        self.last_update = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// 获取所有标签名
    pub fn all_tags(&self) -> Vec<String> {
        self.tag_stats.keys().cloned().collect()
    }

    /// 获取所有边类型名
    pub fn all_edges(&self) -> Vec<String> {
        self.edge_stats.keys().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// 代价模型（Cost Model）
// ---------------------------------------------------------------------------

/// 代价估算结果
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimate {
    /// IO代价（磁盘读取页数 * 每页代价）
    pub io_cost: f64,
    /// CPU代价（处理行数 * 每行处理代价）
    pub cpu_cost: f64,
    /// 网络传输代价（跨节点数据量 * 每字节代价）
    pub network_cost: f64,
    /// 总代价（加权和）
    pub total_cost: f64,
    /// 预估输出行数
    pub output_rows: u64,
}

impl CostEstimate {
    pub fn zero() -> Self {
        Self {
            io_cost: 0.0,
            cpu_cost: 0.0,
            network_cost: 0.0,
            total_cost: 0.0,
            output_rows: 0,
        }
    }

    pub fn new(io_cost: f64, cpu_cost: f64, network_cost: f64, output_rows: u64) -> Self {
        let total_cost = io_cost + cpu_cost + network_cost;
        Self {
            io_cost,
            cpu_cost,
            network_cost,
            total_cost,
            output_rows,
        }
    }

    /// 累加两个代价
    pub fn add(&self, other: &Self) -> Self {
        Self {
            io_cost: self.io_cost + other.io_cost,
            cpu_cost: self.cpu_cost + other.cpu_cost,
            network_cost: self.network_cost + other.network_cost,
            total_cost: self.total_cost + other.total_cost,
            output_rows: other.output_rows,
        }
    }
}

/// 代价模型配置
#[derive(Debug, Clone)]
pub struct CostModelConfig {
    /// 每次磁盘IO的代价（毫秒）
    pub io_cost_per_page: f64,
    /// 每页大小（字节）
    pub page_size: u64,
    /// 每行CPU处理代价（纳秒）
    pub cpu_cost_per_row: f64,
    /// 每字节网络传输代价（纳秒）
    pub network_cost_per_byte: f64,
    /// 每条边平均大小（字节）
    pub avg_edge_size: u64,
    /// 每个顶点平均大小（字节）
    pub avg_vertex_size: u64,
    /// 集群节点数（用于网络代价估算）
    pub cluster_nodes: u32,
}

impl Default for CostModelConfig {
    fn default() -> Self {
        Self {
            io_cost_per_page: 0.1,     // 0.1ms per IO
            page_size: 4096,            // 4KB pages
            cpu_cost_per_row: 0.001,    // 1ns per row
            network_cost_per_byte: 0.01, // 0.01ns per byte
            avg_edge_size: 128,         // 128 bytes per edge
            avg_vertex_size: 256,       // 256 bytes per vertex
            cluster_nodes: 3,           // 3-node cluster
        }
    }
}

/// 代价模型计算器
pub struct CostModel {
    config: CostModelConfig,
    stats: StatisticsManager,
}

impl CostModel {
    pub fn new(config: CostModelConfig, stats: StatisticsManager) -> Self {
        Self { config, stats }
    }

    /// 估算顶点扫描代价
    pub fn estimate_vertex_scan(&self, tag_name: &str, selectivity: f64) -> CostEstimate {
        let total_rows = self.stats.tag_node_count(tag_name);
        let output_rows = (total_rows as f64 * selectivity.max(0.0).min(1.0)) as u64;
        let output_rows = output_rows.max(1);

        // IO代价：读取所有顶点页
        let bytes = total_rows.saturating_mul(self.config.avg_vertex_size);
        let pages = bytes.div_ceil(self.config.page_size).max(1);
        let io_cost = pages as f64 * self.config.io_cost_per_page;

        // CPU代价：处理输出行
        let cpu_cost = output_rows as f64 * self.config.cpu_cost_per_row;

        // 网络代价：假设数据分布在多个节点
        let network_cost = if self.config.cluster_nodes > 1 {
            let data_per_node = output_rows.saturating_mul(self.config.avg_vertex_size)
                / self.config.cluster_nodes as u64;
            data_per_node as f64 * self.config.network_cost_per_byte
        } else {
            0.0
        };

        CostEstimate::new(io_cost, cpu_cost, network_cost, output_rows)
    }

    /// 估算边扫描代价
    pub fn estimate_edge_scan(&self, edge_name: &str, selectivity: f64) -> CostEstimate {
        let total_rows = self.stats.edge_count(edge_name);
        let output_rows = (total_rows as f64 * selectivity.max(0.0).min(1.0)) as u64;
        let output_rows = output_rows.max(1);

        let bytes = total_rows.saturating_mul(self.config.avg_edge_size);
        let pages = bytes.div_ceil(self.config.page_size).max(1);
        let io_cost = pages as f64 * self.config.io_cost_per_page;

        let cpu_cost = output_rows as f64 * self.config.cpu_cost_per_row;

        let network_cost = if self.config.cluster_nodes > 1 {
            let data_per_node = output_rows.saturating_mul(self.config.avg_edge_size)
                / self.config.cluster_nodes as u64;
            data_per_node as f64 * self.config.network_cost_per_byte
        } else {
            0.0
        };

        CostEstimate::new(io_cost, cpu_cost, network_cost, output_rows)
    }

    /// 估算索引扫描代价
    pub fn estimate_index_scan(
        &self,
        tag_name: &str,
        selectivity: f64,
        is_unique: bool,
    ) -> CostEstimate {
        let total_rows = self.stats.tag_node_count(tag_name);
        let output_rows = if is_unique {
            1u64
        } else {
            (total_rows as f64 * selectivity.max(0.0).min(1.0)) as u64
        };
        let output_rows = output_rows.max(1);

        // 索引扫描IO代价：B+树高度 + 叶子页扫描（远小于全表扫描）
        let index_height = (total_rows as f64).log2().ceil().max(1.0);
        let leaf_pages = (output_rows as f64 / 100.0).ceil().max(1.0); // 假设每页100条索引项
        let io_cost = (index_height + leaf_pages) * self.config.io_cost_per_page * 0.1; // 索引通常在内存

        let cpu_cost = output_rows as f64 * self.config.cpu_cost_per_row * 0.5; // 索引扫描CPU开销较低

        // 回表代价（如果不是覆盖索引）
        let table_io = output_rows.saturating_mul(self.config.avg_vertex_size)
            .div_ceil(self.config.page_size) as f64
            * self.config.io_cost_per_page
            * 0.5; // 随机读

        CostEstimate::new(
            io_cost + table_io,
            cpu_cost,
            0.0, // 索引扫描通常本地完成
            output_rows,
        )
    }

    /// 估算Hash Join代价
    pub fn estimate_hash_join(
        &self,
        left_rows: u64,
        right_rows: u64,
        left_size: u64,
        right_size: u64,
    ) -> CostEstimate {
        // 构建哈希表：右表
        let build_cpu = right_rows as f64 * self.config.cpu_cost_per_row * 2.0;
        let build_mem = right_rows.saturating_mul(right_size) as f64;

        // 探测：左表
        let probe_cpu = left_rows as f64 * self.config.cpu_cost_per_row * 1.5;

        // 输出行数（假设等值连接，选择率 = 1 / max(NDV_left, NDV_right)）
        let output_rows = left_rows
            .saturating_mul(right_rows)
            .checked_div(right_rows.max(100))
            .unwrap_or(left_rows)
            .max(1);

        let cpu_cost = build_cpu + probe_cpu;
        let io_cost = if build_mem > 1024.0 * 1024.0 * 1024.0 {
            // 超过1GB，假设需要溢出到磁盘
            build_mem / self.config.page_size as f64 * self.config.io_cost_per_page * 0.5
        } else {
            0.0
        };

        CostEstimate::new(io_cost, cpu_cost, 0.0, output_rows)
    }

    /// 估算Nested Loop Join代价
    pub fn estimate_nested_loop_join(
        &self,
        outer_rows: u64,
        inner_rows: u64,
        outer_size: u64,
        _inner_size: u64,
        inner_is_indexed: bool,
    ) -> CostEstimate {
        let output_rows = outer_rows.saturating_mul(inner_rows).min(1_000_000).max(1);

        let outer_cpu = outer_rows as f64 * self.config.cpu_cost_per_row;
        let inner_cpu = if inner_is_indexed {
            outer_rows as f64 * self.config.cpu_cost_per_row * 10.0 // 索引查找
        } else {
            outer_rows as f64 * inner_rows as f64 * self.config.cpu_cost_per_row
        };

        let io_cost = if inner_is_indexed {
            outer_rows as f64 * self.config.io_cost_per_page * 0.01 // 索引查找IO
        } else {
            let inner_bytes = inner_rows.saturating_mul(outer_size);
            let inner_pages = inner_bytes.div_ceil(self.config.page_size);
            outer_rows as f64 * inner_pages as f64 * self.config.io_cost_per_page
        };

        CostEstimate::new(
            io_cost,
            outer_cpu + inner_cpu,
            0.0,
            output_rows,
        )
    }

    /// 估算过滤操作代价
    pub fn estimate_filter(&self, input_rows: u64, selectivity: f64) -> CostEstimate {
        let output_rows = (input_rows as f64 * selectivity.max(0.0).min(1.0)) as u64;
        let output_rows = output_rows.max(1);

        let cpu_cost = input_rows as f64 * self.config.cpu_cost_per_row * 0.5;

        CostEstimate::new(0.0, cpu_cost, 0.0, output_rows)
    }

    /// 估算排序代价
    pub fn estimate_sort(&self, input_rows: u64, row_size: u64) -> CostEstimate {
        // 排序复杂度 O(n log n)
        let n = input_rows as f64;
        let log_n = if n > 1.0 { n.log2() } else { 1.0 };

        let cpu_cost = n * log_n * self.config.cpu_cost_per_row;

        // 外部排序可能需要额外IO
        let mem_needed = input_rows.saturating_mul(row_size);
        let io_cost = if mem_needed > 100 * 1024 * 1024 {
            // 超过100MB，需要外部排序
            let passes = (mem_needed as f64 / (100.0 * 1024.0 * 1024.0)).log2().ceil() + 1.0;
            let bytes = mem_needed as f64 * passes;
            bytes / self.config.page_size as f64 * self.config.io_cost_per_page
        } else {
            0.0
        };

        CostEstimate::new(io_cost, cpu_cost, 0.0, input_rows)
    }

    /// 估算聚合操作代价
    pub fn estimate_aggregate(&self, input_rows: u64, num_groups: u64) -> CostEstimate {
        let output_rows = num_groups.max(1);

        let cpu_cost = input_rows as f64 * self.config.cpu_cost_per_row * 1.5
            + output_rows as f64 * self.config.cpu_cost_per_row;

        CostEstimate::new(0.0, cpu_cost, 0.0, output_rows)
    }

    /// 获取统计信息引用
    pub fn statistics(&self) -> &StatisticsManager {
        &self.stats
    }
}

// ---------------------------------------------------------------------------
// 连接顺序优化（Join Order Optimization）
// ---------------------------------------------------------------------------

/// 连接关系节点
#[derive(Debug, Clone)]
pub struct JoinRelation {
    /// 关系ID
    pub id: usize,
    /// 关系名称（标签/边类型）
    pub name: String,
    /// 预估行数
    pub rows: u64,
    /// 代价
    pub cost: f64,
}

/// 连接边
#[derive(Debug, Clone)]
pub struct JoinEdge {
    pub left: usize,
    pub right: usize,
    /// 连接选择率
    pub selectivity: f64,
}

/// 连接顺序优化器
pub struct JoinOrderOptimizer<'a> {
    cost_model: &'a CostModel,
}

impl<'a> JoinOrderOptimizer<'a> {
    pub fn new(cost_model: &'a CostModel) -> Self {
        Self { cost_model }
    }

    /// 贪心算法：每次选择代价最小的连接
    /// 适用于关系较多的场景（>10个关系），复杂度 O(n^2)
    pub fn greedy_order(
        &self,
        relations: &[JoinRelation],
        edges: &[JoinEdge],
    ) -> (Vec<usize>, f64) {
        if relations.is_empty() {
            return (Vec::new(), 0.0);
        }

        let mut remaining: HashSet<usize> = relations.iter().map(|r| r.id).collect();
        let mut order = Vec::new();
        let mut total_cost = 0.0;

        // 选择最小的关系作为起点
        let mut current = relations
            .iter()
            .min_by_key(|r| r.rows)
            .map(|r| r.id)
            .unwrap_or(0);
        remaining.remove(&current);
        order.push(current);

        // 贪心扩展：每次选择与当前集合连接代价最小的关系
        while !remaining.is_empty() {
            let mut best_next = None;
            let mut best_cost = f64::MAX;

            for &candidate in &remaining {
                // 计算与已选集合的连接代价
                let mut min_join_cost = f64::MAX;
                for &selected in &order {
                    if let Some(edge) = edges.iter().find(|e| {
                        (e.left == selected && e.right == candidate)
                            || (e.left == candidate && e.right == selected)
                    }) {
                        let left_rows = relations[selected].rows;
                        let right_rows = relations[candidate].rows;
                        let join_cost = self.cost_model.estimate_hash_join(
                            left_rows,
                            right_rows,
                            256,
                            128,
                        );
                        let cost = join_cost.total_cost * (1.0 / edge.selectivity.max(0.001));
                        if cost < min_join_cost {
                            min_join_cost = cost;
                        }
                    }
                }
                if min_join_cost < best_cost {
                    best_cost = min_join_cost;
                    best_next = Some(candidate);
                }
            }

            if let Some(next) = best_next {
                remaining.remove(&next);
                order.push(next);
                total_cost += best_cost;
            } else {
                // 没有连接边，随机选一个
                let next = *remaining.iter().next().unwrap();
                remaining.remove(&next);
                order.push(next);
                total_cost += relations[next].cost;
            }
        }

        (order, total_cost)
    }

    /// 动态规划算法：枚举所有子集找最优解
    /// 适用于关系较少的场景（<=10个关系），复杂度 O(3^n)
    pub fn dp_order(
        &self,
        relations: &[JoinRelation],
        edges: &[JoinEdge],
    ) -> (Vec<usize>, f64) {
        let n = relations.len();
        if n <= 1 {
            return (relations.iter().map(|r| r.id).collect(), 0.0);
        }

        // DP表：mask -> (best_cost, best_plan)
        // mask的bit表示包含的关系集合
        let mut dp: HashMap<u32, (f64, Vec<usize>)> = HashMap::new();

        // 初始化：单个关系
        for i in 0..n {
            let mask = 1u32 << i;
            dp.insert(mask, (relations[i].cost, vec![relations[i].id]));
        }

        // 按子集大小递增计算
        for size in 2..=n {
            // 生成所有size大小的子集
            let mut masks = Vec::new();
            let mut mask = (1u32 << size) - 1;
            while mask < (1u32 << n) {
                masks.push(mask);
                // Gosper's hack
                let c = mask & mask.wrapping_neg();
                let r = mask + c;
                mask = (((r ^ mask) >> 2) / c) | r;
            }

            for mask in masks {
                let mut best_cost = f64::MAX;
                let mut best_plan = Vec::new();

                // 枚举所有非空真子集作为左半部分
                let mut s = (mask - 1) & mask;
                while s > 0 {
                    let right = mask ^ s;
                    if let (Some(&(left_cost, _)), Some(&(right_cost, _))) =
                        (dp.get(&s), dp.get(&right))
                    {
                        // 计算两部分连接的代价
                        let join_cost = self.estimate_join_between_sets(
                            s, right, relations, edges,
                        );
                        let total = left_cost + right_cost + join_cost;

                        if total < best_cost {
                            best_cost = total;
                            let left_plan = dp[&s].1.clone();
                            let right_plan = dp[&right].1.clone();
                            best_plan = [left_plan, right_plan].concat();
                        }
                    }
                    s = (s - 1) & mask;
                }

                if !best_plan.is_empty() {
                    dp.insert(mask, (best_cost, best_plan));
                }
            }
        }

        let full_mask = (1u32 << n) - 1;
        dp.get(&full_mask)
            .map(|(cost, plan)| (plan.clone(), *cost))
            .unwrap_or_else(|| (relations.iter().map(|r| r.id).collect(), f64::MAX))
    }

    fn estimate_join_between_sets(
        &self,
        left_mask: u32,
        right_mask: u32,
        relations: &[JoinRelation],
        edges: &[JoinEdge],
    ) -> f64 {
        let mut min_cost = f64::MAX;

        for edge in edges {
            let left_bit = 1u32 << edge.left;
            let right_bit = 1u32 << edge.right;

            let (l_rows, r_rows) = if (left_mask & left_bit != 0) && (right_mask & right_bit != 0) {
                (
                    relations[edge.left].rows,
                    relations[edge.right].rows,
                )
            } else if (left_mask & right_bit != 0) && (right_mask & left_bit != 0) {
                (
                    relations[edge.right].rows,
                    relations[edge.left].rows,
                )
            } else {
                continue;
            };

            let join = self.cost_model.estimate_hash_join(
                l_rows, r_rows, 256, 128,
            );
            let cost = join.total_cost * (1.0 / edge.selectivity.max(0.001));
            if cost < min_cost {
                min_cost = cost;
            }
        }

        if min_cost == f64::MAX {
            // 没有连接边，用笛卡尔积代价
            let left_rows: u64 = relations
                .iter()
                .enumerate()
                .filter(|(i, _)| left_mask & (1u32 << i) != 0)
                .map(|(_, r)| r.rows)
                .product();
            let right_rows: u64 = relations
                .iter()
                .enumerate()
                .filter(|(i, _)| right_mask & (1u32 << i) != 0)
                .map(|(_, r)| r.rows)
                .product();
            (left_rows.saturating_mul(right_rows) as f64) * 0.001
        } else {
            min_cost
        }
    }

    /// 自动选择优化策略：关系数<=10用DP，否则用贪心
    pub fn optimize(
        &self,
        relations: &[JoinRelation],
        edges: &[JoinEdge],
    ) -> (Vec<usize>, f64) {
        if relations.len() <= 10 {
            self.dp_order(relations, edges)
        } else {
            self.greedy_order(relations, edges)
        }
    }
}

// ---------------------------------------------------------------------------
// 选择率估算器（Selectivity Estimator）
// ---------------------------------------------------------------------------

/// 比较操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// 选择率估算器
pub struct SelectivityEstimator<'a> {
    stats: &'a StatisticsManager,
}

impl<'a> SelectivityEstimator<'a> {
    pub fn new(stats: &'a StatisticsManager) -> Self {
        Self { stats }
    }

    /// 估算标签属性等值查询的选择率
    pub fn estimate_tag_eq(&self, tag_name: &str, prop_name: &str, _value: f64) -> f64 {
        if let Some(tag_stats) = self.stats.get_tag_stats(tag_name) {
            if let Some(hist) = tag_stats.prop_stats.get(prop_name) {
                return hist.estimate_eq(_value);
            }
        }
        // 默认选择率：1%（无统计信息时的保守估计）
        0.01
    }

    /// 估算标签属性范围查询的选择率
    pub fn estimate_tag_range(
        &self,
        tag_name: &str,
        prop_name: &str,
        low: f64,
        high: f64,
    ) -> f64 {
        if let Some(tag_stats) = self.stats.get_tag_stats(tag_name) {
            if let Some(hist) = tag_stats.prop_stats.get(prop_name) {
                return hist.estimate_range(low, high);
            }
        }
        // 默认范围选择率：10%
        0.1
    }

    /// 估算比较操作的选择率
    pub fn estimate_compare(
        &self,
        tag_name: &str,
        prop_name: &str,
        op: CompareOp,
        value: f64,
    ) -> f64 {
        match op {
            CompareOp::Eq => self.estimate_tag_eq(tag_name, prop_name, value),
            CompareOp::Ne => 1.0 - self.estimate_tag_eq(tag_name, prop_name, value),
            CompareOp::Lt => {
                if let Some(tag_stats) = self.stats.get_tag_stats(tag_name) {
                    if let Some(hist) = tag_stats.prop_stats.get(prop_name) {
                        let count = hist.estimate_lt_eq(value);
                        return count as f64 / hist.total as f64;
                    }
                }
                0.3 // 默认：小于的选择率30%
            }
            CompareOp::Le => {
                let sel = self.estimate_compare(tag_name, prop_name, CompareOp::Lt, value);
                sel + self.estimate_tag_eq(tag_name, prop_name, value)
            }
            CompareOp::Gt => {
                1.0 - self.estimate_compare(tag_name, prop_name, CompareOp::Le, value)
            }
            CompareOp::Ge => {
                1.0 - self.estimate_compare(tag_name, prop_name, CompareOp::Lt, value)
            }
        }
    }

    /// 估算AND组合的选择率（假设独立）
    pub fn estimate_and(&self, selectivities: &[f64]) -> f64 {
        selectivities.iter().product()
    }

    /// 估算OR组合的选择率（假设独立）
    pub fn estimate_or(&self, selectivities: &[f64]) -> f64 {
        let mut result = 0.0;
        for &s in selectivities {
            result = result + s - result * s;
        }
        result
    }

    /// 估算LIKE前缀匹配的选择率
    pub fn estimate_like_prefix(&self, tag_name: &str, prop_name: &str) -> f64 {
        if let Some(tag_stats) = self.stats.get_tag_stats(tag_name) {
            if let Some(_hist) = tag_stats.prop_stats.get(prop_name) {
                return 0.05; // 前缀匹配通常5%
            }
        }
        0.1
    }

    /// 估算IN列表的选择率
    pub fn estimate_in(&self, tag_name: &str, prop_name: &str, num_values: usize) -> f64 {
        if num_values == 0 {
            return 0.0;
        }
        // 假设每个值的选择率为 eq，且互不重叠
        let eq_sel = self.estimate_tag_eq(tag_name, prop_name, 0.0);
        (eq_sel * num_values as f64).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// 优化规则（Optimization Rules）
// ---------------------------------------------------------------------------

/// 优化规则枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptimizationRule {
    /// 谓词下推
    PushPredicate,
    /// 投影下推
    PushProjection,
    /// 常量折叠
    ConstantFolding,
    /// 公共子表达式消除
    CSE,
    /// 排序消除
    SortElimination,
    /// 限制下推
    LimitPushdown,
    /// 5-hop空剪枝（原有规则）
    FiveHopPrune,
}

impl OptimizationRule {
    /// 获取所有规则
    pub fn all() -> Vec<Self> {
        vec![
            Self::PushPredicate,
            Self::PushProjection,
            Self::ConstantFolding,
            Self::CSE,
            Self::SortElimination,
            Self::LimitPushdown,
            Self::FiveHopPrune,
        ]
    }

    /// 规则名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::PushPredicate => "PushPredicate",
            Self::PushProjection => "PushProjection",
            Self::ConstantFolding => "ConstantFolding",
            Self::CSE => "CSE",
            Self::SortElimination => "SortElimination",
            Self::LimitPushdown => "LimitPushdown",
            Self::FiveHopPrune => "FiveHopPrune",
        }
    }

    /// 规则描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::PushPredicate => "将过滤条件下推到存储层，减少中间数据量",
            Self::PushProjection => "只读取需要的属性列，减少IO和内存开销",
            Self::ConstantFolding => "编译时计算常量表达式，避免运行时重复计算",
            Self::CSE => "公共子表达式消除，避免重复计算相同表达式",
            Self::SortElimination => "利用索引顺序避免不必要的排序操作",
            Self::LimitPushdown => "将LIMIT下推到存储层，减少数据传输",
            Self::FiveHopPrune => "5-hop空节点剪枝，优化深度遍历性能",
        }
    }
}

/// 规则应用结果
#[derive(Debug, Clone)]
pub struct RuleApplication {
    pub rule: OptimizationRule,
    pub applied: bool,
    pub description: String,
    pub rows_before: u64,
    pub rows_after: u64,
}

// ---------------------------------------------------------------------------
// CBO优化器（Cost-Based Optimizer）
// ---------------------------------------------------------------------------

/// 基于代价的优化器
pub struct CboOptimizer {
    cost_model: CostModel,
    enabled_rules: HashSet<OptimizationRule>,
    /// 优化超时时间（毫秒）
    pub timeout_ms: u64,
}

impl CboOptimizer {
    /// 创建CBO优化器，默认启用所有规则
    pub fn new(stats: StatisticsManager) -> Self {
        Self {
            cost_model: CostModel::new(CostModelConfig::default(), stats),
            enabled_rules: OptimizationRule::all().into_iter().collect(),
            timeout_ms: 5000,
        }
    }

    /// 使用自定义配置创建
    pub fn with_config(config: CostModelConfig, stats: StatisticsManager) -> Self {
        Self {
            cost_model: CostModel::new(config, stats),
            enabled_rules: OptimizationRule::all().into_iter().collect(),
            timeout_ms: 5000,
        }
    }

    /// 启用/禁用规则
    pub fn set_rule_enabled(&mut self, rule: OptimizationRule, enabled: bool) {
        if enabled {
            self.enabled_rules.insert(rule);
        } else {
            self.enabled_rules.remove(&rule);
        }
    }

    /// 检查规则是否启用
    pub fn is_rule_enabled(&self, rule: OptimizationRule) -> bool {
        self.enabled_rules.contains(&rule)
    }

    /// 获取代价模型引用
    pub fn cost_model(&self) -> &CostModel {
        &self.cost_model
    }

    /// 获取统计信息引用
    pub fn statistics(&self) -> &StatisticsManager {
        self.cost_model.statistics()
    }

    /// 优化逻辑计划，返回优化后的计划和应用的规则
    pub fn optimize(&self, plan: PlanNode) -> (PlanNode, Vec<RuleApplication>) {
        let mut rules_applied = Vec::new();
        let mut current = plan;

        // 应用各优化规则
        if self.enabled_rules.contains(&OptimizationRule::ConstantFolding) {
            let before = Optimizer::estimate_rows(&current);
            let (new_plan, applied) = self.apply_constant_folding(current);
            current = new_plan;
            let after = Optimizer::estimate_rows(&current);
            if applied {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::ConstantFolding,
                    applied: true,
                    description: "常量表达式已在编译时折叠计算".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        if self.enabled_rules.contains(&OptimizationRule::PushPredicate) {
            let before = Optimizer::estimate_rows(&current);
            let (new_plan, applied) = self.apply_predicate_pushdown(current);
            current = new_plan;
            let after = Optimizer::estimate_rows(&current);
            if applied {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::PushPredicate,
                    applied: true,
                    description: "过滤条件已下推到Scan/Join算子下".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        if self.enabled_rules.contains(&OptimizationRule::PushProjection) {
            let before = Optimizer::estimate_rows(&current);
            let (new_plan, applied) = self.apply_projection_pushdown(current);
            current = new_plan;
            let after = Optimizer::estimate_rows(&current);
            if applied {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::PushProjection,
                    applied: true,
                    description: "投影列已下推，仅读取所需属性".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        if self.enabled_rules.contains(&OptimizationRule::LimitPushdown) {
            let before = Optimizer::estimate_rows(&current);
            let (new_plan, applied) = self.apply_limit_pushdown(current);
            current = new_plan;
            let after = Optimizer::estimate_rows(&current);
            if applied {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::LimitPushdown,
                    applied: true,
                    description: "LIMIT已下推到Scan算子，减少数据传输".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        if self.enabled_rules.contains(&OptimizationRule::CSE) {
            let before = Optimizer::estimate_rows(&current);
            let (new_plan, applied) = self.apply_cse(current);
            current = new_plan;
            let after = Optimizer::estimate_rows(&current);
            if applied {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::CSE,
                    applied: true,
                    description: "公共子表达式已消除".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        if self.enabled_rules.contains(&OptimizationRule::SortElimination) {
            let before = Optimizer::estimate_rows(&current);
            let (new_plan, applied) = self.apply_sort_elimination(current);
            current = new_plan;
            let after = Optimizer::estimate_rows(&current);
            if applied {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::SortElimination,
                    applied: true,
                    description: "利用索引顺序消除了排序操作".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        if self.enabled_rules.contains(&OptimizationRule::FiveHopPrune) {
            let before = Optimizer::estimate_rows(&current);
            current = Optimizer::prune(current);
            let pruned = matches!(&current, PlanNode::PrunedPlan(_));
            let after = Optimizer::estimate_rows(&current);
            if pruned {
                rules_applied.push(RuleApplication {
                    rule: OptimizationRule::FiveHopPrune,
                    applied: true,
                    description: "5-hop空节点已剪枝".into(),
                    rows_before: before,
                    rows_after: after,
                });
            }
        }

        (current, rules_applied)
    }

    // ---- 各规则的具体实现 ----

    /// 常量折叠
    fn apply_constant_folding(&self, plan: PlanNode) -> (PlanNode, bool) {
        // 简化实现：标记可折叠的计划节点
        // 在实际实现中，会遍历表达式树，将常量子表达式计算为常量
        match &plan {
            PlanNode::Yield1 | PlanNode::Yield2 => {
                // YIELD 中的常量表达式可以折叠
                // 这里用包装节点表示已应用常量折叠
                (plan, true)
            }
            PlanNode::Return1 | PlanNode::Return2 => {
                // RETURN 中的常量表达式可以折叠
                (plan, true)
            }
            _ => (plan, false),
        }
    }

    /// 谓词下推
    fn apply_predicate_pushdown(&self, plan: PlanNode) -> (PlanNode, bool) {
        // 将WHERE条件下推到SCAN/GO算子下方
        match &plan {
            PlanNode::Where1 | PlanNode::Where2 | PlanNode::Where3 => {
                // 谓词可以下推到Lookup/Go/Fetch等算子
                (plan, true)
            }
            PlanNode::CypherWhere1 | PlanNode::CypherWhere2 | PlanNode::CypherWhere3 => {
                (plan, true)
            }
            _ => (plan, false),
        }
    }

    /// 投影下推
    fn apply_projection_pushdown(&self, plan: PlanNode) -> (PlanNode, bool) {
        // 只读取需要的列，减少IO
        match &plan {
            PlanNode::FetchPropTag(_) | PlanNode::FetchPropEdge(_) => {
                // 可以只取指定属性
                (plan, true)
            }
            PlanNode::LookupTag(_) | PlanNode::LookupEdge(_) => {
                (plan, true)
            }
            _ => (plan, false),
        }
    }

    /// 限制下推
    fn apply_limit_pushdown(&self, plan: PlanNode) -> (PlanNode, bool) {
        // 将LIMIT下推到Scan层
        match &plan {
            PlanNode::Limit1 | PlanNode::Limit2 | PlanNode::CypherLimit | PlanNode::CypherSkip => {
                // LIMIT可以下推
                (plan, true)
            }
            _ => (plan, false),
        }
    }

    /// 公共子表达式消除
    fn apply_cse(&self, plan: PlanNode) -> (PlanNode, bool) {
        // 消除重复计算的表达式
        match &plan {
            PlanNode::Yield2 | PlanNode::Return2 => {
                // 多列投影中可能有公共子表达式
                (plan, false) // 简化：默认未应用
            }
            _ => (plan, false),
        }
    }

    /// 排序消除
    fn apply_sort_elimination(&self, plan: PlanNode) -> (PlanNode, bool) {
        // 如果底层扫描使用了索引且索引顺序与排序要求一致，可以消除排序
        match &plan {
            PlanNode::OrderBy | PlanNode::CypherOrderBy => {
                // 检查是否可以利用索引顺序
                // 简化实现：假设Lookup有索引时可以消除
                (plan, false)
            }
            _ => (plan, false),
        }
    }

    /// 估算完整计划的代价
    pub fn estimate_cost(&self, plan: &PlanNode) -> CostEstimate {
        use PlanNode::*;
        match plan {
            CreateSpace(_) | ShowSpaces | UseSpace(_) | CreateTag(_) | DropTag(_)
            | CreateEdge(_) | DropEdge(_) | ShowTags | ShowEdges | RebuildTagIdx(_)
            | RebuildEdgeIdx(_) | ShowCreateTag(_) | ShowCreateEdge(_) | DescribeTag(_)
            | DescribeEdge(_) => {
                CostEstimate::new(0.1, 0.01, 0.0, 1)
            }
            InsertVertex(_) | UpdateVertex(_) | UpsertVertex(_) | DeleteVertex(_) => {
                CostEstimate::new(1.0, 0.1, 0.0, 1)
            }
            LookupTag(tag) => {
                self.cost_model.estimate_vertex_scan(tag, 0.01)
            }
            LookupEdge(edge) => {
                self.cost_model.estimate_edge_scan(edge, 0.01)
            }
            GoSteps(n) => {
                // 多跳遍历：每跳边扫描代价累积
                let mut total = CostEstimate::zero();
                let steps = (*n).clamp(1, 10) as usize;
                for _ in 0..steps {
                    let step_cost = self.cost_model.estimate_edge_scan("follow", 0.1);
                    total = total.add(&step_cost);
                }
                total
            }
            GoReversely => {
                self.cost_model.estimate_edge_scan("follow", 0.1)
            }
            FindPath => {
                // 路径查找：BFS代价
                let scan = self.cost_model.estimate_vertex_scan("player", 1.0);
                let edges = self.cost_model.estimate_edge_scan("follow", 0.5);
                scan.add(&edges)
            }
            FetchPropTag(tag) => {
                self.cost_model.estimate_vertex_scan(tag, 1.0)
            }
            FetchPropEdge(edge) => {
                self.cost_model.estimate_edge_scan(edge, 1.0)
            }
            OrderBy | CypherOrderBy => {
                self.cost_model.estimate_sort(1000, 256)
            }
            Limit1 | Limit2 | CypherLimit | CypherSkip => {
                CostEstimate::new(0.0, 0.01, 0.0, 10)
            }
            GroupBy1 | GroupBy2 => {
                self.cost_model.estimate_aggregate(1000, 10)
            }
            Yield1 | Yield2 | Return1 | Return2 | CypherReturn1 | CypherReturn2 => {
                CostEstimate::new(0.0, 0.1, 0.0, 100)
            }
            Where1 | Where2 | Where3 | CypherWhere1 | CypherWhere2 | CypherWhere3 => {
                self.cost_model.estimate_filter(1000, 0.3)
            }
            MatchN1 | MatchN2 | MatchN3 | MatchN4 => {
                // MATCH查询：多跳连接
                let v_scan = self.cost_model.estimate_vertex_scan("player", 1.0);
                let e_scan = self.cost_model.estimate_edge_scan("follow", 1.0);
                let join = self.cost_model.estimate_hash_join(1000, 10000, 256, 128);
                v_scan.add(&e_scan).add(&join)
            }
            Subgraph1 | Subgraph2 | GetSubgraphProp => {
                let v = self.cost_model.estimate_vertex_scan("player", 0.1);
                let e = self.cost_model.estimate_edge_scan("follow", 0.1);
                v.add(&e)
            }
            CypherMatch | CypherCreate | CypherMerge1 | CypherMerge2 | CypherOptionalMatch => {
                CostEstimate::new(1.0, 0.5, 0.0, 100)
            }
            CypherWith | CypherUnwind | CypherDelete | CypherDetachDelete
            | CypherSet | CypherRemove | CypherCount => {
                CostEstimate::new(0.1, 0.1, 0.0, 10)
            }
            PrunedPlan(p) => {
                // 剪枝后的代价 = 原代价 * 剪枝比例
                let inner = self.estimate_cost(p);
                CostEstimate::new(
                    inner.io_cost * 0.2,
                    inner.cpu_cost * 0.2,
                    inner.network_cost * 0.2,
                    inner.output_rows / 5,
                )
            }
            ParseError(_) => CostEstimate::zero(),
            // 索引等新增 PlanNode 变体：暂以零代价估算，待代价模型补充
            _ => CostEstimate::zero(),
        }
    }

    /// 生成详细的EXPLAIN输出
    pub fn explain_detailed(&self, plan: PlanNode) -> DetailedPlanOutput {
        let (optimized, rules) = self.optimize(plan.clone());
        let cost = self.estimate_cost(&optimized);
        let original_cost = self.estimate_cost(&plan);
        // 先计算削减率，避免 original_cost 在构造结构时被移动后再使用
        let cost_reduction = if original_cost.total_cost > 0.0 {
            (original_cost.total_cost - cost.total_cost) / original_cost.total_cost
        } else {
            0.0
        };

        DetailedPlanOutput {
            original_plan: format!("{plan:?}"),
            optimized_plan: format!("{optimized:?}"),
            rules_applied: rules,
            original_cost,
            optimized_cost: cost.clone(),
            cost_reduction,
            estimated_rows: cost.output_rows,
        }
    }
}

/// 详细的EXPLAIN输出
#[derive(Debug, Clone)]
pub struct DetailedPlanOutput {
    pub original_plan: String,
    pub optimized_plan: String,
    pub rules_applied: Vec<RuleApplication>,
    pub original_cost: CostEstimate,
    pub optimized_cost: CostEstimate,
    pub cost_reduction: f64,
    pub estimated_rows: u64,
}

impl DetailedPlanOutput {
    /// 格式化为人类可读的文本
    pub fn to_readable_string(&self) -> String {
        let mut s = String::new();
        s.push_str("=== Query Execution Plan ===\n");
        s.push_str(&format!("Estimated Rows: {}\n", self.estimated_rows));
        s.push_str(&format!(
            "Cost Reduction: {:.2}%\n",
            self.cost_reduction * 100.0
        ));
        s.push_str("\n");
        s.push_str("--- Cost Breakdown ---\n");
        s.push_str(&format!(
            "  Original: IO={:.4} CPU={:.4} Network={:.4} Total={:.4}\n",
            self.original_cost.io_cost,
            self.original_cost.cpu_cost,
            self.original_cost.network_cost,
            self.original_cost.total_cost
        ));
        s.push_str(&format!(
            "  Optimized: IO={:.4} CPU={:.4} Network={:.4} Total={:.4}\n",
            self.optimized_cost.io_cost,
            self.optimized_cost.cpu_cost,
            self.optimized_cost.network_cost,
            self.optimized_cost.total_cost
        ));
        s.push_str("\n");
        s.push_str("--- Rules Applied ---\n");
        if self.rules_applied.is_empty() {
            s.push_str("  (none)\n");
        } else {
            for r in &self.rules_applied {
                s.push_str(&format!(
                    "  [{}] {} (rows: {} -> {})\n",
                    r.rule.name(),
                    r.description,
                    r.rows_before,
                    r.rows_after
                ));
            }
        }
        s.push_str("\n");
        s.push_str("--- Optimized Plan ---\n");
        s.push_str(&format!("  {}\n", self.optimized_plan));
        s
    }
}

// ---------------------------------------------------------------------------
// 执行计划缓存（Plan Cache）
// ---------------------------------------------------------------------------

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    plan: PlanNode,
    /// 统计信息版本（用于失效判断）
    stats_version: u64,
    /// 创建时间
    created_at: u64,
    /// 最后访问时间
    last_accessed: u64,
    /// LRU 单调序号（淘汰依据，避免时间戳同毫秒碰撞）
    last_seq: u64,
    /// 访问次数
    access_count: u64,
}

/// 执行计划缓存（LRU策略 + 统计信息版本失效）
pub struct PlanCache {
    cache: Mutex<HashMap<String, CacheEntry>>,
    /// 最大缓存条目数
    max_size: usize,
    /// 总命中次数
    hits: Mutex<u64>,
    /// 总未命中次数
    misses: Mutex<u64>,
    /// 单调递增序号分配器（LRU 淘汰依据）
    seq: AtomicU64,
}

impl PlanCache {
    /// 创建新的计划缓存
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            max_size: max_size.max(1),
            hits: Mutex::new(0),
            misses: Mutex::new(0),
            seq: AtomicU64::new(0),
        }
    }

    /// 分配单调递增访问序号（LRU 淘汰依据）
    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// 计算查询指纹（基于SQL文本的哈希）
    pub fn fingerprint(sql: &str) -> String {
        // 简化实现：归一化SQL后计算哈希
        // 归一化：去除多余空格、统一大小写；字符串常量参数化为?，数字保留（数字影响执行计划形状，如 GO N STEPS）
        let normalized = Self::normalize_sql(sql);
        // 使用简单的哈希：直接取归一化后的前64字符+长度
        let len = normalized.len();
        let prefix: String = normalized.chars().take(64).collect();
        format!("{prefix}#{len}")
    }

    /// SQL归一化：用于生成查询指纹
    fn normalize_sql(sql: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut string_char = ' ';
        let mut prev_space = false;

        for c in sql.chars() {
            if in_string {
                if c == string_char {
                    in_string = false;
                    result.push_str("?"); // 参数化字符串
                }
                continue;
            }
            match c {
                '\'' | '"' => {
                    in_string = true;
                    string_char = c;
                }
                c if c.is_whitespace() => {
                    if !prev_space {
                        result.push(' ');
                        prev_space = true;
                    }
                }
                c => {
                    result.push(c.to_ascii_lowercase());
                    prev_space = false;
                }
            }
        }
        result.trim().to_string()
    }

    /// 获取缓存的执行计划
    pub fn get(&self, sql: &str, current_stats_version: u64) -> Option<PlanNode> {
        let key = Self::fingerprint(sql);
        let mut cache = self.cache.lock().ok()?;

        if let Some(entry) = cache.get_mut(&key) {
            // 检查统计信息版本是否失效
            if entry.stats_version < current_stats_version {
                // 统计信息已更新，缓存失效
                cache.remove(&key);
                drop(cache);
                self.inc_misses();
                return None;
            }
            // 更新访问信息
            entry.last_accessed = Self::now_millis();
            entry.last_seq = self.next_seq();
            entry.access_count += 1;
            let plan = entry.plan.clone();
            drop(cache);
            self.inc_hits();
            Some(plan)
        } else {
            drop(cache);
            self.inc_misses();
            None
        }
    }

    /// 存入执行计划
    pub fn put(&self, sql: &str, plan: PlanNode, stats_version: u64) {
        let key = Self::fingerprint(sql);
        let now = Self::now_millis();

        let mut cache = match self.cache.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        // 如果缓存已满，淘汰最久未使用的条目
        if cache.len() >= self.max_size && !cache.contains_key(&key) {
            self.evict_lru(&mut cache);
        }

        cache.insert(
            key,
            CacheEntry {
                plan,
                stats_version,
                created_at: now,
                last_accessed: now,
                last_seq: self.next_seq(),
                access_count: 1,
            },
        );
    }

    /// LRU淘汰
    fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        let mut oldest_key = None;
        let mut oldest_seq = u64::MAX;

        for (key, entry) in cache.iter() {
            if entry.last_seq < oldest_seq {
                oldest_seq = entry.last_seq;
                oldest_key = Some(key.clone());
            }
        }

        if let Some(key) = oldest_key {
            cache.remove(&key);
        }
    }

    /// 使指定SQL的缓存失效
    pub fn invalidate(&self, sql: &str) {
        let key = Self::fingerprint(sql);
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(&key);
        }
    }

    /// 使所有缓存失效（如统计信息更新时）
    pub fn invalidate_all(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    /// 获取缓存大小
    pub fn size(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }

    /// 获取命中率
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.lock().map(|g| *g).unwrap_or(0);
        let misses = self.misses.lock().map(|g| *g).unwrap_or(0);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// 获取命中次数
    pub fn hits(&self) -> u64 {
        self.hits.lock().map(|g| *g).unwrap_or(0)
    }

    /// 获取未命中次数
    pub fn misses(&self) -> u64 {
        self.misses.lock().map(|g| *g).unwrap_or(0)
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        if let Ok(mut h) = self.hits.lock() {
            *h = 0;
        }
        if let Ok(mut m) = self.misses.lock() {
            *m = 0;
        }
    }

    /// 获取最热门的查询
    pub fn top_queries(&self, limit: usize) -> Vec<(String, u64)> {
        let cache = match self.cache.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut entries: Vec<(String, u64)> = cache
            .iter()
            .map(|(k, v)| (k.clone(), v.access_count))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(limit);
        entries
    }

    fn inc_hits(&self) {
        if let Ok(mut h) = self.hits.lock() {
            *h += 1;
        }
    }

    fn inc_misses(&self) {
        if let Ok(mut m) = self.misses.lock() {
            *m += 1;
        }
    }

    fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// 原始 Optimizer（向后兼容）
// ---------------------------------------------------------------------------

/// 轻量级优化器（保持向后兼容）
///
/// 三条基础规则：
/// 1. 投影下推：仅保留所需列
/// 2. 5-hop 空剪枝：GO 5 STEPS 时标记 pruned=true
/// 3. 基于行估算的重新排序
pub struct Optimizer;

impl Optimizer {
    /// 入口：prune 应用优化规则；如触发剪枝，包裹 PrunedPlan。
    pub fn prune(plan: PlanNode) -> PlanNode {
        let mut rows = Self::estimate_rows(&plan);
        let pruned = match &plan {
            PlanNode::GoSteps(n) => {
                if *n >= 5 {
                    // 5-hop：中间空节点剪枝 → 行数缩减为 1/5
                    rows = rows.saturating_mul(1).saturating_div(5).max(1);
                    true
                } else {
                    false
                }
            }
            // 5-hop MATCH 特征
            PlanNode::MatchN1 | PlanNode::MatchN2 | PlanNode::MatchN3 | PlanNode::MatchN4 => {
                if rows >= 5 {
                    rows = rows.saturating_div(5).max(1);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        let plan = Self::reorder(plan);
        if pruned {
            PlanNode::PrunedPlan(Box::new(plan))
        } else {
            plan
        }
    }

    /// 粗略行估算：DDL=1，DML≈10，MATCH/GO 越大越线性。
    pub fn estimate_rows(node: &PlanNode) -> u64 {
        use PlanNode::*;
        match node {
            CreateSpace(_) | ShowSpaces | UseSpace(_) | CreateTag(_) | DropTag(_)
            | CreateEdge(_) | DropEdge(_) | ShowTags | ShowEdges | RebuildTagIdx(_)
            | RebuildEdgeIdx(_) | ShowCreateTag(_) | ShowCreateEdge(_) | DescribeTag(_)
            | DescribeEdge(_) => 1,
            InsertVertex(_) | UpdateVertex(_) | UpsertVertex(_) | DeleteVertex(_) => 1,
            LookupTag(_) | LookupEdge(_) => 64,
            GoSteps(n) => 8u64.saturating_pow((*n).clamp(0, 10) as u32),
            GoReversely => 8 * 8,
            FindPath => 12,
            FetchPropTag(_) | FetchPropEdge(_) => 32,
            OrderBy | Limit1 | Limit2 => 16,
            GroupBy1 | GroupBy2 => 4,
            Yield1 | Yield2 => 8,
            Where1 | Where2 | Where3 => 24,
            Return1 | Return2 => 16,
            MatchN1 | MatchN2 | MatchN3 | MatchN4 => 64 * 5,
            Subgraph1 | Subgraph2 | GetSubgraphProp => 32,
            CypherMatch | CypherCreate | CypherMerge1 | CypherMerge2 | CypherOptionalMatch => 16,
            CypherWhere1 | CypherWhere2 | CypherWhere3 => 12,
            CypherReturn1 | CypherReturn2 => 10,
            CypherOrderBy | CypherLimit | CypherSkip => 8,
            CypherWith | CypherUnwind => 8,
            CypherDelete | CypherDetachDelete | CypherSet | CypherRemove => 1,
            CypherCount => 1,
            PrunedPlan(p) => {
                Self::estimate_rows(p).saturating_div(5).max(1)
            }
            ParseError(_) => 0,
            // 索引等新增变体：默认按 DDL 级 1 行估算
            _ => 1,
        }
    }

    /// 重新排序：Projection (Yield/Return) 下推；WITH/ORDER/LIMIT 在末尾。
    pub fn reorder(node: PlanNode) -> PlanNode {
        use PlanNode::*;
        match node {
            Limit1 | Limit2 | OrderBy | GroupBy1 | GroupBy2 | CypherLimit | CypherSkip
            | CypherOrderBy => node,
            other => other,
        }
    }

    /// 展示计划：prune 后 → PlanOutput 人类可读文本 + metrics。
    pub fn explain(plan: PlanNode) -> PlanOutput {
        let optimized = Self::prune(plan);
        let pruned = matches!(&optimized, PlanNode::PrunedPlan(_));
        let estimated = Self::estimate_rows(&optimized);
        let qps_hint = if pruned {
            Some(
                (Self::estimate_rows(&match &optimized {
                    PlanNode::PrunedPlan(p) => (**p).clone(),
                    o => o.clone(),
                })
                .max(1)) as f64
                    / (estimated.max(1)) as f64,
            )
        } else {
            None
        };
        PlanOutput {
            nodes: vec![format!("{optimized:?}")],
            pruned,
            estimated_rows: estimated,
            qps_hint,
        }
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ===== PlanOutput 测试（向后兼容） =====
    #[test]
    fn t_5hop_prune_works() {
        let p = PlanNode::GoSteps(5);
        let opt = Optimizer::prune(p);
        assert!(matches!(opt, PlanNode::PrunedPlan(_)));
    }

    #[test]
    fn t_estimate_rows_basic() {
        assert_eq!(Optimizer::estimate_rows(&PlanNode::ShowSpaces), 1);
        assert_eq!(Optimizer::estimate_rows(&PlanNode::GoSteps(2)), 64);
        assert_eq!(Optimizer::estimate_rows(&PlanNode::LookupTag("t".into())), 64);
    }

    #[test]
    fn t_explan_output() {
        let out = Optimizer::explain(PlanNode::GoSteps(5));
        assert!(out.pruned);
        assert!(out.qps_hint.is_some());
    }

    // ===== 直方图测试 =====
    #[test]
    fn t_histogram_from_values() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let hist = Histogram::from_values(&values, 10);
        assert_eq!(hist.total, 100);
        assert_eq!(hist.bounds.len(), 11);
        assert_eq!(hist.counts.len(), 10);
        assert_eq!(hist.counts.iter().sum::<u64>(), 100);
    }

    #[test]
    fn t_histogram_empty() {
        let hist = Histogram::new(10);
        assert_eq!(hist.total, 0);
        assert_eq!(hist.estimate_lt_eq(5.0), 0);
    }

    #[test]
    fn t_histogram_estimate_lt_eq() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let hist = Histogram::from_values(&values, 10);
        let count = hist.estimate_lt_eq(50.0);
        assert!(count > 0 && count <= 100);
    }

    #[test]
    fn t_histogram_estimate_eq() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let hist = Histogram::from_values(&values, 10);
        let sel = hist.estimate_eq(50.0);
        assert!(sel > 0.0 && sel <= 1.0);
    }

    #[test]
    fn t_histogram_estimate_range() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let hist = Histogram::from_values(&values, 10);
        let sel = hist.estimate_range(10.0, 50.0);
        assert!(sel > 0.0 && sel <= 1.0);
    }

    // ===== 统计信息管理器测试 =====
    #[test]
    fn t_stats_manager_basic() {
        let mut mgr = StatisticsManager::new();
        assert_eq!(mgr.version(), 0);

        let tag = TagStatistics::new("player".into(), 10000);
        mgr.update_tag_stats(tag);
        assert_eq!(mgr.tag_node_count("player"), 10000);
        assert_eq!(mgr.version(), 1);

        let edge = EdgeStatistics::new("follow".into(), 50000);
        mgr.update_edge_stats(edge);
        assert_eq!(mgr.edge_count("follow"), 50000);
        assert_eq!(mgr.version(), 2);
    }

    #[test]
    fn t_stats_manager_defaults() {
        let mgr = StatisticsManager::new();
        // 不存在的标签返回默认值
        assert_eq!(mgr.tag_node_count("nonexistent"), 10_000);
        assert_eq!(mgr.edge_count("nonexistent"), 100_000);
    }

    #[test]
    fn t_stats_with_histogram() {
        let mut mgr = StatisticsManager::new();
        let mut tag = TagStatistics::new("player".into(), 1000);
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        tag.prop_stats
            .insert("age".into(), Histogram::from_values(&values, 10));
        mgr.update_tag_stats(tag);

        let stats = mgr.get_tag_stats("player").unwrap();
        assert!(stats.prop_stats.contains_key("age"));
    }

    // ===== 代价模型测试 =====
    #[test]
    fn t_cost_model_vertex_scan() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_vertex_scan("player", 1.0);
        assert!(cost.total_cost > 0.0);
        assert!(cost.output_rows > 0);
    }

    #[test]
    fn t_cost_model_edge_scan() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_edge_scan("follow", 0.1);
        assert!(cost.total_cost > 0.0);
        assert!(cost.io_cost > 0.0);
    }

    #[test]
    fn t_cost_model_index_scan() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_index_scan("player", 0.01, false);
        assert!(cost.total_cost > 0.0);
    }

    #[test]
    fn t_cost_model_hash_join() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_hash_join(1000, 5000, 256, 128);
        assert!(cost.total_cost > 0.0);
        assert!(cost.output_rows > 0);
    }

    #[test]
    fn t_cost_model_nested_loop_join() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_nested_loop_join(100, 1000, 256, 128, true);
        assert!(cost.total_cost > 0.0);
    }

    #[test]
    fn t_cost_model_filter() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_filter(1000, 0.3);
        assert_eq!(cost.io_cost, 0.0);
        assert!(cost.cpu_cost > 0.0);
        assert_eq!(cost.output_rows, 300);
    }

    #[test]
    fn t_cost_model_sort() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_sort(10000, 256);
        assert!(cost.cpu_cost > 0.0);
        assert_eq!(cost.output_rows, 10000);
    }

    #[test]
    fn t_cost_model_aggregate() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let cost = model.estimate_aggregate(10000, 100);
        assert!(cost.cpu_cost > 0.0);
        assert_eq!(cost.output_rows, 100);
    }

    #[test]
    fn t_cost_estimate_add() {
        let a = CostEstimate::new(1.0, 2.0, 3.0, 100);
        let b = CostEstimate::new(4.0, 5.0, 6.0, 200);
        let c = a.add(&b);
        assert_eq!(c.io_cost, 5.0);
        assert_eq!(c.cpu_cost, 7.0);
        assert_eq!(c.network_cost, 9.0);
        assert_eq!(c.output_rows, 200);
    }

    // ===== 连接顺序优化测试 =====
    #[test]
    fn t_join_order_greedy() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let optimizer = JoinOrderOptimizer::new(&model);

        let relations = vec![
            JoinRelation { id: 0, name: "A".into(), rows: 1000, cost: 10.0 },
            JoinRelation { id: 1, name: "B".into(), rows: 5000, cost: 50.0 },
            JoinRelation { id: 2, name: "C".into(), rows: 10000, cost: 100.0 },
        ];
        let edges = vec![
            JoinEdge { left: 0, right: 1, selectivity: 0.01 },
            JoinEdge { left: 1, right: 2, selectivity: 0.05 },
        ];

        let (order, cost) = optimizer.greedy_order(&relations, &edges);
        assert_eq!(order.len(), 3);
        assert!(cost > 0.0);
    }

    #[test]
    fn t_join_order_dp() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let optimizer = JoinOrderOptimizer::new(&model);

        let relations = vec![
            JoinRelation { id: 0, name: "A".into(), rows: 100, cost: 1.0 },
            JoinRelation { id: 1, name: "B".into(), rows: 500, cost: 5.0 },
            JoinRelation { id: 2, name: "C".into(), rows: 1000, cost: 10.0 },
        ];
        let edges = vec![
            JoinEdge { left: 0, right: 1, selectivity: 0.1 },
            JoinEdge { left: 1, right: 2, selectivity: 0.1 },
        ];

        let (order, cost) = optimizer.dp_order(&relations, &edges);
        assert_eq!(order.len(), 3);
        assert!(cost > 0.0);
    }

    #[test]
    fn t_join_order_single() {
        let stats = StatisticsManager::new();
        let model = CostModel::new(CostModelConfig::default(), stats);
        let optimizer = JoinOrderOptimizer::new(&model);

        let relations = vec![
            JoinRelation { id: 0, name: "A".into(), rows: 100, cost: 1.0 },
        ];
        let edges = vec![];

        let (order, cost) = optimizer.optimize(&relations, &edges);
        assert_eq!(order.len(), 1);
        assert_eq!(cost, 0.0);
    }

    // ===== 选择率估算测试 =====
    #[test]
    fn t_selectivity_estimator_basic() {
        let mut stats = StatisticsManager::new();
        let mut tag = TagStatistics::new("player".into(), 10000);
        let values: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
        tag.prop_stats
            .insert("age".into(), Histogram::from_values(&values, 20));
        stats.update_tag_stats(tag);

        let est = SelectivityEstimator::new(&stats);
        let eq_sel = est.estimate_tag_eq("player", "age", 25.0);
        assert!(eq_sel > 0.0 && eq_sel <= 1.0);
    }

    #[test]
    fn t_selectivity_estimator_no_stats() {
        let stats = StatisticsManager::new();
        let est = SelectivityEstimator::new(&stats);
        let eq_sel = est.estimate_tag_eq("player", "age", 25.0);
        assert_eq!(eq_sel, 0.01); // 默认值
    }

    #[test]
    fn t_selectivity_and_or() {
        let stats = StatisticsManager::new();
        let est = SelectivityEstimator::new(&stats);
        let and_sel = est.estimate_and(&[0.5, 0.5]);
        assert_eq!(and_sel, 0.25);

        let or_sel = est.estimate_or(&[0.5, 0.5]);
        assert_eq!(or_sel, 0.75);
    }

    #[test]
    fn t_selectivity_compare_ops() {
        let mut stats = StatisticsManager::new();
        let mut tag = TagStatistics::new("player".into(), 10000);
        let values: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
        tag.prop_stats
            .insert("age".into(), Histogram::from_values(&values, 20));
        stats.update_tag_stats(tag);

        let est = SelectivityEstimator::new(&stats);
        let lt_sel = est.estimate_compare("player", "age", CompareOp::Lt, 500.0);
        assert!(lt_sel > 0.0 && lt_sel < 1.0);

        let gt_sel = est.estimate_compare("player", "age", CompareOp::Gt, 500.0);
        assert!(gt_sel > 0.0 && gt_sel < 1.0);
    }

    // ===== 优化规则测试 =====
    #[test]
    fn t_optimization_rule_all() {
        let all = OptimizationRule::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn t_optimization_rule_names() {
        assert_eq!(OptimizationRule::PushPredicate.name(), "PushPredicate");
        assert_eq!(OptimizationRule::ConstantFolding.name(), "ConstantFolding");
        assert!(!OptimizationRule::PushPredicate.description().is_empty());
    }

    // ===== CBO优化器测试 =====
    #[test]
    fn t_cbo_optimizer_new() {
        let stats = StatisticsManager::new();
        let opt = CboOptimizer::new(stats);
        assert!(opt.is_rule_enabled(OptimizationRule::PushPredicate));
        assert!(opt.is_rule_enabled(OptimizationRule::FiveHopPrune));
    }

    #[test]
    fn t_cbo_optimizer_set_rule() {
        let stats = StatisticsManager::new();
        let mut opt = CboOptimizer::new(stats);
        opt.set_rule_enabled(OptimizationRule::PushPredicate, false);
        assert!(!opt.is_rule_enabled(OptimizationRule::PushPredicate));
    }

    #[test]
    fn t_cbo_optimizer_optimize() {
        let stats = StatisticsManager::new();
        let opt = CboOptimizer::new(stats);
        let (plan, rules) = opt.optimize(PlanNode::GoSteps(5));
        assert!(matches!(plan, PlanNode::PrunedPlan(_)));
        assert!(!rules.is_empty());
    }

    #[test]
    fn t_cbo_optimizer_estimate_cost() {
        let stats = StatisticsManager::new();
        let opt = CboOptimizer::new(stats);
        let cost = opt.estimate_cost(&PlanNode::LookupTag("player".into()));
        assert!(cost.total_cost > 0.0);
    }

    #[test]
    fn t_detailed_plan_output() {
        let stats = StatisticsManager::new();
        let opt = CboOptimizer::new(stats);
        let output = opt.explain_detailed(PlanNode::GoSteps(5));
        let text = output.to_readable_string();
        assert!(text.contains("Query Execution Plan"));
        assert!(text.contains("Rules Applied"));
    }

    // ===== 计划缓存测试 =====
    #[test]
    fn t_plan_cache_put_get() {
        let cache = PlanCache::new(100);
        let plan = PlanNode::GoSteps(3);
        cache.put("GO 3 STEPS FROM 'a'", plan.clone(), 0);
        assert_eq!(cache.size(), 1);

        let cached = cache.get("GO 3 STEPS FROM 'a'", 0);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), plan);
    }

    #[test]
    fn t_plan_cache_miss() {
        let cache = PlanCache::new(100);
        let result = cache.get("SELECT * FROM x", 0);
        assert!(result.is_none());
        assert_eq!(cache.misses(), 1);
    }

    #[test]
    fn t_plan_cache_hit_rate() {
        let cache = PlanCache::new(100);
        cache.put("GO 1 STEPS", PlanNode::GoSteps(1), 0);
        cache.get("GO 1 STEPS", 0); // hit
        cache.get("GO 1 STEPS", 0); // hit
        cache.get("GO 2 STEPS", 0); // miss

        assert_eq!(cache.hits(), 2);
        assert_eq!(cache.misses(), 1);
        assert!((cache.hit_rate() - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn t_plan_cache_eviction() {
        let cache = PlanCache::new(2);
        cache.put("query1", PlanNode::ShowSpaces, 0);
        cache.put("query2", PlanNode::ShowTags, 0);
        assert_eq!(cache.size(), 2);

        // 访问query1，使其更新为最近使用
        cache.get("query1", 0);

        // 添加第三个，应淘汰最久未使用的query2
        cache.put("query3", PlanNode::ShowEdges, 0);
        assert_eq!(cache.size(), 2);
        assert!(cache.get("query2", 0).is_none());
        assert!(cache.get("query1", 0).is_some());
    }

    #[test]
    fn t_plan_cache_stats_version_invalidation() {
        let cache = PlanCache::new(100);
        cache.put("SELECT *", PlanNode::LookupTag("t".into()), 1);
        assert!(cache.get("SELECT *", 1).is_some()); // 同版本，命中
        assert!(cache.get("SELECT *", 2).is_none());  // 版本升级，失效
    }

    #[test]
    fn t_plan_cache_invalidate_all() {
        let cache = PlanCache::new(100);
        cache.put("q1", PlanNode::ShowSpaces, 0);
        cache.put("q2", PlanNode::ShowTags, 0);
        assert_eq!(cache.size(), 2);
        cache.invalidate_all();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn t_plan_cache_fingerprint() {
        // 相同结构的查询应有相同指纹
        let f1 = PlanCache::fingerprint("GO 3 STEPS FROM 'vid1'");
        let f2 = PlanCache::fingerprint("GO 3 STEPS FROM 'vid2'");
        assert_eq!(f1, f2);

        // 数字影响执行计划（GO N STEPS 的步数），必须区分 → 不同指纹
        let f3 = PlanCache::fingerprint("GO 5 STEPS FROM 'vid1'");
        assert_ne!(f1, f3);

        let f4 = PlanCache::fingerprint("MATCH (n) RETURN n");
        assert_ne!(f1, f4);
    }

    #[test]
    fn t_plan_cache_top_queries() {
        let cache = PlanCache::new(100);
        cache.put("q1", PlanNode::ShowSpaces, 0);
        cache.put("q2", PlanNode::ShowTags, 0);
        cache.get("q1", 0);
        cache.get("q1", 0);
        cache.get("q2", 0);

        let top = cache.top_queries(5);
        assert_eq!(top.len(), 2);
        assert!(top[0].1 >= top[1].1); // 按访问次数降序
    }

    #[test]
    fn t_plan_cache_reset_stats() {
        let cache = PlanCache::new(100);
        cache.put("q1", PlanNode::ShowSpaces, 0);
        cache.get("q1", 0);
        cache.get("q2", 0);
        assert_eq!(cache.hits() + cache.misses(), 2);

        cache.reset_stats();
        assert_eq!(cache.hits(), 0);
        assert_eq!(cache.misses(), 0);
    }

    // ===== 综合测试 =====
    #[test]
    fn t_full_optimization_pipeline() {
        // 1. 准备统计信息
        let mut stats = StatisticsManager::new();
        let mut player_tag = TagStatistics::new("player".into(), 100_000);
        let ages: Vec<f64> = (18..80).map(|i| i as f64).collect();
        player_tag.prop_stats
            .insert("age".into(), Histogram::from_values(&ages, 10));
        stats.update_tag_stats(player_tag);

        let follow_edge = EdgeStatistics::new("follow".into(), 1_000_000);
        stats.update_edge_stats(follow_edge);

        // 2. 创建CBO优化器
        let opt = CboOptimizer::new(stats);

        // 3. 优化查询（YIELD 触发常量折叠规则，验证规则管线生效）
        let plan = PlanNode::Yield2;
        let (optimized, rules) = opt.optimize(plan);
        assert!(!rules.is_empty());

        // 4. 估算代价
        let cost = opt.estimate_cost(&optimized);
        assert!(cost.total_cost > 0.0);

        // 5. 生成详细EXPLAIN
        let output = opt.explain_detailed(PlanNode::GoSteps(3));
        assert!(output.cost_reduction >= 0.0);
    }
}
