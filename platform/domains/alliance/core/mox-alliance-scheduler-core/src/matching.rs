// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 中文分词与任务描述 → 领域推断
//!
//! 解决原匹配器用 `split_whitespace()` 切词导致中文整句永不命中的缺陷：
//! - [`tokenize`]：中英混合分词（中文按单字 + 二元 bigram，英文按字母数字词），过滤停用词
//! - [`expert_text`]：把专家可检索文本（名称 + 描述 + 领域 + 能力）拼成一份索引文本
//! - [`description_overlap`]：计算任务 token 在专家文本 token 中的命中比例
//! - [`infer_domains`]：综合「描述重叠 + 领域词典」证据，推断任务所属领域
//!
//! 领域推断采用保守策略：证据数 >= 2 才推断，避免误过滤导致真正相关的专家被排除。

use mox_alliance_common_proto::{Expert, ExpertStatus};
use std::collections::HashMap;
use parking_lot::RwLock;

/// 常见停用词（中英）
const STOPWORDS: &[&str] = &[
    // 中文
    "的", "了", "和", "与", "在", "是", "我", "你", "他", "她", "它", "为", "对", "把", "被",
    "让", "请", "帮", "给", "等", "及", "并", "或", "之", "于", "这", "那",
    "一下", "一个", "这个", "那个", "我们", "你们", "它们", "他们", "她们", "以及", "或者",
    "进行", "提供", "关于", "可以", "需要", "能否", "是否", "如何", "什么", "为什么",
    "一种", "一些", "非常", "比较", "相关", "相应", "以及", "还有", "通过",
    // 英文
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "for", "with", "at", "by", "is",
    "are", "was", "were", "be", "been", "being", "this", "that", "these", "those", "it",
    "as", "from", "into", "about", "than", "then", "so", "but", "not", "no", "please",
    "help", "can", "could", "should", "would", "will", "shall", "do", "does", "did", "me",
    "my", "you", "your", "we", "our", "us", "they", "their", "he", "she", "his", "her",
    "i", "its", "has", "have", "had", "may", "might", "must", "need", "want", "using",
    "use", "used", "make", "made", "like", "also", "just", "only", "very", "any", "some",
    "such", "each", "both", "all", "most", "more", "there", "here", "where", "when", "who",
    "whom", "which", "what", "how", "why",
];

/// 通用/低信息量中文 bigram：跨领域高频、对领域推断几乎无区分度，
/// 在描述重叠证据中不计入命中，避免「分析/计算/实现」等通用词把无关
/// 专家顶到高证据（例：图像专家文本中的「计算机」「场景分析」误命中
/// 数学任务）。注意：不要放入领域强信号词（代码/财务/算法 等）。
const GENERIC_CJK_BIGRAMS: &[&str] = &[
    "分析", "计算", "实现", "时间", "输出", "要求", "描述", "结果", "过程", "数据", "信息",
    "方法", "模型", "进行", "需要", "使用", "提供", "内容", "支持", "相关", "根据", "问题",
    "知识", "能力", "专业", "领域", "专家", "任务", "包括", "能够", "以及", "处理", "通过",
    "主要", "擅长", "准确", "丰富", "帮助", "用于", "管理", "技术", "应用", "服务", "平台",
    "回答", "给出", "给定", "生成", "结合", "针对", "关于", "围绕", "面向", "涉及", "配合",
    "要求", "说明", "解释", "提供", "涵盖", "用户", "希望", "期望", "评估", "总结", "梳理",
    "介绍", "核心", "关键", "重要", "基础", "整体", "当前", "如下", "以上", "以下", "输入",
];

