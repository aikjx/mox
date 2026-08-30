// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 图索引管理（Graph Index Manager）
//!
//! 支持千亿级数据的高性能图索引系统，提供多种索引类型和智能索引选择。
//!
//! ## 索引类型
//!
//! - **主键索引**：VID → 顶点属性（点查，O(1)）
//! - **类型索引**：标签 → 顶点列表 / 边类型 → 边列表
//! - **属性索引**：单属性/组合属性的 B+ 树索引（范围查询、排序）
//! - **全文索引**：基于倒排索引的文本搜索
//! - **向量索引**：基于向量相似度的最近邻搜索（ANN）
//!
//! ## 索引管理
//!
//! - 创建/删除/重建索引
//! - 索引状态跟踪（Building / Ready / Invalid）
//! - 索引使用统计
//! - 索引选择性分析
//!
//! ## 查询时索引选择
//!
//! - 基于代价的索引选择
//! - 多索引组合使用
//! - 索引覆盖查询（Covering Index）

use crate::error::{GraphError, GraphResult};
use crate::result_set::PropValue;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// 索引类型定义
// ---------------------------------------------------------------------------

/// 索引类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexType {
    /// 主键索引：VID -> 顶点
    PrimaryKey,
    /// 类型索引：标签 -> 顶点列表
    TagType,
    /// 类型索引：边类型 -> 边列表
    EdgeType,
    /// 属性索引：单属性/组合属性 B+树
    Property,
    /// 全文索引：倒排索引
    FullText,
    /// 向量索引：最近邻搜索
    Vector,
}

impl IndexType {
    /// 索引类型名称
    pub fn name(&self) -> &'static str {
        match self {
            IndexType::PrimaryKey => "PRIMARY_KEY",
            IndexType::TagType => "TAG_TYPE",
            IndexType::EdgeType => "EDGE_TYPE",
            IndexType::Property => "PROPERTY",
            IndexType::FullText => "FULLTEXT",
            IndexType::Vector => "VECTOR",
        }
    }

    /// 是否支持范围查询
    pub fn supports_range_scan(&self) -> bool {
        matches!(self, IndexType::PrimaryKey | IndexType::Property)
    }

    /// 是否支持排序
    pub fn supports_order_by(&self) -> bool {
        matches!(self, IndexType::PrimaryKey | IndexType::Property)
    }
}

/// 索引状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexStatus {
    /// 构建中
    Building,
    /// 就绪可用
    Ready,
    /// 无效（需要重建）
    Invalid,
    /// 正在删除
    Dropping,
}

impl IndexStatus {
    pub fn name(&self) -> &'static str {
        match self {
            IndexStatus::Building => "BUILDING",
            IndexStatus::Ready => "READY",
            IndexStatus::Invalid => "INVALID",
            IndexStatus::Dropping => "DROPPING",
        }
    }

    /// 是否可用于查询
    pub fn is_usable(&self) -> bool {
        matches!(self, IndexStatus::Ready)
    }
}

/// 索引列定义
#[derive(Debug, Clone, PartialEq)]
pub struct IndexColumn {
    /// 列名
    pub name: String,
    /// 是否升序（仅对属性索引有效）
    pub ascending: bool,
    /// 数据类型
    pub data_type: String,
}

/// 索引定义
#[derive(Debug, Clone)]
pub struct IndexDefinition {
    /// 索引名称
    pub name: String,
    /// 索引类型
    pub index_type: IndexType,
    /// 所属标签（顶点索引）
    pub tag_name: Option<String>,
    /// 所属边类型（边索引）
    pub edge_name: Option<String>,
    /// 索引列
    pub columns: Vec<IndexColumn>,
    /// 是否唯一索引
    pub is_unique: bool,
    /// 创建时间
    pub created_at: u64,
    /// 索引状态
    pub status: IndexStatus,
    /// 注释
    pub comment: String,
}

impl IndexDefinition {
    /// 创建顶点属性索引
    pub fn tag_property(
        name: String,
        tag_name: String,
        columns: Vec<IndexColumn>,
        is_unique: bool,
    ) -> Self {
        Self {
            name,
            index_type: IndexType::Property,
            tag_name: Some(tag_name),
            edge_name: None,
            columns,
            is_unique,
            created_at: now_secs(),
            status: IndexStatus::Building,
            comment: String::new(),
        }
    }

    /// 创建边属性索引
    pub fn edge_property(
        name: String,
        edge_name: String,
        columns: Vec<IndexColumn>,
        is_unique: bool,
    ) -> Self {
        Self {
            name,
            index_type: IndexType::Property,
            tag_name: None,
            edge_name: Some(edge_name),
            columns,
            is_unique,
            created_at: now_secs(),
            status: IndexStatus::Building,
            comment: String::new(),
        }
    }

    /// 创建全文索引
    pub fn fulltext(name: String, tag_name: String, columns: Vec<IndexColumn>) -> Self {
        Self {
            name,
            index_type: IndexType::FullText,
            tag_name: Some(tag_name),
            edge_name: None,
            columns,
            is_unique: false,
            created_at: now_secs(),
            status: IndexStatus::Building,
            comment: String::new(),
        }
    }

    /// 创建向量索引
    pub fn vector(
        name: String,
        tag_name: String,
        column: IndexColumn,
        dimension: usize,
    ) -> Self {
        let _ = dimension; // 维度信息在实际实现中存储
        Self {
            name,
            index_type: IndexType::Vector,
            tag_name: Some(tag_name),
            edge_name: None,
            columns: vec![column],
            is_unique: false,
            created_at: now_secs(),
            status: IndexStatus::Building,
            comment: String::new(),
        }
    }

    /// 索引是否是顶点索引
    pub fn is_vertex_index(&self) -> bool {
        self.tag_name.is_some()
    }

    /// 索引是否是边索引
    pub fn is_edge_index(&self) -> bool {
        self.edge_name.is_some()
    }

    /// 获取索引列名列表
    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }
}

// ---------------------------------------------------------------------------
// 索引使用统计
// ---------------------------------------------------------------------------

/// 索引使用统计
#[derive(Debug, Default)]
pub struct IndexUsageStats {
    /// 扫描次数
    pub scan_count: AtomicU64,
    /// 范围扫描次数
    pub range_scan_count: AtomicU64,
    /// 唯一查找次数
    pub point_lookup_count: AtomicU64,
    /// 总扫描行数
    pub total_rows_scanned: AtomicU64,
    /// 最后使用时间
    pub last_used: AtomicU64,
    /// 索引大小（字节）
    pub index_size_bytes: AtomicU64,
    /// 索引项数量
    pub entry_count: AtomicU64,
}

