// Copyright (c) 2026 璇玑 RelGraph · AI对话全维自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 意图识别器
//!
//! 基于关键词匹配 + 模式匹配的意图识别，支持：
//! - 关键词触发
//! - 正则模式匹配
//! - 多意图识别
//! - 置信度评分
//! - 实体提取

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::AiResult;
use crate::types::IntentType;

/// 意图模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPattern {
    /// 意图类型
    pub intent: IntentType,
    /// 关键词列表（任意一个匹配即触发）
    pub keywords: Vec<String>,
    /// 正则模式列表
    pub regex_patterns: Vec<String>,
    /// 必须包含的词（全部包含才匹配）
    pub required_words: Vec<String>,
    /// 排除词（包含则不匹配）
    pub excluded_words: Vec<String>,
    /// 基础置信度
    pub base_confidence: f64,
    /// 是否启用
    pub enabled: bool,
}

impl IntentPattern {
    pub fn new(intent: IntentType) -> Self {
        Self {
            intent,
            keywords: Vec::new(),
            regex_patterns: Vec::new(),
            required_words: Vec::new(),
            excluded_words: Vec::new(),
            base_confidence: 0.8,
            enabled: true,
        }
    }

    /// 添加关键词
    pub fn with_keyword(mut self, keyword: &str) -> Self {
        self.keywords.push(keyword.to_string());
        self
    }

    /// 添加必填词
    pub fn with_required(mut self, word: &str) -> Self {
        self.required_words.push(word.to_string());
        self
    }
}

/// 意图匹配结果
#[derive(Debug, Clone)]
pub struct IntentMatch {
    /// 匹配的意图
    pub intent: IntentType,
    /// 置信度 (0-1)
    pub confidence: f64,
    /// 匹配的关键词
    pub matched_keywords: Vec<String>,
    /// 提取的实体
    pub entities: HashMap<String, String>,
}

/// 意图识别器
pub struct IntentRecognizer {
    /// 意图模式：intent -> Vec<IntentPattern>
    patterns: RwLock<HashMap<IntentType, Vec<IntentPattern>>>,
    /// 总匹配次数
    total_matches: std::sync::atomic::AtomicU64,
}

impl IntentRecognizer {
    /// 创建意图识别器（内置默认模式）
    pub fn new() -> Self {
        let recognizer = Self {
            patterns: RwLock::new(HashMap::new()),
            total_matches: std::sync::atomic::AtomicU64::new(0),
        };
        recognizer.register_default_patterns();
        recognizer
    }

    /// 注册默认模式
    fn register_default_patterns(&self) {
        // 知识图谱查询
        let graph = IntentPattern::new(IntentType::GraphQuery)
            .with_keyword("图谱")
            .with_keyword("知识图谱")
            .with_keyword("查询节点")
            .with_keyword("节点")
            .with_keyword("关系")
            .with_keyword("路径")
            .with_keyword("图分析");
        self.register_pattern(graph).unwrap();

        // 知识库检索
        let kb = IntentPattern::new(IntentType::KnowledgeSearch)
            .with_keyword("搜索")
            .with_keyword("查找")
            .with_keyword("知识库")
            .with_keyword("文档")
            .with_keyword("全文检索")
            .with_keyword("检索");
        self.register_pattern(kb).unwrap();

        // 数据分析
        let analysis = IntentPattern::new(IntentType::DataAnalysis)
            .with_keyword("分析")
            .with_keyword("统计")
            .with_keyword("报表")
            .with_keyword("趋势")
            .with_keyword("数据");
        self.register_pattern(analysis).unwrap();

        // 算法执行
        let algo = IntentPattern::new(IntentType::AlgorithmRun)
            .with_keyword("算法")
            .with_keyword("运行")
            .with_keyword("执行算法")
            .with_keyword("PageRank")
            .with_keyword("社区发现");
        self.register_pattern(algo).unwrap();

        // 流程启动
        let workflow = IntentPattern::new(IntentType::WorkflowStart)
            .with_keyword("启动流程")
            .with_keyword("发起流程")
            .with_keyword("审批")
            .with_keyword("流程");
        self.register_pattern(workflow).unwrap();

        // 实体创建
        let entity = IntentPattern::new(IntentType::EntityCreate)
            .with_keyword("创建实体")
            .with_keyword("新增实体")
            .with_keyword("添加实体");
        self.register_pattern(entity).unwrap();

        // 文件操作
        let file = IntentPattern::new(IntentType::FileOperation)
            .with_keyword("文件")
            .with_keyword("上传")
            .with_keyword("下载")
            .with_keyword("云盘")
            .with_keyword("文件夹");
        self.register_pattern(file).unwrap();

        // 报表生成
        let report = IntentPattern::new(IntentType::ReportGenerate)
            .with_keyword("生成报表")
            .with_keyword("报告")
            .with_keyword("导出报表");
        self.register_pattern(report).unwrap();
    }

    /// 注册意图模式
    pub fn register_pattern(&self, pattern: IntentPattern) -> AiResult<()> {
        self.patterns
            .write()
            .entry(pattern.intent)
            .or_default()
            .push(pattern);
        Ok(())
    }