/// 领域词典：关键词 → 领域标签（用于短任务/英文任务的补充证据）
const DOMAIN_KEYWORDS: &[(&str, &str)] = &[
    // 代码编程
    ("代码", "code"), ("编程", "code"), ("程序", "code"), ("开发", "code"),
    ("接口", "code"), ("软件", "code"), ("工程师", "code"), ("调试", "code"),
    ("重构", "code"), ("bug", "code"), ("测试", "code"),
    ("rust", "code"), ("python", "code"), ("java", "code"), ("javascript", "code"),
    ("typescript", "code"), ("golang", "code"), ("c++", "code"), ("csharp", "code"),
    ("代码生成", "code"), ("code", "code"), ("coding", "code"), ("programming", "code"),
    // 数学推理
    ("数学", "mathematics"), ("证明", "mathematics"), ("微积分", "mathematics"),
    ("线性代数", "mathematics"), ("概率", "mathematics"), ("统计", "mathematics"),
    ("数学建模", "mathematics"), ("方程", "mathematics"), ("几何", "mathematics"),
    ("算法", "mathematics"), ("递归", "mathematics"), ("复杂度", "mathematics"),
    ("时间复杂度", "mathematics"), ("空间复杂度", "mathematics"), ("数列", "mathematics"),
    ("斐波那契", "mathematics"), ("动态规划", "mathematics"), ("导数", "mathematics"),
    ("积分", "mathematics"), ("极限", "mathematics"), ("函数", "mathematics"),
    ("不等式", "mathematics"), ("矩阵", "mathematics"), ("概率论", "mathematics"),
    ("数论", "mathematics"), ("拓扑", "mathematics"), ("图论", "mathematics"),
    ("逻辑推理", "mathematics"), ("数值计算", "mathematics"), ("统计分析", "mathematics"),
    ("mathematics", "mathematics"), ("math", "mathematics"), ("equation", "mathematics"),
    ("calculus", "mathematics"), ("algebra", "mathematics"), ("probability", "mathematics"),
    ("recursion", "mathematics"), ("complexity", "mathematics"), ("algorithm", "mathematics"),
    // 医学咨询
    ("医学", "medical"), ("医疗", "medical"), ("疾病", "medical"), ("药物", "medical"),
    ("健康", "medical"), ("症状", "medical"), ("医院", "medical"), ("临床", "medical"),
    ("病理", "medical"), ("药学", "medical"), ("诊", "medical"),
    ("头痛", "medical"), ("失眠", "medical"), ("感冒", "medical"), ("咳嗽", "medical"),
    ("发烧", "medical"), ("疼痛", "medical"), ("头晕", "medical"), ("恶心", "medical"),
    ("乏力", "medical"), ("疲劳", "medical"), ("腹泻", "medical"), ("呕吐", "medical"),
    ("医生", "medical"), ("看病", "medical"), ("体检", "medical"), ("检查", "medical"),
    ("治疗", "medical"), ("手术", "medical"), ("处方", "medical"), ("用药", "medical"),
    ("血压", "medical"), ("血糖", "medical"), ("肿瘤", "medical"), ("感染", "medical"),
    ("过敏", "medical"), ("疫苗", "medical"), ("化验", "medical"), ("病情", "medical"),
    ("痊愈", "medical"), ("康复", "medical"), ("慢性病", "medical"), ("传染病", "medical"),
    ("medical", "medical"), ("disease", "medical"), ("doctor", "medical"),
    ("clinic", "medical"), ("healthcare", "medical"), ("symptom", "medical"),
    ("headache", "medical"), ("fever", "medical"), ("cough", "medical"),
    ("pain", "medical"), ("patient", "medical"), ("diagnosis", "medical"),
    ("treatment", "medical"), ("illness", "medical"), ("therapy", "medical"),
    // 法律咨询
    ("法律", "law"), ("合同", "law"), ("法条", "law"), ("合规", "law"),
    ("劳动法", "law"), ("知识产权", "law"), ("诉讼", "law"), ("律师", "law"),
    ("法规", "law"), ("立法", "law"),
    ("law", "law"), ("contract", "law"), ("legal", "law"), ("lawsuit", "law"),
    ("regulation", "law"), ("compliance", "law"),
    // 金融分析
    ("金融", "finance"), ("财务", "finance"), ("投资", "finance"), ("股票", "finance"),
    ("股价", "finance"), ("基金", "finance"), ("债券", "finance"), ("估值", "finance"),
    ("财报", "finance"), ("资产", "finance"), ("收益", "finance"), ("融资", "finance"),
    ("市值", "finance"), ("行情", "finance"), ("理财", "finance"), ("炒股", "finance"),
    ("finance", "finance"), ("financial", "finance"), ("investment", "finance"),
    ("stock", "finance"), ("portfolio", "finance"), ("equity", "finance"),
    ("bond", "finance"), ("valuation", "finance"), ("trading", "finance"),
    ("budget", "finance"), ("revenue", "finance"), ("earnings", "finance"),
    // 创意写作
    ("文案", "creative"), ("故事", "creative"), ("写作", "creative"), ("剧本", "creative"),
    ("广告", "creative"), ("品牌", "creative"), ("创意", "creative"), ("小说", "creative"),
    ("散文", "creative"), ("诗歌", "creative"), ("营销", "creative"), ("剧情", "creative"),
    ("漫画", "creative"), ("文案创作", "creative"),
    ("creative", "creative"), ("copywriting", "creative"), ("story", "creative"),
    ("script", "creative"), ("novel", "creative"), ("poem", "creative"),
    ("write", "creative"), ("writing", "creative"), ("content", "creative"),
    // 图像理解
    ("图像", "vision"), ("图片", "vision"), ("视觉", "vision"), ("识别", "vision"),
    ("画面", "vision"), ("照片", "vision"), ("图表解读", "vision"), ("ocr", "vision"),
    ("image", "vision"), ("vision", "vision"), ("photo", "vision"),
    ("picture", "vision"), ("visual", "vision"), ("detect", "vision"),
    // 多语言翻译
    ("翻译", "translation"), ("译", "translation"), ("本地化", "translation"),
    ("术语", "translation"), ("审校", "translation"), ("语言", "translation"),
    ("translation", "translation"), ("translate", "translation"),
    ("translator", "translation"), ("localization", "translation"),
    ("chinese", "translation"), ("english", "translation"),
    // 学术研究
    ("研究", "research"), ("论文", "research"), ("文献", "research"), ("学术", "research"),
    ("实验", "research"), ("综述", "research"), ("方法学", "research"), ("科研", "research"),
    ("引用", "research"), ("课题", "research"),
    ("research", "research"), ("paper", "research"), ("literature", "research"),
    ("academic", "research"), ("experiment", "research"), ("citation", "research"),
    // 架构设计
    ("架构", "architecture"), ("系统设计", "architecture"), ("微服务", "architecture"),
    ("分布式", "architecture"), ("云原生", "architecture"), ("架构设计", "architecture"),
    ("高并发", "architecture"), ("可扩展", "architecture"), ("部署", "architecture"),
    ("architecture", "architecture"), ("system design", "architecture"),
    ("microservice", "architecture"), ("distributed", "architecture"),
    ("cloud native", "architecture"), ("scalable", "architecture"),
];