impl IndexUsageStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一次扫描
    pub fn record_scan(&self, rows: u64) {
        self.scan_count.fetch_add(1, Ordering::SeqCst);
        self.total_rows_scanned.fetch_add(rows, Ordering::SeqCst);
        self.last_used.store(now_secs(), Ordering::SeqCst);
    }

    /// 记录一次范围扫描
    pub fn record_range_scan(&self, rows: u64) {
        self.range_scan_count.fetch_add(1, Ordering::SeqCst);
        self.total_rows_scanned.fetch_add(rows, Ordering::SeqCst);
        self.last_used.store(now_secs(), Ordering::SeqCst);
    }

    /// 记录一次点查
    pub fn record_point_lookup(&self) {
        self.point_lookup_count.fetch_add(1, Ordering::SeqCst);
        self.last_used.store(now_secs(), Ordering::SeqCst);
    }

    /// 获取统计快照
    pub fn snapshot(&self) -> IndexUsageSnapshot {
        IndexUsageSnapshot {
            scan_count: self.scan_count.load(Ordering::SeqCst),
            range_scan_count: self.range_scan_count.load(Ordering::SeqCst),
            point_lookup_count: self.point_lookup_count.load(Ordering::SeqCst),
            total_rows_scanned: self.total_rows_scanned.load(Ordering::SeqCst),
            last_used: self.last_used.load(Ordering::SeqCst),
            index_size_bytes: self.index_size_bytes.load(Ordering::SeqCst),
            entry_count: self.entry_count.load(Ordering::SeqCst),
        }
    }

    /// 重置统计
    pub fn reset(&self) {
        self.scan_count.store(0, Ordering::SeqCst);
        self.range_scan_count.store(0, Ordering::SeqCst);
        self.point_lookup_count.store(0, Ordering::SeqCst);
        self.total_rows_scanned.store(0, Ordering::SeqCst);
        self.last_used.store(0, Ordering::SeqCst);
    }
}

/// 索引使用统计快照
#[derive(Debug, Clone, Copy)]
pub struct IndexUsageSnapshot {
    pub scan_count: u64,
    pub range_scan_count: u64,
    pub point_lookup_count: u64,
    pub total_rows_scanned: u64,
    pub last_used: u64,
    pub index_size_bytes: u64,
    pub entry_count: u64,
}

// ---------------------------------------------------------------------------
// 索引选择性分析
// ---------------------------------------------------------------------------

/// 索引选择性分析结果
#[derive(Debug, Clone)]
pub struct IndexSelectivity {
    /// 索引名称
    pub index_name: String,
    /// 索引类型
    pub index_type: IndexType,
    /// 总行数
    pub total_rows: u64,
    /// 不同值数量（NDV）
    pub distinct_values: u64,
    /// 选择性（NDV / 总行数），越高越好
    pub selectivity: f64,
    /// 平均每个键对应的行数
    pub avg_rows_per_key: f64,
    /// 是否适合作为连接键
    pub good_for_join: bool,
    /// 是否适合作为过滤条件
    pub good_for_filter: bool,
}

impl IndexSelectivity {
    /// 计算选择性评级
    pub fn grade(&self) -> SelectivityGrade {
        if self.selectivity >= 0.9 {
            SelectivityGrade::Excellent
        } else if self.selectivity >= 0.5 {
            SelectivityGrade::Good
        } else if self.selectivity >= 0.1 {
            SelectivityGrade::Fair
        } else if self.selectivity >= 0.01 {
            SelectivityGrade::Poor
        } else {
            SelectivityGrade::VeryPoor
        }
    }
}

/// 选择性等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SelectivityGrade {
    VeryPoor,
    Poor,
    Fair,
    Good,
    Excellent,
}

impl SelectivityGrade {
    pub fn description(&self) -> &'static str {
        match self {
            SelectivityGrade::Excellent => "极佳：几乎唯一，适合点查",
            SelectivityGrade::Good => "良好：高选择性，适合过滤",
            SelectivityGrade::Fair => "一般：中等选择性",
            SelectivityGrade::Poor => "较差：低选择性，索引效果有限",
            SelectivityGrade::VeryPoor => "极差：几乎无选择性，不建议使用索引",
        }
    }
}

// ---------------------------------------------------------------------------
// B+ 树索引（简化实现）
// ---------------------------------------------------------------------------

/// B+ 树节点
#[derive(Debug, Clone)]
enum BPlusTreeNode<K: Ord + Clone, V: Clone> {
    Internal {
        keys: Vec<K>,
        children: Vec<Arc<BPlusTreeNode<K, V>>>,
    },
    Leaf {
        keys: Vec<K>,
        values: Vec<V>,
        next: Option<Arc<BPlusTreeNode<K, V>>>,
    },
}

/// B+ 树索引（简化实现，用于演示）
pub struct BPlusTreeIndex<K: Ord + Clone, V: Clone> {
    root: Option<Arc<BPlusTreeNode<K, V>>>,
    /// 阶数（每个节点最多 key 数）
    order: usize,
    /// 元素数量
    size: usize,
}

impl<K: Ord + Clone, V: Clone> BPlusTreeIndex<K, V> {
    /// 创建新的 B+ 树索引
    pub fn new(order: usize) -> Self {
        Self {
            root: None,
            order: order.max(3),
            size: 0,
        }
    }

    /// 索引大小
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// 插入键值对
    pub fn insert(&mut self, key: K, value: V) {
        // 简化实现：使用线性列表模拟
        self.size += 1;
        let _ = (key, value);
        // 完整实现需要节点分裂等操作
    }

    /// 点查询
    pub fn get(&self, _key: &K) -> Option<&V> {
        // 简化实现
        None
    }

    /// 范围查询
    pub fn range_query(&self, _low: &K, _high: &K) -> Vec<(&K, &V)> {
        // 简化实现
        Vec::new()
    }

    /// 前缀查询
    pub fn prefix_query(&self, _prefix: &K) -> Vec<(&K, &V)> {
        Vec::new()
    }
}

impl<K: Ord + Clone, V: Clone> Default for BPlusTreeIndex<K, V> {
    fn default() -> Self {
        Self::new(64)
    }
}

// ---------------------------------------------------------------------------
// 倒排索引（全文索引，简化实现）
// ---------------------------------------------------------------------------

/// 倒排列表项
#[derive(Debug, Clone)]
pub struct PostingItem {
    /// 文档ID（VID）
    pub doc_id: String,
    /// 词频
    pub term_frequency: u32,
    /// 位置列表
    pub positions: Vec<u32>,
}

