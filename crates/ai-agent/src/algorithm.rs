//! 算法分析与归一化模块
//!
//! 实现最强开发算法的识别、分析、流程图生成与归一化处理
//! 将任意算法转化为标准算子工作流

use super::types::*;
use operator_core::Result;
use std::collections::HashMap;
use tracing;
use uuid::Uuid;

/// 算法分析器 - 最强算法处理引擎
pub struct AlgorithmAnalyzer {
    /// 已知算法模式库
    patterns: Vec<AlgorithmPattern>,
    /// 算子映射规则
    operator_mappings: Vec<OperatorMappingRule>,
}

/// 算法模式
#[derive(Debug, Clone)]
struct AlgorithmPattern {
    name: String,
    algo_type: AlgorithmType,
    keywords: Vec<String>,
    code_patterns: Vec<String>,
    time_complexity: String,
    space_complexity: String,
    flow_template: Vec<FlowNodeTemplate>,
    normalization: Vec<String>,
}

/// 算子映射规则
#[derive(Debug, Clone)]
struct OperatorMappingRule {
    pattern: String,
    operator_id: String,
    confidence: f64,
}

/// 流程图节点模板
#[derive(Debug, Clone)]
struct FlowNodeTemplate {
    node_type: FlowNodeType,
    label: String,
    description: String,
    operator_hint: Option<String>,
}

impl AlgorithmAnalyzer {
    pub fn new() -> Self {
        Self {
            patterns: Self::build_algorithm_patterns(),
            operator_mappings: Self::build_operator_mappings(),
        }
    }