/// 判断字符是否属于 CJK 统一表意文字区
fn is_cjk(c: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&c) || ('\u{3400}'..='\u{4dbf}').contains(&c)
}

/// 把一个连续 CJK 片段切分为单字 + bigram（过滤停用词）
fn push_cjk_run(run: &str, tokens: &mut Vec<String>) {
    if run.is_empty() {
        return;
    }
    let chars: Vec<char> = run.chars().collect();
    if chars.len() == 1 {
        let s = run.to_string();
        if !STOPWORDS.contains(&s.as_str()) {
            tokens.push(s);
        }
    } else {
        for w in chars.windows(2) {
            let s: String = w.iter().collect();
            if !STOPWORDS.contains(&s.as_str()) {
                tokens.push(s);
            }
        }
    }
}

/// 把一个英文/数字词加入 token（过滤长度 < 2 与停用词）
fn push_word(word: &str, tokens: &mut Vec<String>) {
    if word.is_empty() {
        return;
    }
    let w = word.to_lowercase();
    if w.len() >= 2 && !STOPWORDS.contains(&w.as_str()) {
        tokens.push(w);
    }
}

/// 中英混合分词
///
/// - 中文连续片段按「单字 + 二元 bigram」切分（字符 bigram）
/// - 英文/数字按字母数字序列切分并小写化
/// - 统一过滤停用词与长度 < 2 的项
pub fn tokenize(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens: Vec<String> = Vec::new();
    let mut cjk_run = String::new();
    let mut word = String::new();

    for ch in lower.chars() {
        if is_cjk(ch) {
            // 关闭英文词
            if !word.is_empty() {
                push_word(&word, &mut tokens);
                word.clear();
            }
            cjk_run.push(ch);
        } else {
            // 关闭 CJK 片段
            if !cjk_run.is_empty() {
                push_cjk_run(&cjk_run, &mut tokens);
                cjk_run.clear();
            }
            if ch.is_alphanumeric() {
                word.push(ch);
            } else {
                push_word(&word, &mut tokens);
                word.clear();
            }
        }
    }
    push_cjk_run(&cjk_run, &mut tokens);
    push_word(&word, &mut tokens);

    tokens
}