/// 倒排索引（全文索引）
pub struct InvertedIndex {
    /// 词项 -> 倒排列表
    postings: HashMap<String, Vec<PostingItem>>,
    /// 文档总数
    doc_count: u64,
    /// 总词数
    total_terms: u64,
}

impl InvertedIndex {
    /// 创建新的倒排索引
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            doc_count: 0,
            total_terms: 0,
        }
    }

    /// 添加文档
    pub fn add_document(&mut self, doc_id: &str, text: &str) {
        let tokens = self.tokenize(text);
        let mut term_positions: HashMap<String, Vec<u32>> = HashMap::new();

        for (pos, token) in tokens.iter().enumerate() {
            term_positions
                .entry(token.clone())
                .or_default()
                .push(pos as u32);
        }

        for (term, positions) in term_positions {
            let posting = PostingItem {
                doc_id: doc_id.to_string(),
                term_frequency: positions.len() as u32,
                positions,
            };
            self.postings.entry(term).or_default().push(posting);
            self.total_terms += 1;
        }

        self.doc_count += 1;
    }

    /// 简单分词（按空格和标点）
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// 搜索单个词项
    pub fn search_term(&self, term: &str) -> Vec<&PostingItem> {
        let term = term.to_lowercase();
        self.postings
            .get(&term)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// 多词 AND 搜索
    pub fn search_and(&self, terms: &[String]) -> Vec<String> {
        if terms.is_empty() {
            return Vec::new();
        }

        let mut result: Option<HashSet<String>> = None;

        for term in terms {
            let docs: HashSet<String> = self
                .search_term(term)
                .iter()
                .map(|p| p.doc_id.clone())
                .collect();

            result = match result {
                Some(r) => Some(r.intersection(&docs).cloned().collect()),
                None => Some(docs),
            };
        }

        result.unwrap_or_default().into_iter().collect()
    }

    /// 多词 OR 搜索
    pub fn search_or(&self, terms: &[String]) -> Vec<String> {
        let mut result = HashSet::new();
        for term in terms {
            for posting in self.search_term(term) {
                result.insert(posting.doc_id.clone());
            }
        }
        result.into_iter().collect()
    }

    /// 文档总数
    pub fn doc_count(&self) -> u64 {
        self.doc_count
    }

    /// 词项数
    pub fn term_count(&self) -> usize {
        self.postings.len()
    }

    /// 计算 IDF（逆文档频率）
    pub fn idf(&self, term: &str) -> f64 {
        let doc_freq = self
            .postings
            .get(&term.to_lowercase())
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        if doc_freq == 0 {
            return 0.0;
        }
        ((self.doc_count as f64) / (doc_freq as f64)).ln()
    }
}

impl Default for InvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 向量索引（简化实现）
// ---------------------------------------------------------------------------

/// 向量索引类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorIndexType {
    /// HNSW：层次化导航小世界图
    HNSW,
    /// IVF：倒排文件
    IVF,
    /// Flat：暴力搜索（用于小数据集）
    Flat,
}

/// 向量索引配置
#[derive(Debug, Clone)]
pub struct VectorIndexConfig {
    /// 索引类型
    pub index_type: VectorIndexType,
    /// 向量维度
    pub dimension: usize,
    /// M 参数（HNSW）
    pub m: usize,
    /// ef_construction 参数（HNSW）
    pub ef_construction: usize,
    /// 搜索时的 ef 参数
    pub ef_search: usize,
    /// nlist 参数（IVF）
    pub nlist: usize,
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            index_type: VectorIndexType::HNSW,
            dimension: 128,
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            nlist: 100,
        }
    }
}

/// 向量搜索结果
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub vid: String,
    pub distance: f64,
}

/// 向量索引（简化实现）
pub struct VectorIndex {
    /// 配置
    pub config: VectorIndexConfig,
    /// 向量数据：vid -> 向量
    vectors: HashMap<String, Vec<f32>>,
}

impl VectorIndex {
    /// 创建新的向量索引
    pub fn new(config: VectorIndexConfig) -> Self {
        Self {
            config,
            vectors: HashMap::new(),
        }
    }

    /// 添加向量
    pub fn add_vector(&mut self, vid: &str, vector: Vec<f32>) -> GraphResult<()> {
        if vector.len() != self.config.dimension {
            return Err(GraphError::SemanticError(format!(
                "vector dimension mismatch: expected {}, got {}",
                self.config.dimension,
                vector.len()
            )));
        }
        self.vectors.insert(vid.to_string(), vector);
        Ok(())
    }

    /// 余弦相似度
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0f64;
        let mut norm_a = 0.0f64;
        let mut norm_b = 0.0f64;
        for i in 0..a.len() {
            dot += a[i] as f64 * b[i] as f64;
            norm_a += (a[i] as f64).powi(2);
            norm_b += (b[i] as f64).powi(2);
        }
        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom == 0.0 {
            0.0
        } else {
            dot / denom
        }
    }

    /// 欧氏距离
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return f64::MAX;
        }
        let mut sum = 0.0f64;
        for i in 0..a.len() {
            let diff = a[i] as f64 - b[i] as f64;
            sum += diff * diff;
        }
        sum.sqrt()
    }

    /// K近邻搜索
    pub fn knn_search(&self, query: &[f32], k: usize) -> Vec<VectorSearchResult> {
        let mut results: Vec<(String, f64)> = self
            .vectors
            .iter()
            .map(|(vid, vec)| {
                let dist = Self::euclidean_distance(query, vec);
                (vid.clone(), dist)
            })
            .collect();

        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);

        results
            .into_iter()
            .map(|(vid, distance)| VectorSearchResult { vid, distance })
            .collect()
    }

    /// 向量数量
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
}

// ---------------------------------------------------------------------------
// 索引管理器
// ---------------------------------------------------------------------------

/// 索引管理器
pub struct IndexManager {
    /// 索引定义映射：索引名 -> 索引定义
    indexes: Mutex<HashMap<String, IndexDefinition>>,
    /// 索引使用统计
    usage_stats: Mutex<HashMap<String, Arc<IndexUsageStats>>>,
    /// 标签 -> 索引名列表
    tag_indexes: Mutex<HashMap<String, Vec<String>>>,
    /// 边类型 -> 索引名列表
    edge_indexes: Mutex<HashMap<String, Vec<String>>>,
    /// 重建任务队列
    rebuild_queue: Mutex<VecDeque<String>>,
}