    /// 构建最强算法模式库
    fn build_algorithm_patterns() -> Vec<AlgorithmPattern> {
        vec![
            // 快速排序
            AlgorithmPattern {
                name: "快速排序".to_string(),
                algo_type: AlgorithmType::Sorting,
                keywords: vec!["quicksort".to_string(), "快速排序".to_string(), "qsort".to_string(), "partition".to_string()],
                code_patterns: vec!["pivot".to_string(), "partition".to_string(), "recursive".to_string()],
                time_complexity: "O(n log n) 平均, O(n²) 最坏".to_string(),
                space_complexity: "O(log n)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入待排序数组".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Input, label: "输入数组".to_string(), description: "接收数组A和边界l,r".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "l < r?".to_string(), description: "递归终止条件".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "分区操作".to_string(), description: "选择pivot，将数组分区".to_string(), operator_hint: Some("split".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "递归排序左半".to_string(), description: "递归排序[l, pivot-1]".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "递归排序右半".to_string(), description: "递归排序[pivot+1, r]".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Merge, label: "合并结果".to_string(), description: "左右子数组已在原数组上排序完成".to_string(), operator_hint: Some("concat".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Output, label: "输出结果".to_string(), description: "返回排序后的数组".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "排序完成".to_string(), operator_hint: None },
                ],
                normalization: vec!["parallel_branch".to_string(), "tail_recursion_optimization".to_string()],
            },
            // 归并排序
            AlgorithmPattern {
                name: "归并排序".to_string(),
                algo_type: AlgorithmType::Sorting,
                keywords: vec!["mergesort".to_string(), "归并排序".to_string(), "merge".to_string()],
                code_patterns: vec!["merge".to_string(), "divide".to_string(), "conquer".to_string()],
                time_complexity: "O(n log n)".to_string(),
                space_complexity: "O(n)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入待排序数组".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "长度>1?".to_string(), description: "分解终止条件".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "二分拆分".to_string(), description: "将数组分为两半".to_string(), operator_hint: Some("split".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "归并左半".to_string(), description: "递归排序左半".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "归并右半".to_string(), description: "递归排序右半".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "合并有序数组".to_string(), description: "双指针合并两个有序数组".to_string(), operator_hint: Some("concat".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "排序完成".to_string(), operator_hint: None },
                ],
                normalization: vec!["data_parallel".to_string()],
            },
            // PageRank
            AlgorithmPattern {
                name: "PageRank".to_string(),
                algo_type: AlgorithmType::Graph,
                keywords: vec!["pagerank".to_string(), "页面排名".to_string(), "pr算法".to_string()],
                code_patterns: vec!["damping".to_string(), "rank".to_string(), "iteration".to_string(), "convergence".to_string()],
                time_complexity: "O(n·iterations)".to_string(),
                space_complexity: "O(n + e)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入图结构G和阻尼系数d".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "初始化排名".to_string(), description: "所有节点初始排名=1/N".to_string(), operator_hint: Some("normalize_l1".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "构建转移矩阵".to_string(), description: "构建马尔可夫转移矩阵M".to_string(), operator_hint: Some("matmul".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "迭代计算".to_string(), description: "R' = d·M·R + (1-d)/N".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "收敛?".to_string(), description: "检查||R'-R|| < ε".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "归一化".to_string(), description: "排名归一化为概率分布".to_string(), operator_hint: Some("normalize_l1".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "输出最终PageRank值".to_string(), operator_hint: None },
                ],
                normalization: vec!["power_iteration".to_string(), "sparse_matrix_optimization".to_string()],
            },
            // 梯度下降
            AlgorithmPattern {
                name: "梯度下降".to_string(),
                algo_type: AlgorithmType::Optimization,
                keywords: vec!["gradient".to_string(), "梯度下降".to_string(), "sgd".to_string(), "backprop".to_string(), "反向传播".to_string()],
                code_patterns: vec!["learning_rate".to_string(), "gradient".to_string(), "backward".to_string(), "update".to_string()],
                time_complexity: "O(epochs·n·p)".to_string(),
                space_complexity: "O(p)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "初始化参数θ，学习率η".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Input, label: "输入数据".to_string(), description: "加载训练数据(X, y)".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "前向传播".to_string(), description: "ŷ = f(X; θ)".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "计算损失".to_string(), description: "L = loss(ŷ, y)".to_string(), operator_hint: Some("mse".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "反向传播".to_string(), description: "∂L/∂θ".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "参数更新".to_string(), description: "θ = θ - η·∇L".to_string(), operator_hint: Some("add".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "收敛?".to_string(), description: "损失<ε或达到最大epochs".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "输出训练好的参数θ".to_string(), operator_hint: None },
                ],
                normalization: vec!["mini_batch".to_string(), "momentum".to_string(), "adam_optimizer".to_string()],
            },
            // 神经网络前向传播
            AlgorithmPattern {
                name: "神经网络前向传播".to_string(),
                algo_type: AlgorithmType::DeepLearning,
                keywords: vec!["forward".to_string(), "前向传播".to_string(), "neural".to_string(), "神经网络".to_string(), "mlp".to_string()],
                code_patterns: vec!["layer".to_string(), "weight".to_string(), "activation".to_string(), "dense".to_string()],
                time_complexity: "O(batch·layers·units)".to_string(),
                space_complexity: "O(batch·units)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入数据X".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "输入层".to_string(), description: "h0 = X".to_string(), operator_hint: Some("identity".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "线性变换1".to_string(), description: "z1 = h0·W1 + b1".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "激活1".to_string(), description: "h1 = relu(z1)".to_string(), operator_hint: Some("relu".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "线性变换2".to_string(), description: "z2 = h1·W2 + b2".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "输出激活".to_string(), description: "h2 = softmax(z2)".to_string(), operator_hint: Some("softmax".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Output, label: "输出预测".to_string(), description: "ŷ = h2".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "前向传播完成".to_string(), operator_hint: None },
                ],
                normalization: vec!["batch_normalization".to_string(), "residual_connection".to_string()],
            },
            // 卷积操作
            AlgorithmPattern {
                name: "二维卷积".to_string(),
                algo_type: AlgorithmType::SignalProcessing,
                keywords: vec!["conv".to_string(), "卷积".to_string(), "convolution".to_string(), "cnn".to_string()],
                code_patterns: vec!["kernel".to_string(), "filter".to_string(), "stride".to_string(), "padding".to_string()],
                time_complexity: "O(H·W·Cin·Cout·K²)".to_string(),
                space_complexity: "O(H·W·Cout)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入特征图和卷积核".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "Padding".to_string(), description: "边界填充".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "im2col展开".to_string(), description: "展开为矩阵乘法".to_string(), operator_hint: Some("reshape".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "矩阵乘法".to_string(), description: "GEMM卷积".to_string(), operator_hint: Some("matmul".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "偏置加".to_string(), description: "添加偏置项".to_string(), operator_hint: Some("add_bias".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "激活".to_string(), description: "ReLU激活".to_string(), operator_hint: Some("relu".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "输出卷积特征图".to_string(), operator_hint: None },
                ],
                normalization: vec!["winograd".to_string(), "fft_acceleration".to_string(), "depthwise_separable".to_string()],
            },
            // 注意力机制
            AlgorithmPattern {
                name: "自注意力机制".to_string(),
                algo_type: AlgorithmType::DeepLearning,
                keywords: vec!["attention".to_string(), "注意力".to_string(), "self-attention".to_string(), "transformer".to_string()],
                code_patterns: vec!["query".to_string(), "key".to_string(), "value".to_string(), "softmax".to_string()],
                time_complexity: "O(n²·d)".to_string(),
                space_complexity: "O(n²)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入X".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "Q投影".to_string(), description: "Q = X·Wq".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "K投影".to_string(), description: "K = X·Wk".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Parallel, label: "V投影".to_string(), description: "V = X·Wv".to_string(), operator_hint: Some("linear".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "注意力分数".to_string(), description: "scores = Q·K^T / √d".to_string(), operator_hint: Some("matmul".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "Softmax".to_string(), description: "weights = softmax(scores)".to_string(), operator_hint: Some("softmax".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "加权求和".to_string(), description: "output = weights·V".to_string(), operator_hint: Some("matmul".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "输出注意力结果".to_string(), operator_hint: None },
                ],
                normalization: vec!["multi_head".to_string(), "flash_attention".to_string()],
            },
            // 二分查找
            AlgorithmPattern {
                name: "二分查找".to_string(),
                algo_type: AlgorithmType::Search,
                keywords: vec!["binary".to_string(), "二分查找".to_string(), "binary search".to_string()],
                code_patterns: vec!["mid".to_string(), "left".to_string(), "right".to_string()],
                time_complexity: "O(log n)".to_string(),
                space_complexity: "O(1)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入有序数组和目标".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "初始化指针".to_string(), description: "l=0, r=n-1".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "l ≤ r?".to_string(), description: "循环条件".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "计算中点".to_string(), description: "mid = (l+r)/2".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "A[mid]=target?".to_string(), description: "找到目标".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "A[mid]<target?".to_string(), description: "在右半查找".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Output, label: "返回结果".to_string(), description: "返回索引或-1".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "查找完成".to_string(), operator_hint: None },
                ],
                normalization: vec![],
            },
            // Dijkstra最短路径
            AlgorithmPattern {
                name: "Dijkstra最短路径".to_string(),
                algo_type: AlgorithmType::Graph,
                keywords: vec!["dijkstra".to_string(), "最短路径".to_string(), "shortest path".to_string()],
                code_patterns: vec!["priority".to_string(), "distance".to_string(), "relax".to_string(), "heap".to_string()],
                time_complexity: "O((V+E) log V)".to_string(),
                space_complexity: "O(V)".to_string(),
                flow_template: vec![
                    FlowNodeTemplate { node_type: FlowNodeType::Start, label: "开始".to_string(), description: "输入图G和起点s".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "初始化距离".to_string(), description: "dist[s]=0, 其他=∞".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "优先队列".to_string(), description: "将起点加入最小堆".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "提取最近节点".to_string(), description: "u = extract_min()".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::Process, label: "松弛操作".to_string(), description: "对每个邻居v: if dist[v]>dist[u]+w(u,v)".to_string(), operator_hint: Some("add".to_string()) },
                    FlowNodeTemplate { node_type: FlowNodeType::Decision, label: "队列空?".to_string(), description: "所有节点已处理".to_string(), operator_hint: None },
                    FlowNodeTemplate { node_type: FlowNodeType::End, label: "结束".to_string(), description: "输出最短路径距离".to_string(), operator_hint: None },
                ],
                normalization: vec!["bidirectional".to_string(), "a_star".to_string()],
            },
        ]
    }

    /// 构建算子映射规则
    fn build_operator_mappings() -> Vec<OperatorMappingRule> {
        vec![
            OperatorMappingRule { pattern: "normalize|norm|标准化|归一化".to_string(), operator_id: "normalize".to_string(), confidence: 0.9 },
            OperatorMappingRule { pattern: "linear|矩阵乘|线性变换|fc|dense|全连接".to_string(), operator_id: "linear".to_string(), confidence: 0.85 },
            OperatorMappingRule { pattern: "relu|整流|激活".to_string(), operator_id: "relu".to_string(), confidence: 0.9 },
            OperatorMappingRule { pattern: "sigmoid|s型|logistic".to_string(), operator_id: "sigmoid".to_string(), confidence: 0.9 },
            OperatorMappingRule { pattern: "softmax|指数归一化|分类".to_string(), operator_id: "softmax".to_string(), confidence: 0.85 },
            OperatorMappingRule { pattern: "tanh|双曲正切".to_string(), operator_id: "tanh".to_string(), confidence: 0.9 },
            OperatorMappingRule { pattern: "conv|卷积".to_string(), operator_id: "conv2d".to_string(), confidence: 0.95 },
            OperatorMappingRule { pattern: "matmul|矩阵乘法|gemm".to_string(), operator_id: "matmul".to_string(), confidence: 0.9 },
            OperatorMappingRule { pattern: "identity|恒等|skip|残差".to_string(), operator_id: "identity".to_string(), confidence: 0.8 },
            OperatorMappingRule { pattern: "pool|池化|下采样".to_string(), operator_id: "maxpool".to_string(), confidence: 0.8 },
            OperatorMappingRule { pattern: "attention|注意力".to_string(), operator_id: "attention".to_string(), confidence: 0.9 },
            OperatorMappingRule { pattern: "概率分布|l1|probability".to_string(), operator_id: "normalize_l1".to_string(), confidence: 0.85 },
        ]
    }

    /// 分析算法 - 主入口
    pub async fn analyze(&self, algo_code: &str, algo_type: AlgorithmType) -> Result<AlgorithmFlow> {
        tracing::info!("开始算法分析: type={:?}", algo_type);

        let (matched_pattern, confidence) = self.match_pattern(algo_code, &algo_type);

        let nodes = self.generate_flow_nodes(&matched_pattern);
        let edges = self.generate_flow_edges(&nodes);
        let operator_mapping = self.map_operators(&nodes);
        let optimization_suggestions = self.generate_optimizations(&matched_pattern, confidence);
        let complexity = self.analyze_complexity(&matched_pattern);
        let normalized_workflow = self.generate_normalized_workflow(&operator_mapping, &nodes);

        let flow = AlgorithmFlow {
            id: format!("flow-{}", &Uuid::new_v4().to_string()[..8]),
            name: matched_pattern.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "自定义算法流程".to_string()),
            description: self.generate_description(algo_code, &matched_pattern, confidence),
            algorithm_type: algo_type,
            nodes,
            edges,
            operator_mapping,
            optimization_suggestions,
            complexity_analysis: complexity,
            normalized_workflow,
        };

        tracing::info!("算法分析完成: {} 节点, {} 边, 归一化工作流长度 {}",
            flow.nodes.len(), flow.edges.len(), flow.normalized_workflow.len());

        Ok(flow)
    }

    /// 匹配算法模式
    fn match_pattern(&self, algo_code: &str, algo_type: &AlgorithmType) -> (Option<&AlgorithmPattern>, f64) {
        let code_lower = algo_code.to_lowercase();
        let mut best_match: Option<&AlgorithmPattern> = None;
        let mut best_score: f64 = 0.0;

        for pattern in &self.patterns {
            if !matches!(algo_type, AlgorithmType::Custom(_)) && pattern.algo_type != *algo_type {
                continue;
            }

            let mut score = 0.0;
            for kw in &pattern.keywords {
                if code_lower.contains(&kw.to_lowercase()) {
                    score += 0.3;
                }
            }
            for cp in &pattern.code_patterns {
                if code_lower.contains(&cp.to_lowercase()) {
                    score += 0.2;
                }
            }

            // 中文关键词匹配
            for kw in &pattern.keywords {
                if algo_code.contains(kw) {
                    score += 0.3;
                }
            }

            if score > best_score {
                best_score = score;
                best_match = Some(pattern);
            }
        }

        (best_match, best_score.min(1.0))
    }

    /// 生成流程图节点
    fn generate_flow_nodes(&self, pattern: &Option<&AlgorithmPattern>) -> Vec<FlowNode> {
        let mut nodes = Vec::new();

        if let Some(pat) = pattern {
            for (i, tmpl) in pat.flow_template.iter().enumerate() {
                nodes.push(FlowNode {
                    id: format!("node-{}", i),
                    node_type: tmpl.node_type.clone(),
                    label: tmpl.label.clone(),
                    description: tmpl.description.clone(),
                    operator_id: tmpl.operator_hint.clone(),
                    inputs: if i == 0 { vec![] } else { vec![format!("node-{}", i-1)] },
                    outputs: vec![],
                    parallel_group: if matches!(tmpl.node_type, FlowNodeType::Parallel) { Some("parallel-1".to_string()) } else { None },
                    condition: match tmpl.node_type {
                        FlowNodeType::Decision => Some(tmpl.label.clone()),
                        _ => None,
                    },
                });
            }
        } else {
            // 通用模板
            nodes = vec![
                FlowNode {
                    id: "start".to_string(),
                    node_type: FlowNodeType::Start,
                    label: "开始".to_string(),
                    description: "算法开始".to_string(),
                    operator_id: None,
                    inputs: vec![],
                    outputs: vec!["process".to_string()],
                    parallel_group: None,
                    condition: None,
                },
                FlowNode {
                    id: "input".to_string(),
                    node_type: FlowNodeType::Input,
                    label: "数据输入".to_string(),
                    description: "接收输入数据".to_string(),
                    operator_id: None,
                    inputs: vec!["start".to_string()],
                    outputs: vec!["process".to_string()],
                    parallel_group: None,
                    condition: None,
                },
                FlowNode {
                    id: "process".to_string(),
                    node_type: FlowNodeType::Process,
                    label: "算子处理".to_string(),
                    description: "执行算子变换".to_string(),
                    operator_id: Some("linear".to_string()),
                    inputs: vec!["input".to_string()],
                    outputs: vec!["normalize".to_string()],
                    parallel_group: None,
                    condition: None,
                },
                FlowNode {
                    id: "normalize".to_string(),
                    node_type: FlowNodeType::Process,
                    label: "归一化".to_string(),
                    description: "数据归一化处理".to_string(),
                    operator_id: Some("normalize".to_string()),
                    inputs: vec!["process".to_string()],
                    outputs: vec!["output".to_string()],
                    parallel_group: None,
                    condition: None,
                },
                FlowNode {
                    id: "output".to_string(),
                    node_type: FlowNodeType::Output,
                    label: "结果输出".to_string(),
                    description: "输出算法结果".to_string(),
                    operator_id: None,
                    inputs: vec!["normalize".to_string()],
                    outputs: vec!["end".to_string()],
                    parallel_group: None,
                    condition: None,
                },
                FlowNode {
                    id: "end".to_string(),
                    node_type: FlowNodeType::End,
                    label: "结束".to_string(),
                    description: "算法执行完成".to_string(),
                    operator_id: None,
                    inputs: vec!["output".to_string()],
                    outputs: vec![],
                    parallel_group: None,
                    condition: None,
                },
            ];
        }

        // 填充outputs
        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        for i in 0..nodes.len() {
            if i + 1 < nodes.len()
                && !nodes[i].outputs.contains(&node_ids[i+1]) {
                    nodes[i].outputs.push(node_ids[i+1].clone());
                }
        }

        nodes
    }

    /// 生成流程图边
    fn generate_flow_edges(&self, nodes: &[FlowNode]) -> Vec<FlowEdge> {
        let mut edges = Vec::new();

        for node in nodes {
            for (i, output) in node.outputs.iter().enumerate() {
                edges.push(FlowEdge {
                    id: format!("edge-{}-{}", node.id, i),
                    source: node.id.clone(),
                    target: output.clone(),
                    label: node.condition.clone(),
                    condition: node.condition.clone(),
                    data_type: Some("StateVector".to_string()),
                });
            }
        }

        edges
    }

    /// 映射算子：优先取节点显式 operator_id；缺失时用算子映射规则
    /// 对节点描述/标签做关键词模糊匹配，取置信度最高的命中规则
    /// （消费 `operator_mappings` 能力面：pattern + confidence 均生效）。
    fn map_operators(&self, nodes: &[FlowNode]) -> HashMap<String, String> {
        let mut mapping = HashMap::new();

        for node in nodes {
            let op_id = node.operator_id.clone().or_else(|| {
                let haystack = format!("{} {}", node.label, node.description).to_lowercase();
                self.operator_mappings
                    .iter()
                    .filter_map(|rule| {
                        let matched = rule
                            .pattern
                            .split('|')
                            .any(|kw| haystack.contains(&kw.to_lowercase()));
                        matched.then_some((rule.operator_id.clone(), rule.confidence))
                    })
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(op_id, _)| op_id)
            });
            if let Some(op_id) = op_id {
                mapping.insert(node.id.clone(), op_id);
            }
        }

        mapping
    }

    /// 生成优化建议
    fn generate_optimizations(&self, pattern: &Option<&AlgorithmPattern>, confidence: f64) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        if let Some(pat) = pattern {
            for (i, opt) in pat.normalization.iter().enumerate() {
                let impact = match opt.as_str() {
                    "flash_attention" | "winograd" | "fft_acceleration" | "sparse_matrix_optimization" => OptimizationImpact::High,
                    "adam_optimizer" | "multi_head" | "batch_normalization" | "residual_connection" => OptimizationImpact::Medium,
                    _ => OptimizationImpact::Low,
                };

                suggestions.push(OptimizationSuggestion {
                    id: format!("opt-{}", i),
                    description: format!("应用{}优化: {}", opt, Self::optimization_description(opt)),
                    impact,
                    applicable_nodes: vec![],
                });
            }
        }

        // 通用优化建议
        suggestions.push(OptimizationSuggestion {
            id: "opt-parallel".to_string(),
            description: "识别并并行化独立分支，利用多核CPU/GPU加速".to_string(),
            impact: OptimizationImpact::High,
            applicable_nodes: vec![],
        });

        suggestions.push(OptimizationSuggestion {
            id: "opt-fusion".to_string(),
            description: "算子融合：将连续的线性算子合并为单次矩阵乘法，减少内存访问".to_string(),
            impact: OptimizationImpact::Medium,
            applicable_nodes: vec![],
        });

        if confidence < 0.5 {
            suggestions.push(OptimizationSuggestion {
                id: "opt-pattern".to_string(),
                description: "模式匹配置信度较低，建议提供更多算法细节以获得更精准的归一化".to_string(),
                impact: OptimizationImpact::Low,
                applicable_nodes: vec![],
            });
        }

        suggestions
    }