/// 拼接专家的可检索文本（名称 + 描述 + 领域 + 能力名 + 能力描述）
pub fn expert_text(expert: &Expert) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !expert.name.is_empty() {
        parts.push(expert.name.clone());
    }
    if !expert.description.is_empty() {
        parts.push(expert.description.clone());
    }
    for d in &expert.domains {
        parts.push(d.clone());
    }
    for c in &expert.capabilities {
        if !c.name.is_empty() {
            parts.push(c.name.clone());
        }
        if !c.description.is_empty() && c.description != c.name {
            parts.push(c.description.clone());
        }
    }
    parts.join(" ")
}

/// 判断 token 是否全是 ASCII 字母数字（英文/数字词）
fn is_ascii_word(token: &str) -> bool {
    !token.is_empty() && token.chars().all(|c| c.is_ascii_alphanumeric())
}

/// 两个 ASCII 词的最长公共前缀长度
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count()
}

/// 两个 token 是否命中（支持英文词干/词形匹配）
///
/// - 完全相等直接命中
/// - 两个均为 ASCII 字母数字词且长度 >= 3 时：
///   - 一方是另一方的前缀（investments~investment、coding~code）
///   - 或共享词根 >= 4 个字符（financial~finance 共根 "financ"）
fn token_hit(query: &str, expert: &str) -> bool {
    if query == expert {
        return true;
    }
    if is_ascii_word(query) && is_ascii_word(expert) && query.len() >= 3 && expert.len() >= 3 {
        if query.starts_with(expert) || expert.starts_with(query) {
            return true;
        }
        if common_prefix_len(query, expert) >= 4 {
            return true;
        }
    }
    false
}

/// 专家文本分词缓存
///
/// 缓存 expert_id → tokenized tokens，避免匹配器遍历所有专家时
/// 对同一份专家文本重复执行 tokenize（O(N*M) 重复分词 → O(N) 一次分词）。
///
/// 假设专家注册后文本不变；若专家文本更新，调用 [`ExpertTokenCache::invalidate`]。
pub struct ExpertTokenCache {
    inner: RwLock<HashMap<String, Vec<String>>>,
}

impl ExpertTokenCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// 获取或计算专家文本的分词结果（命中则 clone 返回，未命中则 tokenize 后写入缓存）
    pub fn get_or_compute(&self, expert: &Expert) -> Vec<String> {
        // 先读缓存（读锁，无竞争时快速路径）
        if let Some(tokens) = self.inner.read().get(&expert.expert_id) {
            return tokens.clone();
        }
        // 未命中：计算并写入（写锁）
        let tokens = tokenize(&expert_text(expert));
        self.inner
            .write()
            .insert(expert.expert_id.clone(), tokens.clone());
        tokens
    }

    /// 失效指定专家的缓存（专家文本更新时调用）
    pub fn invalidate(&self, expert_id: &str) {
        self.inner.write().remove(expert_id);
    }

    /// 清空全部缓存
    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

impl Default for ExpertTokenCache {
    fn default() -> Self {
        Self::new()
    }
}