    /// 识别用户输入的意图
    pub fn recognize(&self, input: &str) -> Vec<IntentMatch> {
        let input_lower = input.to_lowercase();
        let mut matches: Vec<IntentMatch> = Vec::new();

        let patterns = self.patterns.read();

        for (intent, pattern_list) in patterns.iter() {
            for pattern in pattern_list {
                if !pattern.enabled {
                    continue;
                }

                let (matched, confidence, matched_keywords) =
                    self.evaluate_pattern(pattern, &input_lower);

                if matched && confidence > 0.0 {
                    matches.push(IntentMatch {
                        intent: *intent,
                        confidence,
                        matched_keywords,
                        entities: self.extract_entities(input, pattern),
                    });
                }
            }
        }

        // 按置信度降序排列
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));

        self.total_matches.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 如果没有匹配，返回 Unknown
        if matches.is_empty() {
            vec![IntentMatch {
                intent: IntentType::Unknown,
                confidence: 0.3,
                matched_keywords: Vec::new(),
                entities: HashMap::new(),
            }]
        } else {
            matches
        }
    }

    /// 获取最可能的意图
    pub fn top_intent(&self, input: &str) -> IntentMatch {
        self.recognize(input).into_iter().next().unwrap()
    }

    /// 评估单个模式
    fn evaluate_pattern(
        &self,
        pattern: &IntentPattern,
        input_lower: &str,
    ) -> (bool, f64, Vec<String>) {
        // 检查排除词
        for word in &pattern.excluded_words {
            if input_lower.contains(&word.to_lowercase()) {
                return (false, 0.0, Vec::new());
            }
        }

        // 检查必填词
        for word in &pattern.required_words {
            if !input_lower.contains(&word.to_lowercase()) {
                return (false, 0.0, Vec::new());
            }
        }

        let mut matched_keywords = Vec::new();
        let mut keyword_hits = 0;

        // 检查关键词
        for keyword in &pattern.keywords {
            if input_lower.contains(&keyword.to_lowercase()) {
                matched_keywords.push(keyword.clone());
                keyword_hits += 1;
            }
        }

        // 计算置信度
        let base_confidence = pattern.base_confidence;

        if keyword_hits > 0 {
            // 关键词越多置信度越高
            let keyword_bonus = (keyword_hits as f64 * 0.05).min(0.15);
            let confidence = (base_confidence + keyword_bonus).min(1.0);
            (true, confidence, matched_keywords)
        } else if !pattern.required_words.is_empty() {
            // 必填词匹配也算匹配
            (true, base_confidence * 0.8, matched_keywords)
        } else {
            (false, 0.0, Vec::new())
        }
    }

    /// 提取实体（简化版）
    fn extract_entities(&self, input: &str, _pattern: &IntentPattern) -> HashMap<String, String> {
        let mut entities = HashMap::new();

        // 提取数字
        for word in input.split_whitespace() {
            if let Ok(n) = word.parse::<i64>() {
                entities.insert("number".to_string(), n.to_string());
            }
        }

        entities
    }

    /// 获取所有已注册的意图类型
    pub fn registered_intents(&self) -> Vec<IntentType> {
        self.patterns.read().keys().copied().collect()
    }

    /// 获取总匹配次数
    pub fn total_matches(&self) -> u64 {
        self.total_matches.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for IntentRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_patterns() {
        let recognizer = IntentRecognizer::new();
        let intents = recognizer.registered_intents();
        assert!(intents.len() >= 5);
    }

    #[test]
    fn test_recognize_graph_query() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.top_intent("帮我查询知识图谱中的节点");
        assert_eq!(result.intent, IntentType::GraphQuery);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_recognize_knowledge_search() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.top_intent("搜索知识库中关于AI的文档");
        assert_eq!(result.intent, IntentType::KnowledgeSearch);
    }

    #[test]
    fn test_recognize_data_analysis() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.top_intent("分析一下最近的数据趋势");
        assert_eq!(result.intent, IntentType::DataAnalysis);
    }

    #[test]
    fn test_recognize_unknown() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.top_intent("今天天气怎么样");
        assert_eq!(result.intent, IntentType::Unknown);
    }

    #[test]
    fn test_multiple_intents() {
        let recognizer = IntentRecognizer::new();
        let results = recognizer.recognize("分析图谱数据并生成报表");
        assert!(results.len() >= 2);
        // 第一个应该是置信度最高的
        assert!(results[0].confidence >= results[1].confidence);
    }

    #[test]
    fn test_custom_pattern() {
        let recognizer = IntentRecognizer::new();
        let pattern = IntentPattern::new(IntentType::ChitChat)
            .with_keyword("你好")
            .with_keyword("hi")
            .with_keyword("hello");
        recognizer.register_pattern(pattern).unwrap();

        let result = recognizer.top_intent("你好啊");
        assert_eq!(result.intent, IntentType::ChitChat);
    }

    #[test]
    fn test_total_matches() {
        let recognizer = IntentRecognizer::new();
        assert_eq!(recognizer.total_matches(), 0);
        recognizer.recognize("测试");
        assert_eq!(recognizer.total_matches(), 1);
    }

    #[test]
    fn test_matched_keywords() {
        let recognizer = IntentRecognizer::new();
        let result = recognizer.top_intent("查询知识图谱的关系和节点");
        assert!(!result.matched_keywords.is_empty());
    }
}