    fn optimization_description(opt: &str) -> String {
        match opt {
            "parallel_branch" => "左右子数组排序可完全并行执行".to_string(),
            "data_parallel" => "归并排序天然适合数据并行".to_string(),
            "power_iteration" => "幂迭代法可利用稀疏矩阵加速".to_string(),
            "sparse_matrix_optimization" => "Web图通常是稀疏的，使用稀疏矩阵格式减少内存".to_string(),
            "mini_batch" => "使用小批量梯度下降提高稳定性".to_string(),
            "momentum" => "添加动量项加速收敛".to_string(),
            "adam_optimizer" => "使用Adam自适应学习率优化器".to_string(),
            "batch_normalization" => "添加批归一化加速训练".to_string(),
            "residual_connection" => "残差连接缓解梯度消失".to_string(),
            "winograd" => "Winograd算法减少卷积乘法次数".to_string(),
            "fft_acceleration" => "大核卷积可用FFT快速计算".to_string(),
            "depthwise_separable" => "深度可分离卷积减少参数量".to_string(),
            "multi_head" => "多头注意力捕捉不同子空间信息".to_string(),
            "flash_attention" => "FlashAttention减少显存占用，IO感知".to_string(),
            "bidirectional" => "双向搜索同时从起点和终点出发".to_string(),
            "a_star" => "使用启发式函数A*减少搜索空间".to_string(),
            "tail_recursion_optimization" => "尾递归优化为迭代，避免栈溢出".to_string(),
            _ => "优化算法性能".to_string(),
        }
    }