/// 计算任务 token 在专家文本 token 中的命中情况
///
/// 返回 `(命中比例 0.0-1.0, 命中 token 数)`。无任务 token 时返回 (0.3, 0)（中性分）。
pub fn description_overlap(
    expert: &Expert,
    query_tokens: &[String],
    cache: Option<&ExpertTokenCache>,
) -> (f64, usize) {
    if query_tokens.is_empty() {
        return (0.3, 0);
    }
    let expert_tokens: Vec<String> = match cache {
        Some(c) => c.get_or_compute(expert),
        None => tokenize(&expert_text(expert)),
    };
    let mut hit = 0usize;
    for t in query_tokens {
        // 通用低信息量词不计入重叠证据（避免误报）
        if GENERIC_CJK_BIGRAMS.contains(&t.as_str()) {
            continue;
        }
        if expert_tokens.iter().any(|et| token_hit(t, et)) {
            hit += 1;
        }
    }
    (hit as f64 / query_tokens.len() as f64, hit)
}

/// 词典命中：返回关键词匹配到的领域列表
///
/// 判定规则：**关键词自身的全部 token 均须命中查询 token 集**，才计一次领域证据。
/// - 双字中文关键词（递归/财务）：其 token 就是该 bigram，查询中存在即命中；
/// - 多字中文关键词（动态规划/时间复杂度/斐波那契）：拆分为 bigram 后
///   全部存在于查询中才命中，天然抗误报（"动态规划" 不会因查询含单个"动态"命中）；
/// - ASCII 关键词沿用词干前缀/共享词根匹配（investments→investment）。
fn lexicon_match_all(query_tokens: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (kw, domain) in DOMAIN_KEYWORDS {
        let kw_tokens: Vec<String> = tokenize(kw);
        if kw_tokens.is_empty() {
            continue;
        }
        let all_hit = kw_tokens
            .iter()
            .all(|kt| query_tokens.iter().any(|qt| token_hit(qt, kt)));
        if all_hit && !out.contains(&domain.to_string()) {
            out.push(domain.to_string());
        }
    }
    out
}