impl IndexManager {
    /// 创建新的索引管理器
    pub fn new() -> Self {
        Self {
            indexes: Mutex::new(HashMap::new()),
            usage_stats: Mutex::new(HashMap::new()),
            tag_indexes: Mutex::new(HashMap::new()),
            edge_indexes: Mutex::new(HashMap::new()),
            rebuild_queue: Mutex::new(VecDeque::new()),
        }
    }

    /// 创建索引
    pub fn create_index(&self, index_def: IndexDefinition) -> GraphResult<()> {
        let name = index_def.name.clone();

        // 检查是否已存在
        {
            let indexes = self.indexes.lock().map_err(|_| {
                GraphError::Internal("indexes lock poisoned".into())
            })?;
            if indexes.contains_key(&name) {
                return Err(GraphError::SemanticError(format!(
                    "index '{}' already exists",
                    name
                )));
            }
        }

        // 注册索引
        {
            let mut indexes = self.indexes.lock().map_err(|_| {
                GraphError::Internal("indexes lock poisoned".into())
            })?;
            indexes.insert(name.clone(), index_def.clone());
        }

        // 注册使用统计
        {
            let mut stats = self.usage_stats.lock().map_err(|_| {
                GraphError::Internal("usage_stats lock poisoned".into())
            })?;
            stats.insert(name.clone(), Arc::new(IndexUsageStats::new()));
        }

        // 注册到标签/边类型
        if let Some(ref tag) = index_def.tag_name {
            let mut tag_idx = self.tag_indexes.lock().map_err(|_| {
                GraphError::Internal("tag_indexes lock poisoned".into())
            })?;
            tag_idx.entry(tag.clone()).or_default().push(name.clone());
        }
        if let Some(ref edge) = index_def.edge_name {
            let mut edge_idx = self.edge_indexes.lock().map_err(|_| {
                GraphError::Internal("edge_indexes lock poisoned".into())
            })?;
            edge_idx.entry(edge.clone()).or_default().push(name.clone());
        }

        // 添加到重建队列（异步构建）
        {
            let mut queue = self.rebuild_queue.lock().map_err(|_| {
                GraphError::Internal("rebuild_queue lock poisoned".into())
            })?;
            queue.push_back(name);
        }

        Ok(())
    }

    /// 删除索引
    pub fn drop_index(&self, name: &str) -> GraphResult<bool> {
        let indexes = self.indexes.lock().map_err(|_| {
            GraphError::Internal("indexes lock poisoned".into())
        })?;

        let index_def = match indexes.get(name) {
            Some(def) => def.clone(),
            None => return Ok(false),
        };

        drop(indexes);

        // 更新状态为 Dropping
        {
            let mut indexes = self.indexes.lock().map_err(|_| {
                GraphError::Internal("indexes lock poisoned".into())
            })?;
            if let Some(def) = indexes.get_mut(name) {
                def.status = IndexStatus::Dropping;
            }
        }

        // 从标签/边类型中移除
        if let Some(ref tag) = index_def.tag_name {
            if let Ok(mut tag_idx) = self.tag_indexes.lock() {
                if let Some(list) = tag_idx.get_mut(tag) {
                    list.retain(|n| n != name);
                }
            }
        }
        if let Some(ref edge) = index_def.edge_name {
            if let Ok(mut edge_idx) = self.edge_indexes.lock() {
                if let Some(list) = edge_idx.get_mut(edge) {
                    list.retain(|n| n != name);
                }
            }
        }

        // 移除索引定义
        {
            let mut indexes = self.indexes.lock().map_err(|_| {
                GraphError::Internal("indexes lock poisoned".into())
            })?;
            indexes.remove(name);
        }

        // 移除统计
        {
            let mut stats = self.usage_stats.lock().map_err(|_| {
                GraphError::Internal("usage_stats lock poisoned".into())
            })?;
            stats.remove(name);
        }

        Ok(true)
    }

    /// 重建索引
    pub fn rebuild_index(&self, name: &str) -> GraphResult<()> {
        let indexes = self.indexes.lock().map_err(|_| {
            GraphError::Internal("indexes lock poisoned".into())
        })?;

        if !indexes.contains_key(name) {
            return Err(GraphError::SchemaNotFound(format!(
                "Index:{}",
                name
            )));
        }

        drop(indexes);

        // 设置为 Building 状态
        {
            let mut indexes = self.indexes.lock().map_err(|_| {
                GraphError::Internal("indexes lock poisoned".into())
            })?;
            if let Some(def) = indexes.get_mut(name) {
                def.status = IndexStatus::Building;
            }
        }

        // 添加到重建队列
        {
            let mut queue = self.rebuild_queue.lock().map_err(|_| {
                GraphError::Internal("rebuild_queue lock poisoned".into())
            })?;
            queue.push_back(name.to_string());
        }

        Ok(())
    }

    /// 获取索引定义
    pub fn get_index(&self, name: &str) -> Option<IndexDefinition> {
        self.indexes
            .lock()
            .ok()
            .and_then(|m| m.get(name).cloned())
    }

    /// 获取标签的所有索引
    pub fn get_tag_indexes(&self, tag_name: &str) -> Vec<IndexDefinition> {
        let tag_idx = match self.tag_indexes.lock() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let index_names = match tag_idx.get(tag_name) {
            Some(names) => names.clone(),
            None => return Vec::new(),
        };

        drop(tag_idx);

        let indexes = match self.indexes.lock() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        index_names
            .iter()
            .filter_map(|name| indexes.get(name).cloned())
            .collect()
    }