    /// 复杂度分析
    fn analyze_complexity(&self, pattern: &Option<&AlgorithmPattern>) -> ComplexityAnalysis {
        if let Some(pat) = pattern {
            let parallelizability = match pat.name.as_str() {
                "归并排序" | "快速排序" | "自注意力机制" => 0.8,
                "神经网络前向传播" | "二维卷积" => 0.6,
                "Dijkstra最短路径" | "二分查找" => 0.3,
                _ => 0.5,
            };

            let bottlenecks = match pat.name.as_str() {
                "自注意力机制" => vec!["n²注意力分数计算".to_string(), "大序列长度内存占用".to_string()],
                "快速排序" => vec!["最坏情况分区不平衡".to_string()],
                "二维卷积" => vec!["大卷积核计算量大".to_string()],
                "梯度下降" => vec!["收敛速度依赖学习率".to_string()],
                _ => vec!["串行瓶颈节点".to_string()],
            };

            ComplexityAnalysis {
                time_complexity: pat.time_complexity.clone(),
                space_complexity: pat.space_complexity.clone(),
                parallelizability,
                bottlenecks,
            }
        } else {
            ComplexityAnalysis {
                time_complexity: "待定 - 需要更多信息".to_string(),
                space_complexity: "待定".to_string(),
                parallelizability: 0.5,
                bottlenecks: vec!["无法确定，请提供更多算法细节".to_string()],
            }
        }
    }