/// 从任务描述推断所属领域
///
/// 综合两类证据：
/// 1. **描述重叠**：任务 token 命中专家文本 token（bigram/词）的数量
/// 2. **领域词典**：任务 token 命中领域关键词的数量
///
/// 仅当某领域的累计证据数 >= 2 时推断该领域（保守策略，避免误过滤）。
/// 返回按证据强度降序、最多 3 个的领域标签。
pub fn infer_domains(
    description: &str,
    experts: &[Expert],
    cache: Option<&ExpertTokenCache>,
) -> Vec<String> {
    let q_tokens = tokenize(description);
    if q_tokens.is_empty() {
        return vec![];
    }

    // 领域 → (证据数, 最大描述重叠命中数)
    let mut evidence: HashMap<String, usize> = HashMap::new();

    // 证据 1：描述重叠（取该领域下所有专家的最大命中数）
    for e in experts {
        if e.status != ExpertStatus::Active {
            continue;
        }
        let (_, hits) = description_overlap(e, &q_tokens, cache);
        if hits == 0 {
            continue;
        }
        // 该专家的领域各获得其命中数证据（取同领域最大值，见下方 max 逻辑）
        for d in &e.domains {
            let entry = evidence.entry(d.clone()).or_insert(0);
            *entry = (*entry).max(hits);
        }
    }

    // 证据 2：领域词典（复合词子集匹配；词典命中为高精度强证据，权重 +2）
    for d in lexicon_match_all(&q_tokens) {
        let entry = evidence.entry(d).or_insert(0);
        *entry += 2;
    }

    let mut ranked: Vec<(String, usize)> = evidence.into_iter().collect();
    ranked.retain(|(_, ev)| *ev >= 2);
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(3);
    ranked.into_iter().map(|(d, _)| d).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::Capability;

    fn make_expert(
        id: &str,
        name: &str,
        description: &str,
        domains: &[&str],
        caps: &[&str],
    ) -> Expert {
        let mut e = Expert::new_system(name.to_string(), description.to_string());
        e.expert_id = id.to_string();
        e.domains = domains.iter().map(|s| s.to_string()).collect();
        e.capabilities = caps
            .iter()
            .map(|c| Capability {
                capability_id: format!("cap-{}", c),
                name: c.to_string(),
                description: c.to_string(),
                domain: "general".to_string(),
                version: "1.0.0".to_string(),
            })
            .collect();
        e
    }

    #[test]
    fn infer_math_domain_algorithm_task() {
        // 真实内置模块配置复刻 server.rs::expert_from_module 的专家文本；
        // 回归用例：算法/复杂度任务不得被图像专家「计算机/场景分析」的通用词误判
        use mox_alliance_config_core::examples::domain_experts::build_domain_experts;
        let modules = build_domain_experts();
        let experts: Vec<Expert> = modules
            .iter()
            .map(|m| {
                let mut e = Expert::new_system(m.name.clone(), m.name.clone());
                e.expert_id = m.expert_id.clone();
                e.description = match &m.llm_config.system_prompt_template {
                    Some(tpl) => format!("{}. {}", m.name, tpl),
                    None => m.name.clone(),
                };
                e.domains = m.tags.clone();
                e
            })
            .collect();
        let desc = "计算斐波那契数列第20项的值，并分析朴素递归与动态规划两种实现的时间复杂度差异";
        let domains = infer_domains(desc, &experts, None);
        eprintln!("INFERRED={:?}", domains);
        assert!(
            domains.contains(&"mathematics".to_string()),
            "算法/递归任务应推断数学领域: {:?}",
            domains
        );
        assert!(
            !domains.iter().any(|d| d == "image" || d == "vision"),
            "数学任务被误判为图像领域: {:?}",
            domains
        );
    }

    #[test]
    fn tokenize_cjk_bigram() {
        let toks = tokenize("财务报表分析");
        // 2字: 财务, 务报, 报表, 表分, 分析
        assert!(toks.contains(&"财务".to_string()));
        assert!(toks.contains(&"分析".to_string()));
        assert!(toks.contains(&"报表".to_string()));
    }

    #[test]
    fn tokenize_english_word() {
        let toks = tokenize("Financial analysis");
        assert!(toks.contains(&"financial".to_string()));
        assert!(toks.contains(&"analysis".to_string()));
        // 停用词被过滤
        assert!(!toks.contains(&"the".to_string()));
    }

    #[test]
    fn tokenize_mixed() {
        let toks = tokenize("Rust Web服务登录接口");
        assert!(toks.contains(&"rust".to_string()));
        assert!(toks.contains(&"登录".to_string()));
        assert!(toks.contains(&"接口".to_string()));
    }

    #[test]
    fn description_overlap_chinese() {
        let expert = make_expert(
            "e1",
            "金融分析专家",
            "你是金融分析师。擅长：财务报表分析、投资研究、风险评估、金融建模、市场分析、资产配置。",
            &["finance", "investment"],
            &["财务分析"],
        );
        let toks = tokenize("分析2026年第二季度财务报表并给出投资建议");
        let (ratio, hits) = description_overlap(&expert, &toks, None);
        assert!(hits >= 3, "hits={}", hits);
        assert!(ratio > 0.2, "ratio={}", ratio);
    }

    #[test]
    fn infer_finance_domain_chinese() {
        let experts = vec![
            make_expert(
                "finance",
                "金融分析专家",
                "擅长：财务报表分析、投资研究、风险评估、金融建模、市场分析、资产配置。",
                &["finance", "investment"],
                &["财务分析"],
            ),
            make_expert(
                "code",
                "代码编程专家",
                "擅长：Python/Rust/TypeScript/Go/Java 等多种语言、系统架构设计、算法优化、代码重构。",
                &["programming", "code"],
                &["代码生成"],
            ),
        ];
        let domains = infer_domains("分析2026年第二季度财务报表并给出投资建议", &experts, None);
        assert!(domains.contains(&"finance".to_string()), "{:?}", domains);
        assert!(!domains.contains(&"code".to_string()), "{:?}", domains);
    }

    #[test]
    fn infer_finance_domain_english() {
        let experts = vec![
            make_expert(
                "finance",
                "金融分析专家",
                "擅长：财务报表分析、投资研究、风险评估、金融建模、市场分析、资产配置。",
                &["finance", "investment"],
                &["财务分析"],
            ),
            make_expert(
                "code",
                "代码编程专家",
                "擅长：Python/Rust/TypeScript/Go/Java 等多种语言、代码重构。",
                &["programming", "code"],
                &["代码生成"],
            ),
        ];
        let domains =
            infer_domains("financial statement analysis and investment recommendation", &experts, None);
        assert!(domains.contains(&"finance".to_string()), "{:?}", domains);
        assert!(!domains.contains(&"code".to_string()), "{:?}", domains);
    }

    #[test]
    fn infer_finance_domain_english_plural() {
        // 复数/派生词形（investments、financial）通过词干前缀匹配命中 finance/investment
        let experts = vec![
            make_expert(
                "finance",
                "金融分析专家",
                "擅长：财务报表分析、投资研究、风险评估。",
                &["finance", "investment"],
                &["财务分析"],
            ),
            make_expert(
                "code",
                "代码编程专家",
                "擅长：Python/Rust 代码生成。",
                &["programming", "code"],
                &["代码生成"],
            ),
        ];
        let domains = infer_domains(
            "Analyze financial statements and recommend investments",
            &experts,
            None,
        );
        assert!(domains.contains(&"finance".to_string()), "{:?}", domains);
        assert!(!domains.contains(&"code".to_string()), "{:?}", domains);
    }

    #[test]
    fn description_overlap_english_stem() {
        // 词形变化仍能命中专家文本中的领域词
        let expert = make_expert(
            "e1",
            "金融分析专家",
            "擅长：财务报表分析、投资研究、风险评估、金融建模、市场分析、资产配置。",
            &["finance", "investment"],
            &["财务分析"],
        );
        let toks = tokenize("Analyze financial statements and recommend investments");
        let (ratio, hits) = description_overlap(&expert, &toks, None);
        // financial→finance、investments→investment 命中
        assert!(hits >= 2, "hits={}", hits);
        assert!(ratio > 0.0, "ratio={}", ratio);
    }

    #[test]
    fn infer_code_domain_mixed() {
        let experts = vec![
            make_expert(
                "code",
                "代码编程专家",
                "擅长：Python/Rust/TypeScript/Go/Java 等多种语言、系统架构设计、算法优化、代码重构、调试排错。",
                &["programming", "code"],
                &["代码生成"],
            ),
            make_expert(
                "finance",
                "金融分析专家",
                "擅长：财务报表分析、投资研究、风险评估、金融建模。",
                &["finance", "investment"],
                &["财务分析"],
            ),
        ];
        let domains = infer_domains("用Rust写一个Web服务的登录接口", &experts, None);
        assert!(domains.contains(&"code".to_string()), "{:?}", domains);
        assert!(!domains.contains(&"finance".to_string()), "{:?}", domains);
    }

    #[test]
    fn infer_medical_domain_chinese_symptom() {
        let experts = vec![
            make_expert(
                "medical",
                "医学咨询专家",
                "擅长：疾病诊断、症状分析、用药建议、健康管理。",
                &["medical", "healthcare"],
                &["症状分析"],
            ),
            make_expert(
                "code",
                "代码编程专家",
                "擅长：Python/Rust 代码生成。",
                &["programming", "code"],
                &["代码生成"],
            ),
        ];
        // 症状词（头痛/失眠/医生）通过领域词典命中 medical
        let domains = infer_domains("我最近经常头痛失眠，需要看医生吗", &experts, None);
        assert!(domains.contains(&"medical".to_string()), "{:?}", domains);
        assert!(!domains.contains(&"code".to_string()), "{:?}", domains);
    }

    #[test]
    fn generic_query_no_domain_inferred() {
        let experts = vec![
            make_expert("e1", "金融分析专家", "擅长：财务报表分析。", &["finance"], &["财务"]),
            make_expert("e2", "代码编程专家", "擅长：代码生成。", &["code"], &["代码"]),
        ];
        // 弱信号：只有 1 个证据，不应推断（保守）
        let domains = infer_domains("请帮我分析一下", &experts, None);
        assert!(domains.is_empty(), "{:?}", domains);
    }
}