    /// 获取边类型的所有索引
    pub fn get_edge_indexes(&self, edge_name: &str) -> Vec<IndexDefinition> {
        let edge_idx = match self.edge_indexes.lock() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        let index_names = match edge_idx.get(edge_name) {
            Some(names) => names.clone(),
            None => return Vec::new(),
        };

        drop(edge_idx);

        let indexes = match self.indexes.lock() {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        index_names
            .iter()
            .filter_map(|name| indexes.get(name).cloned())
            .collect()
    }

    /// 列出所有索引
    pub fn list_indexes(&self) -> Vec<IndexDefinition> {
        self.indexes
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// 获取索引使用统计
    pub fn get_usage_stats(&self, name: &str) -> Option<IndexUsageSnapshot> {
        let stats = self.usage_stats.lock().ok()?;
        stats.get(name).map(|s| s.snapshot())
    }

    /// 标记索引为就绪状态
    pub fn mark_index_ready(&self, name: &str) -> GraphResult<()> {
        let mut indexes = self.indexes.lock().map_err(|_| {
            GraphError::Internal("indexes lock poisoned".into())
        })?;

        if let Some(def) = indexes.get_mut(name) {
            def.status = IndexStatus::Ready;
            Ok(())
        } else {
            Err(GraphError::SchemaNotFound(format!("Index:{}", name)))
        }
    }

    /// 标记索引为无效
    pub fn mark_index_invalid(&self, name: &str) -> GraphResult<()> {
        let mut indexes = self.indexes.lock().map_err(|_| {
            GraphError::Internal("indexes lock poisoned".into())
        })?;

        if let Some(def) = indexes.get_mut(name) {
            def.status = IndexStatus::Invalid;
            Ok(())
        } else {
            Err(GraphError::SchemaNotFound(format!("Index:{}", name)))
        }
    }

    /// 获取重建队列长度
    pub fn rebuild_queue_size(&self) -> usize {
        self.rebuild_queue
            .lock()
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// 处理下一个重建任务
    pub fn process_next_rebuild(&self) -> Option<String> {
        let mut queue = self.rebuild_queue.lock().ok()?;
        let name = queue.pop_front()?;

        // 标记为就绪（简化实现：直接设为 Ready）
        if let Ok(mut indexes) = self.indexes.lock() {
            if let Some(def) = indexes.get_mut(&name) {
                def.status = IndexStatus::Ready;
            }
        }

        Some(name)
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 索引选择器（Index Selector）
// ---------------------------------------------------------------------------

/// 索引选择器：基于代价选择最优索引
pub struct IndexSelector<'a> {
    index_manager: &'a IndexManager,
}

impl<'a> IndexSelector<'a> {
    pub fn new(index_manager: &'a IndexManager) -> Self {
        Self { index_manager }
    }

    /// 为标签属性等值查询选择最优索引
    pub fn select_for_tag_eq(
        &self,
        tag_name: &str,
        prop_name: &str,
        _value: &PropValue,
    ) -> Option<IndexSelection> {
        let indexes = self.index_manager.get_tag_indexes(tag_name);

        let mut candidates: Vec<IndexSelection> = Vec::new();

        for idx in indexes {
            if !idx.status.is_usable() {
                continue;
            }

            match idx.index_type {
                IndexType::PrimaryKey => {
                    // 主键索引点查代价最低
                    if prop_name == "vid" || prop_name == "id" {
                        candidates.push(IndexSelection {
                            index_name: idx.name.clone(),
                            index_type: idx.index_type,
                            estimated_cost: 0.001,
                            estimated_rows: 1,
                            is_covering: false,
                            reason: "primary key point lookup".into(),
                        });
                    }
                }
                IndexType::Property => {
                    // 检查索引列是否匹配
                    let cols = idx.column_names();
                    if cols.first().map(|c| c == prop_name).unwrap_or(false) {
                        let cost = if idx.is_unique {
                            0.001 // 唯一索引点查
                        } else {
                            0.01 // 非唯一索引扫描
                        };
                        candidates.push(IndexSelection {
                            index_name: idx.name.clone(),
                            index_type: idx.index_type,
                            estimated_cost: cost,
                            estimated_rows: if idx.is_unique { 1 } else { 100 },
                            is_covering: cols.len() == 1,
                            reason: format!("property index on {}", prop_name),
                        });
                    }
                }
                _ => {}
            }
        }

        // 按代价排序，返回最优
        candidates.sort_by(|a, b| {
            a.estimated_cost
                .partial_cmp(&b.estimated_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.into_iter().next()
    }

    /// 为范围查询选择最优索引
    pub fn select_for_tag_range(
        &self,
        tag_name: &str,
        prop_name: &str,
        _low: &PropValue,
        _high: &PropValue,
    ) -> Option<IndexSelection> {
        let indexes = self.index_manager.get_tag_indexes(tag_name);

        let mut candidates: Vec<IndexSelection> = Vec::new();

        for idx in indexes {
            if !idx.status.is_usable() {
                continue;
            }

            if idx.index_type == IndexType::Property {
                let cols = idx.column_names();
                if cols.first().map(|c| c == prop_name).unwrap_or(false) {
                    candidates.push(IndexSelection {
                        index_name: idx.name.clone(),
                        index_type: idx.index_type,
                        estimated_cost: 0.1,
                        estimated_rows: 1000,
                        is_covering: cols.len() == 1,
                        reason: format!("range scan on {} index", prop_name),
                    });
                }
            }
        }

        candidates.sort_by(|a, b| {
            a.estimated_cost
                .partial_cmp(&b.estimated_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.into_iter().next()
    }

    /// 为全文搜索选择索引
    pub fn select_for_fulltext(&self, tag_name: &str) -> Option<IndexSelection> {
        let indexes = self.index_manager.get_tag_indexes(tag_name);

        for idx in indexes {
            if !idx.status.is_usable() {
                continue;
            }
            if idx.index_type == IndexType::FullText {
                return Some(IndexSelection {
                    index_name: idx.name.clone(),
                    index_type: idx.index_type,
                    estimated_cost: 0.5,
                    estimated_rows: 100,
                    is_covering: false,
                    reason: "full-text index".into(),
                });
            }
        }

        None
    }

    /// 为向量搜索选择索引
    pub fn select_for_vector(&self, tag_name: &str) -> Option<IndexSelection> {
        let indexes = self.index_manager.get_tag_indexes(tag_name);

        for idx in indexes {
            if !idx.status.is_usable() {
                continue;
            }
            if idx.index_type == IndexType::Vector {
                return Some(IndexSelection {
                    index_name: idx.name.clone(),
                    index_type: idx.index_type,
                    estimated_cost: 1.0,
                    estimated_rows: 10,
                    is_covering: false,
                    reason: "vector ANN index".into(),
                });
            }
        }

        None
    }

    /// 检查是否为覆盖索引（索引包含所有需要的列，无需回表）
    pub fn is_covering_index(
        &self,
        index_name: &str,
        required_columns: &[String],
    ) -> bool {
        let idx = match self.index_manager.get_index(index_name) {
            Some(i) => i,
            None => return false,
        };

        let index_cols: HashSet<String> = idx.column_names().into_iter().collect();
        required_columns
            .iter()
            .all(|c| index_cols.contains(c))
    }

    /// 选择最佳排序索引
    pub fn select_for_order_by(
        &self,
        tag_name: &str,
        sort_columns: &[String],
    ) -> Option<IndexSelection> {
        let indexes = self.index_manager.get_tag_indexes(tag_name);

        for idx in indexes {
            if !idx.status.is_usable() {
                continue;
            }
            if !idx.index_type.supports_order_by() {
                continue;
            }

            let idx_cols = idx.column_names();
            // 检查索引前缀是否匹配排序键
            let mut match_count = 0;
            for (i, sort_col) in sort_columns.iter().enumerate() {
                if i < idx_cols.len() && &idx_cols[i] == sort_col {
                    match_count += 1;
                } else {
                    break;
                }
            }

            if match_count > 0 {
                return Some(IndexSelection {
                    index_name: idx.name.clone(),
                    index_type: idx.index_type,
                    estimated_cost: 0.05 * (sort_columns.len() - match_count) as f64 + 0.01,
                    estimated_rows: 1000,
                    is_covering: match_count == sort_columns.len(),
                    reason: format!(
                        "sort elimination using index ({} prefix matched)",
                        match_count
                    ),
                });
            }
        }

        None
    }
}

/// 索引选择结果
#[derive(Debug, Clone)]
pub struct IndexSelection {
    /// 选中的索引名
    pub index_name: String,
    /// 索引类型
    pub index_type: IndexType,
    /// 估算代价
    pub estimated_cost: f64,
    /// 估算返回行数
    pub estimated_rows: u64,
    /// 是否为覆盖索引
    pub is_covering: bool,
    /// 选择理由
    pub reason: String,
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ===== IndexType 测试 =====
    #[test]
    fn t_index_type_names() {
        assert_eq!(IndexType::PrimaryKey.name(), "PRIMARY_KEY");
        assert_eq!(IndexType::Property.name(), "PROPERTY");
        assert_eq!(IndexType::FullText.name(), "FULLTEXT");
        assert_eq!(IndexType::Vector.name(), "VECTOR");
    }

    #[test]
    fn t_index_type_capabilities() {
        assert!(IndexType::PrimaryKey.supports_range_scan());
        assert!(IndexType::Property.supports_range_scan());
        assert!(!IndexType::FullText.supports_range_scan());
        assert!(!IndexType::TagType.supports_range_scan());
    }

    // ===== IndexStatus 测试 =====
    #[test]
    fn t_index_status_usable() {
        assert!(IndexStatus::Ready.is_usable());
        assert!(!IndexStatus::Building.is_usable());
        assert!(!IndexStatus::Invalid.is_usable());
        assert!(!IndexStatus::Dropping.is_usable());
    }

    #[test]
    fn t_index_status_names() {
        assert_eq!(IndexStatus::Building.name(), "BUILDING");
        assert_eq!(IndexStatus::Ready.name(), "READY");
        assert_eq!(IndexStatus::Invalid.name(), "INVALID");
    }

    // ===== IndexDefinition 测试 =====
    #[test]
    fn t_index_def_tag_property() {
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        let idx = IndexDefinition::tag_property("idx_name".into(), "player".into(), cols, false);
        assert_eq!(idx.name, "idx_name");
        assert_eq!(idx.index_type, IndexType::Property);
        assert_eq!(idx.tag_name, Some("player".into()));
        assert!(idx.is_vertex_index());
        assert!(!idx.is_edge_index());
        assert_eq!(idx.status, IndexStatus::Building);
    }

    #[test]
    fn t_index_def_edge_property() {
        let cols = vec![IndexColumn {
            name: "weight".into(),
            ascending: true,
            data_type: "double".into(),
        }];
        let idx = IndexDefinition::edge_property("idx_weight".into(), "follow".into(), cols, false);
        assert!(idx.is_edge_index());
        assert!(!idx.is_vertex_index());
    }

    #[test]
    fn t_index_def_fulltext() {
        let cols = vec![IndexColumn {
            name: "bio".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        let idx = IndexDefinition::fulltext("idx_bio_ft".into(), "player".into(), cols);
        assert_eq!(idx.index_type, IndexType::FullText);
    }

    #[test]
    fn t_index_def_vector() {
        let col = IndexColumn {
            name: "embedding".into(),
            ascending: true,
            data_type: "vector(128)".into(),
        };
        let idx = IndexDefinition::vector("idx_embedding".into(), "player".into(), col, 128);
        assert_eq!(idx.index_type, IndexType::Vector);
        assert_eq!(idx.columns.len(), 1);
    }

    #[test]
    fn t_index_def_column_names() {
        let cols = vec![
            IndexColumn { name: "a".into(), ascending: true, data_type: "int".into() },
            IndexColumn { name: "b".into(), ascending: true, data_type: "string".into() },
        ];
        let idx = IndexDefinition::tag_property("idx".into(), "t".into(), cols, false);
        assert_eq!(idx.column_names(), vec!["a", "b"]);
    }

    // ===== IndexUsageStats 测试 =====
    #[test]
    fn t_usage_stats_record_scan() {
        let stats = IndexUsageStats::new();
        stats.record_scan(100);
        stats.record_scan(200);
        let snap = stats.snapshot();
        assert_eq!(snap.scan_count, 2);
        assert_eq!(snap.total_rows_scanned, 300);
    }

    #[test]
    fn t_usage_stats_record_point_lookup() {
        let stats = IndexUsageStats::new();
        stats.record_point_lookup();
        stats.record_point_lookup();
        let snap = stats.snapshot();
        assert_eq!(snap.point_lookup_count, 2);
    }

    #[test]
    fn t_usage_stats_record_range_scan() {
        let stats = IndexUsageStats::new();
        stats.record_range_scan(50);
        let snap = stats.snapshot();
        assert_eq!(snap.range_scan_count, 1);
        assert_eq!(snap.total_rows_scanned, 50);
    }

    #[test]
    fn t_usage_stats_reset() {
        let stats = IndexUsageStats::new();
        stats.record_scan(100);
        stats.record_point_lookup();
        stats.reset();
        let snap = stats.snapshot();
        assert_eq!(snap.scan_count, 0);
        assert_eq!(snap.point_lookup_count, 0);
    }

    // ===== IndexSelectivity 测试 =====
    #[test]
    fn t_selectivity_grade_excellent() {
        let sel = IndexSelectivity {
            index_name: "idx".into(),
            index_type: IndexType::Property,
            total_rows: 1000,
            distinct_values: 950,
            selectivity: 0.95,
            avg_rows_per_key: 1.05,
            good_for_join: true,
            good_for_filter: true,
        };
        assert_eq!(sel.grade(), SelectivityGrade::Excellent);
    }

    #[test]
    fn t_selectivity_grade_poor() {
        let sel = IndexSelectivity {
            index_name: "idx".into(),
            index_type: IndexType::Property,
            total_rows: 1000,
            distinct_values: 5,
            selectivity: 0.005,
            avg_rows_per_key: 200.0,
            good_for_join: false,
            good_for_filter: false,
        };
        assert_eq!(sel.grade(), SelectivityGrade::VeryPoor);
    }

    #[test]
    fn t_selectivity_grade_descriptions() {
        assert!(!SelectivityGrade::Excellent.description().is_empty());
        assert!(!SelectivityGrade::Good.description().is_empty());
        assert!(!SelectivityGrade::Fair.description().is_empty());
        assert!(!SelectivityGrade::Poor.description().is_empty());
        assert!(!SelectivityGrade::VeryPoor.description().is_empty());
    }

    // ===== B+ 树测试 =====
    #[test]
    fn t_bplus_tree_new() {
        let tree: BPlusTreeIndex<i32, String> = BPlusTreeIndex::new(64);
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn t_bplus_tree_insert() {
        let mut tree: BPlusTreeIndex<i32, String> = BPlusTreeIndex::new(64);
        tree.insert(1, "one".into());
        tree.insert(2, "two".into());
        assert_eq!(tree.len(), 2);
    }

    // ===== 倒排索引测试 =====
    #[test]
    fn t_inverted_index_add_and_search() {
        let mut idx = InvertedIndex::new();
        idx.add_document("doc1", "hello world hello");
        idx.add_document("doc2", "world peace");

        assert_eq!(idx.doc_count(), 2);
        assert!(idx.term_count() > 0);

        let results = idx.search_term("hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, "doc1");
        assert_eq!(results[0].term_frequency, 2);
    }

    #[test]
    fn t_inverted_index_and_search() {
        let mut idx = InvertedIndex::new();
        idx.add_document("doc1", "hello world");
        idx.add_document("doc2", "hello peace");
        idx.add_document("doc3", "foo bar");

        let results = idx.search_and(&["hello".into(), "world".into()]);
        assert_eq!(results.len(), 1);
        assert!(results.contains(&"doc1".to_string()));
    }

    #[test]
    fn t_inverted_index_or_search() {
        let mut idx = InvertedIndex::new();
        idx.add_document("doc1", "hello");
        idx.add_document("doc2", "world");

        let results = idx.search_or(&["hello".into(), "world".into()]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn t_inverted_index_idf() {
        let mut idx = InvertedIndex::new();
        idx.add_document("doc1", "hello");
        idx.add_document("doc2", "hello");
        idx.add_document("doc3", "world");

        let idf_hello = idx.idf("hello");
        let idf_world = idx.idf("world");
        // world 出现在更少的文档中，IDF 应该更高
        assert!(idf_world > idf_hello);
    }

    // ===== 向量索引测试 =====
    #[test]
    fn t_vector_index_add_and_search() {
        let config = VectorIndexConfig {
            dimension: 3,
            ..Default::default()
        };
        let mut idx = VectorIndex::new(config);

        idx.add_vector("v1", vec![1.0, 0.0, 0.0]).unwrap();
        idx.add_vector("v2", vec![0.0, 1.0, 0.0]).unwrap();
        idx.add_vector("v3", vec![1.0, 1.0, 0.0]).unwrap();

        assert_eq!(idx.len(), 3);

        let results = idx.knn_search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].vid, "v1"); // 最近的应该是自己
        assert!(results[0].distance < results[1].distance);
    }

    #[test]
    fn t_vector_index_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorIndex::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(VectorIndex::cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[test]
    fn t_vector_index_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((VectorIndex::euclidean_distance(&a, &b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn t_vector_index_dimension_mismatch() {
        let config = VectorIndexConfig {
            dimension: 3,
            ..Default::default()
        };
        let mut idx = VectorIndex::new(config);
        let result = idx.add_vector("v1", vec![1.0, 2.0]);
        assert!(result.is_err());
    }

    // ===== IndexManager 测试 =====
    #[test]
    fn t_index_manager_create_and_get() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        let idx = IndexDefinition::tag_property("idx_name".into(), "player".into(), cols, false);
        mgr.create_index(idx).unwrap();

        let result = mgr.get_index("idx_name");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "idx_name");
    }

    #[test]
    fn t_index_manager_create_duplicate() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        let idx1 = IndexDefinition::tag_property("idx".into(), "t".into(), cols.clone(), false);
        mgr.create_index(idx1).unwrap();

        let idx2 = IndexDefinition::tag_property("idx".into(), "t".into(), cols, false);
        let result = mgr.create_index(idx2);
        assert!(result.is_err());
    }

    #[test]
    fn t_index_manager_drop() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        let idx = IndexDefinition::tag_property("idx_name".into(), "player".into(), cols, false);
        mgr.create_index(idx).unwrap();

        let dropped = mgr.drop_index("idx_name").unwrap();
        assert!(dropped);
        assert!(mgr.get_index("idx_name").is_none());
    }

    #[test]
    fn t_index_manager_drop_nonexistent() {
        let mgr = IndexManager::new();
        let result = mgr.drop_index("nonexistent").unwrap();
        assert!(!result);
    }

    #[test]
    fn t_index_manager_get_tag_indexes() {
        let mgr = IndexManager::new();
        let cols1 = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        let cols2 = vec![IndexColumn {
            name: "age".into(),
            ascending: true,
            data_type: "int".into(),
        }];

        mgr.create_index(IndexDefinition::tag_property(
            "idx_name".into(),
            "player".into(),
            cols1,
            false,
        ))
        .unwrap();
        mgr.create_index(IndexDefinition::tag_property(
            "idx_age".into(),
            "player".into(),
            cols2,
            false,
        ))
        .unwrap();

        let indexes = mgr.get_tag_indexes("player");
        assert_eq!(indexes.len(), 2);
    }

    #[test]
    fn t_index_manager_get_edge_indexes() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "weight".into(),
            ascending: true,
            data_type: "double".into(),
        }];
        mgr.create_index(IndexDefinition::edge_property(
            "idx_weight".into(),
            "follow".into(),
            cols,
            false,
        ))
        .unwrap();

        let indexes = mgr.get_edge_indexes("follow");
        assert_eq!(indexes.len(), 1);
    }

    #[test]
    fn t_index_manager_rebuild() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx".into(),
            "t".into(),
            cols,
            false,
        ))
        .unwrap();

        assert!(mgr.rebuild_index("idx").is_ok());
        assert_eq!(mgr.rebuild_queue_size(), 1);

        let processed = mgr.process_next_rebuild();
        assert_eq!(processed, Some("idx".into()));
        assert_eq!(mgr.rebuild_queue_size(), 0);

        // 处理后应该变为 Ready 状态
        let idx = mgr.get_index("idx").unwrap();
        assert_eq!(idx.status, IndexStatus::Ready);
    }

    #[test]
    fn t_index_manager_mark_ready_invalid() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx".into(),
            "t".into(),
            cols,
            false,
        ))
        .unwrap();

        mgr.mark_index_ready("idx").unwrap();
        assert_eq!(mgr.get_index("idx").unwrap().status, IndexStatus::Ready);

        mgr.mark_index_invalid("idx").unwrap();
        assert_eq!(mgr.get_index("idx").unwrap().status, IndexStatus::Invalid);
    }

    #[test]
    fn t_index_manager_list_indexes() {
        let mgr = IndexManager::new();
        let cols1 = vec![IndexColumn {
            name: "a".into(),
            ascending: true,
            data_type: "int".into(),
        }];
        let cols2 = vec![IndexColumn {
            name: "b".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx1".into(),
            "t".into(),
            cols1,
            false,
        ))
        .unwrap();
        mgr.create_index(IndexDefinition::tag_property(
            "idx2".into(),
            "t".into(),
            cols2,
            false,
        ))
        .unwrap();

        assert_eq!(mgr.list_indexes().len(), 2);
    }

    #[test]
    fn t_index_manager_usage_stats() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx".into(),
            "t".into(),
            cols,
            false,
        ))
        .unwrap();

        let stats = mgr.get_usage_stats("idx");
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().scan_count, 0);
    }

    // ===== IndexSelector 测试 =====
    #[test]
    fn t_index_selector_eq() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx_name".into(),
            "player".into(),
            cols,
            false,
        ))
        .unwrap();
        mgr.mark_index_ready("idx_name").unwrap();

        let selector = IndexSelector::new(&mgr);
        let selection =
            selector.select_for_tag_eq("player", "name", &PropValue::Str("test".into()));
        assert!(selection.is_some());
        assert_eq!(selection.unwrap().index_name, "idx_name");
    }

    #[test]
    fn t_index_selector_no_usable_index() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx_name".into(),
            "player".into(),
            cols,
            false,
        ))
        .unwrap();
        // 索引仍在 Building 状态，不可用

        let selector = IndexSelector::new(&mgr);
        let selection =
            selector.select_for_tag_eq("player", "name", &PropValue::Str("test".into()));
        assert!(selection.is_none());
    }

    #[test]
    fn t_index_selector_fulltext() {
        let mgr = IndexManager::new();
        let cols = vec![IndexColumn {
            name: "bio".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::fulltext(
            "idx_bio_ft".into(),
            "player".into(),
            cols,
        ))
        .unwrap();
        mgr.mark_index_ready("idx_bio_ft").unwrap();

        let selector = IndexSelector::new(&mgr);
        let selection = selector.select_for_fulltext("player");
        assert!(selection.is_some());
        assert_eq!(selection.unwrap().index_type, IndexType::FullText);
    }

    #[test]
    fn t_index_selector_vector() {
        let mgr = IndexManager::new();
        let col = IndexColumn {
            name: "embedding".into(),
            ascending: true,
            data_type: "vector".into(),
        };
        mgr.create_index(IndexDefinition::vector(
            "idx_emb".into(),
            "player".into(),
            col,
            128,
        ))
        .unwrap();
        mgr.mark_index_ready("idx_emb").unwrap();

        let selector = IndexSelector::new(&mgr);
        let selection = selector.select_for_vector("player");
        assert!(selection.is_some());
        assert_eq!(selection.unwrap().index_type, IndexType::Vector);
    }

    #[test]
    fn t_index_selector_covering() {
        let mgr = IndexManager::new();
        let cols = vec![
            IndexColumn { name: "name".into(), ascending: true, data_type: "string".into() },
        ];
        mgr.create_index(IndexDefinition::tag_property(
            "idx_name".into(),
            "player".into(),
            cols,
            false,
        ))
        .unwrap();
        mgr.mark_index_ready("idx_name").unwrap();

        let selector = IndexSelector::new(&mgr);
        assert!(selector.is_covering_index("idx_name", &["name".into()]));
        assert!(!selector.is_covering_index("idx_name", &["name".into(), "age".into()]));
    }

    #[test]
    fn t_index_selector_order_by() {
        let mgr = IndexManager::new();
        let cols = vec![
            IndexColumn { name: "age".into(), ascending: true, data_type: "int".into() },
            IndexColumn { name: "name".into(), ascending: true, data_type: "string".into() },
        ];
        mgr.create_index(IndexDefinition::tag_property(
            "idx_age_name".into(),
            "player".into(),
            cols,
            false,
        ))
        .unwrap();
        mgr.mark_index_ready("idx_age_name").unwrap();

        let selector = IndexSelector::new(&mgr);
        let selection = selector.select_for_order_by("player", &["age".into(), "name".into()]);
        assert!(selection.is_some());
        assert!(selection.unwrap().is_covering);
    }

    // ===== 综合测试 =====
    #[test]
    fn t_full_index_lifecycle() {
        let mgr = IndexManager::new();

        // 1. 创建索引
        let cols = vec![IndexColumn {
            name: "name".into(),
            ascending: true,
            data_type: "string".into(),
        }];
        mgr.create_index(IndexDefinition::tag_property(
            "idx_name".into(),
            "player".into(),
            cols,
            false,
        ))
        .unwrap();
        assert_eq!(mgr.rebuild_queue_size(), 1);

        // 2. 处理构建
        let name = mgr.process_next_rebuild().unwrap();
        assert_eq!(name, "idx_name");
        assert_eq!(mgr.get_index("idx_name").unwrap().status, IndexStatus::Ready);

        // 3. 使用索引
        let selector = IndexSelector::new(&mgr);
        let selection =
            selector.select_for_tag_eq("player", "name", &PropValue::Str("test".into()));
        assert!(selection.is_some());

        // 4. 重建索引
        mgr.rebuild_index("idx_name").unwrap();
        assert_eq!(mgr.get_index("idx_name").unwrap().status, IndexStatus::Building);
        mgr.process_next_rebuild();
        assert_eq!(mgr.get_index("idx_name").unwrap().status, IndexStatus::Ready);

        // 5. 删除索引
        mgr.drop_index("idx_name").unwrap();
        assert!(mgr.get_index("idx_name").is_none());
        assert!(mgr.get_tag_indexes("player").is_empty());
    }
}