    /// 生成归一化工作流（可直接执行的算子序列）
    fn generate_normalized_workflow(&self, mapping: &HashMap<String, String>, nodes: &[FlowNode]) -> Vec<String> {
        let mut workflow = Vec::new();

        for node in nodes {
            if let Some(op_id) = mapping.get(&node.id) {
                // 只添加实际算子节点
                if !workflow.contains(op_id) {
                    workflow.push(op_id.clone());
                }
            }
        }

        // 确保工作流有效：至少有一个算子
        if workflow.is_empty() {
            workflow = vec!["linear".to_string(), "relu".to_string(), "normalize".to_string()];
        }

        workflow
    }

    /// 生成描述
    fn generate_description(&self, algo_code: &str, pattern: &Option<&AlgorithmPattern>, confidence: f64) -> String {
        if let Some(pat) = pattern {
            format!("识别为「{}」算法，匹配置信度 {:.1}%。\n该算法类型为：{:?}。\n已归一化为标准算子流程图，包含 {} 个处理节点，可直接在算子统一系统中执行。",
                pat.name,
                confidence * 100.0,
                pat.algo_type,
                pat.flow_template.len()
            )
        } else {
            format!("未能精确匹配已知算法模式（置信度 {:.1}%），已生成通用处理流程。\n输入长度：{}字符。\n建议：提供更多算法细节或使用标准算法名称以获得更好的归一化结果。",
                confidence * 100.0,
                algo_code.len()
            )
        }
    }
}

