//! 统一知识字典（单一事实源 / Single Source of Truth）
//!
//! 收敛对话开发系统三处分散的知识字典，消除重复维护与漂移风险：
//! 1. 对话引擎算子推荐关键词（原 `conversation::build_operator_knowledge`）
//! 2. 需求编译器 动作动词 / 实体名词 / 动词别名（原 `requirement_compiler` 三常量）
//! 3. 对话图谱 规则抽取已知项（原 `dialogue_graph::rule_based_extract` 的 `known`）
//!
//! 约定：本模块仅承载**数据**，不含逻辑；各消费方按需转换（如 `String` / `Vec` 化）。

use crate::flow_engine::NodeType;

/// 算子 → 中文/英文关键词（意图识别后用于算子推荐）
pub const OPERATOR_KEYWORDS: &[(&str, &[&str])] = &[
    ("identity", &["恒等", "直接", "不变", "passthrough"]),
    ("linear", &["线性", "变换", "缩放", "矩阵", "乘法"]),
    ("normalize", &["归一化", "标准化", "norm", "单位向量"]),
    ("relu", &["relu", "激活", "整流", "非线性"]),
    ("sigmoid", &["sigmoid", "S型", "概率", "0到1"]),
    ("tanh", &["tanh", "双曲正切", "-1到1"]),
    ("softmax", &["softmax", "指数归一化", "分类", "概率分布"]),
    ("matmul", &["矩阵乘法", "matmul", "线性变换"]),
    ("conv2d", &["卷积", "conv", "CNN", "特征提取"]),
    (
        "attention",
        &["注意力", "attention", "transformer", "自注意力"],
    ),
    ("adam", &["adam", "优化器", "训练", "梯度下降"]),
];

/// 算子分类 → 关键词（兜底推荐）
pub const OPERATOR_CATEGORY_KEYWORDS: &[(&str, &[&str])] = &[
    ("core", &["基础", "核心"]),
    ("activation", &["激活", "非线性"]),
    ("math", &["数学", "计算"]),
    ("ai", &["AI", "机器学习", "深度学习", "神经网络"]),
    ("signal", &["信号", "图像处理"]),
    ("optimizer", &["优化", "训练"]),
];

/// 全部已知算子（抽取用户消息中提及的算子）
pub const ALL_OPERATORS: &[&str] = &[
    "identity",
    "linear",
    "normalize",
    "normalize_l1",
    "relu",
    "sigmoid",
    "tanh",
    "softmax",
    "scale",
    "add_bias",
    "matmul",
    "conv2d",
    "maxpool",
    "attention",
    "self_attention",
    "cross_attention",
    "feedforward",
    "embedding",
    "adam",
    "sgd",
];

/// 需求编译：动作动词 → 节点类型
pub const REQUIREMENT_ACTION_VERBS: &[(&str, NodeType)] = &[
    ("支付", NodeType::Operator),
    ("下单", NodeType::Operator),
    ("购买", NodeType::Operator),
    ("登录", NodeType::Operator),
    ("注册", NodeType::Operator),
    ("上传", NodeType::Operator),
    ("发布", NodeType::Operator),
    ("审核", NodeType::Operator),
    ("生成", NodeType::LLM),
    ("推荐", NodeType::LLM),
    ("校验", NodeType::Transform),
    ("判断", NodeType::Condition),
    ("检查", NodeType::Transform),
    ("通知", NodeType::DataOutput),
];

/// 需求编译：从一句话里识别的"实体名词" → 数据表候选
pub const REQUIREMENT_ENTITY_NOUNS: &[&str] = &[
    "商品",
    "用户",
    "订单",
    "购物车",
    "支付",
    "评论",
    "文章",
    "小说",
    "论文",
    "图书",
    "视频",
    "产品",
    "库存",
    "会员",
    "日志",
];

/// 需求编译：动作动词 → 实体别名（"下单"语义指向"订单"实体）
pub const REQUIREMENT_VERB_TO_ENTITY: &[(&str, &str)] = &[
    ("下单", "订单"),
    ("购买", "商品"),
    ("支付", "订单"),
    ("加购", "购物车"),
    ("收藏", "商品"),
];

/// 对话图谱规则抽取：已知关键词 → 实体类型
pub const DIALOGUE_KNOWN_ENTITIES: &[(&str, &str)] = &[
    ("线性变换", "operator"),
    ("激活函数", "operator"),
    ("归一化", "operator"),
    ("卷积", "operator"),
    ("注意力", "operator"),
    ("注意力机制", "algorithm"),
    ("PageRank", "algorithm"),
    ("最短路径", "algorithm"),
    ("社区发现", "algorithm"),
    ("工作流", "workflow"),
    ("插件", "capability"),
    ("资源管理", "capability"),
    ("浏览器自动化", "capability"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_keywords_have_unique_keys() {
        let mut seen = std::collections::HashSet::new();
        for (op, _kws) in OPERATOR_KEYWORDS {
            assert!(seen.insert(*op), "OPERATOR_KEYWORDS 存在重复算子键: {op}");
        }
        assert!(!OPERATOR_KEYWORDS.is_empty());
    }

    #[test]
    fn all_operators_cover_keyword_keys() {
        let kw_keys: std::collections::HashSet<&str> =
            OPERATOR_KEYWORDS.iter().map(|(op, _)| *op).collect();
        for k in kw_keys {
            assert!(
                ALL_OPERATORS.contains(&k),
                "ALL_OPERATORS 遗漏 OPERATOR_KEYWORDS 的算子: {k}"
            );
        }
        assert!(!ALL_OPERATORS.is_empty());
    }

    #[test]
    fn dialogue_known_entities_have_unique_keys() {
        let mut seen = std::collections::HashSet::new();
        for (kw, _ty) in DIALOGUE_KNOWN_ENTITIES {
            assert!(
                seen.insert(*kw),
                "DIALOGUE_KNOWN_ENTITIES 存在重复关键词: {kw}"
            );
        }
        assert!(!DIALOGUE_KNOWN_ENTITIES.is_empty());
    }

    #[test]
    fn requirement_dicts_non_empty() {
        assert!(!REQUIREMENT_ACTION_VERBS.is_empty());
        assert!(!REQUIREMENT_ENTITY_NOUNS.is_empty());
        assert!(!REQUIREMENT_VERB_TO_ENTITY.is_empty());
    }
}