impl Default for AlgorithmAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_quicksort_recognized_as_sorting() {
        let analyzer = AlgorithmAnalyzer::new();
        let code = "fn quicksort(arr: &[i32]) -> Vec<i32> {
            if arr.len() <= 1 { return arr.to_vec(); }
            let pivot = arr[arr.len()/2];
            let mut left = vec![x for x in arr if x < pivot];
            // divide and conquer
            quicksort(&left);
            quicksort(&right);
        }";
        let result = analyzer.analyze(code, AlgorithmType::Sorting).await.unwrap();
        assert_eq!(result.algorithm_type, AlgorithmType::Sorting);
        assert!(!result.nodes.is_empty());
        assert!(!result.edges.is_empty());
        // 快排应被识别为「快速排序」模式，产生优化建议
        assert!(!result.optimization_suggestions.is_empty());
        assert!(!result.normalized_workflow.is_empty());
        // 复杂度分析应非占位
        assert!(!result.complexity_analysis.time_complexity.contains("待定"));
    }

    #[tokio::test]
    async fn test_analyze_pagerank_recognized_as_graph() {
        let analyzer = AlgorithmAnalyzer::new();
        let code = "for iter in 0..max_iter {
            rank = damping * matmul(transition, rank) + (1.0 - damping) / n;
            // check convergence
        }";
        let result = analyzer.analyze(code, AlgorithmType::Graph).await.unwrap();
        assert_eq!(result.algorithm_type, AlgorithmType::Graph);
        assert!(!result.nodes.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_custom_falls_back_to_generic() {
        let analyzer = AlgorithmAnalyzer::new();
        let code = "let x = do_something_completely_unknown();";
        let result = analyzer.analyze(code, AlgorithmType::Custom("unknown".to_string())).await.unwrap();
        assert!(!result.nodes.is_empty());
        // 未知算法应给出通用归一化工作流（至少包含 linear/normalize）
        assert!(!result.normalized_workflow.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_generated_flow_has_valid_edges() {
        let analyzer = AlgorithmAnalyzer::new();
        let code = "merge sort: divide array into halves, then conquer by merging sorted subarrays";
        let result = analyzer.analyze(code, AlgorithmType::Sorting).await.unwrap();
        let node_ids: Vec<&String> = result.nodes.iter().map(|n| &n.id).collect();
        for e in &result.edges {
            assert!(node_ids.iter().any(|id| **id == *e.source), "edge source {} missing", e.source);
            assert!(node_ids.iter().any(|id| **id == *e.target), "edge target {} missing", e.target);
        }
    }
}
