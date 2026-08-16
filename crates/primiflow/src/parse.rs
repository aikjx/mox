//! 层2 · 需求解析与结构化建模服务
//!
//! 输入：用户自然语言业务描述（一句话或多段）。
//! 输出：标准化结构化需求树 [`ParsedRequirement`]，并可一键映射为
//! [`crate::runner::Spec`]（子任务规格），直接喂给 [`crate::runner::run_pipeline`]
//! 跑 κ‑τ 自涌现闭环。
//!
//! 所有需求/功能/业务流/代码均由需求 ID 唯一溯源（对应 PrimiFlow 五向绑定主键）。
//! 解析采用确定性启发式（关键词分类 + 分句），不依赖外部大模型，保证离线可跑、可复现。

use crate::runner::Spec;
use flow_ai::model::ToolKind;
use flow_ai::primitive::DeliveryPolicy;

/// 子任务能力类别：同时作为知识库复用的稳定标识（同类意图命中同一 key，自动抬高 κ）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// 抓取 / 拉取 / 接入外部数据
    Fetch,
    /// 清洗 / 计算 / 特征 / 向量化
    Compute,
    /// 报告 / 图表 / 汇总 / 分析（LLM 生成）
    Llm,
    /// 入库 / 检索 / 查询（数据库）
    Database,
    /// 告警 / 下发 / 执行脚本
    Shell,
}

impl Category {
    /// 稳定的 ascii 标识（同时是知识库复用 key 与代码节点标识）
    pub fn key(self) -> &'static str {
        match self {
            Category::Fetch => "fetch",
            Category::Compute => "compute",
            Category::Llm => "report",
            Category::Database => "db",
            Category::Shell => "shell",
        }
    }

    /// 该类别默认使用的工具原语
    pub fn tool(self) -> ToolKind {
        match self {
            Category::Fetch => ToolKind::Http,
            Category::Compute => ToolKind::Compute,
            Category::Llm => ToolKind::Llm,
            Category::Database => ToolKind::Database,
            Category::Shell => ToolKind::Shell,
        }
    }

    /// 该类别默认算力预算（ms），用于守恒预算闸门
    pub fn default_ms(self) -> u64 {
        match self {
            Category::Fetch => 300,
            Category::Compute => 250,
            Category::Llm => 400,
            Category::Database => 250,
            Category::Shell => 300,
        }
    }
}

/// 定时/周期调度信息（对应蓝图「层4 算法编排执行层」的 Cron 自动生成）
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule {
    /// 推测的 Cron 表达式（离线启发式，非权威）
    pub cron: String,
    /// 人类可读描述
    pub desc: String,
}

/// 解析出的单个子任务意图
#[derive(Debug, Clone)]
pub struct ParsedSubtask {
    pub label: String,
    pub category: Category,
}

/// 结构化需求树（层2 的核心输出物）
#[derive(Debug, Clone)]
pub struct ParsedRequirement {
    pub raw: String,
    pub goal: String,
    pub roles: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub rules: Vec<String>,
    pub constraints: Vec<String>,
    pub schedule: Option<Schedule>,
    pub external_systems: Vec<String>,
    pub subtasks: Vec<ParsedSubtask>,
    pub policy: DeliveryPolicy,
}

impl ParsedRequirement {
    /// 转成 [`Spec`]，可直接喂给 `run_pipeline`
    pub fn to_spec(&self) -> Spec {
        let mut spec = Spec::new(&slug(&self.goal), &self.goal, self.policy);
        let mut seen: std::collections::HashSet<String> = Default::default();
        for (i, st) in self.subtasks.iter().enumerate() {
            // 同类意图复用同一 key（命中知识库抬高 κ）；同需求内重复则加序号避免碰撞
            let mut key = st.category.key().to_string();
            while !seen.insert(key.clone()) {
                key = format!("{}_{}", st.category.key(), i);
            }
            spec = spec.sub(&key, &st.label, st.category.tool(), st.category.default_ms());
        }
        spec
    }
}

/// 把自然语言业务描述解析为结构化需求树
pub fn parse(text: &str) -> ParsedRequirement {
    let raw = text.trim().to_string();
    let clauses = split_clauses(&raw);

    // 1) 调度检测
    let schedule = detect_schedule(&raw);

    // 2) 角色 / 外部系统 / 约束
    let roles = extract_list(
        &raw,
        &["角色", "用户", "管理员", "运营", "客服", "员工", "客户", "商家", "分析师"],
    );
    let external_systems = extract_list(
        &raw,
        &[
            "对接", "数据库", "PostgreSQL", "MySQL", "Redis", "Kafka", "API", "第三方", "支付",
            "微信", "短信", "erp", "ERP", "CRM",
        ],
    );
    let constraints = extract_list(
        &raw,
        &["限制", "约束", "不能超过", "不超过", "要求", "必须", "禁止", "不得"],
    );

    // 3) 交付策略
    let policy = detect_policy(&raw);

    // 4) 子任务分类（每个从句可映射到一个或多个能力类别，按优先级顺序展开）
    let mut subtasks = Vec::new();
    let mut goal = raw.clone();
    for c in &clauses {
        if c.trim().is_empty() {
            continue;
        }
        let cats = classify(c);
        if !cats.is_empty() {
            for cat in cats {
                subtasks.push(ParsedSubtask {
                    label: normalize_label(c),
                    category: cat,
                });
            }
        } else if goal == raw && c.len() >= 4 {
            // 首个无法分类的较长从句作为业务目标
            goal = c.trim().to_string();
        }
    }

    // 5) 兜底：若未解析出任何子任务，给出最小可跑闭环（抓取→报告）
    if subtasks.is_empty() {
        subtasks.push(ParsedSubtask {
            label: "抓取源数据".into(),
            category: Category::Fetch,
        });
        subtasks.push(ParsedSubtask {
            label: "生成图表报告".into(),
            category: Category::Llm,
        });
    }

    ParsedRequirement {
        raw: raw.clone(),
        goal,
        roles,
        inputs: extract_list(&raw, &["输入", "来源", "数据源", "对接"]),
        outputs: extract_list(&raw, &["输出", "产出", "生成", "报告", "图表"]),
        rules: extract_list(&raw, &["规则", "逻辑", "当", "如果", "若"]),
        constraints,
        schedule,
        external_systems,
        subtasks,
        policy,
    }
}

/// 便捷封装：自然语言 → 可直接运行的 [`Spec`]
pub fn parse_to_spec(text: &str) -> Spec {
    parse(text).to_spec()
}

// ————————————————————————————————————————————————————————————
// 内部启发式实现
// ————————————————————————————————————————————————————————————

/// 按句号/分号/换行/连接词切分为业务从句
fn split_clauses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in text.split(|c| "。；;\n！!".contains(c)) {
        for sub in part.split(|c| "，,、".contains(c)) {
            let s = sub.trim().to_string();
            if !s.is_empty() {
                out.push(s);
            }
        }
    }
    out
}

/// 从句 → 能力类别集合（按优先级顺序：先 Shell/DB，再 Fetch/Compute/Llm；从句可同时命中多个）
fn classify(clause: &str) -> Vec<Category> {
    let c = clause;
    let mut out = Vec::new();
    if contains_any(c, &["告警", "下发", "执行", "脚本", "命令", "通知", "推送"]) {
        out.push(Category::Shell);
    }
    if contains_any(c, &["入库", "落库", "存储", "写库", "检索", "查询", "查库", "保存至", "同步到"]) {
        out.push(Category::Database);
    }
    if contains_any(c, &["抓取", "拉取", "采集", "接入", "爬取", "读取", "获取", "下载", "订阅"]) {
        out.push(Category::Fetch);
    }
    if contains_any(c, &["清洗", "对账", "核算", "计算", "特征", "向量化", "聚类", "分析", "建模", "预测", "转换"]) {
        out.push(Category::Compute);
    }
    if contains_any(c, &["报告", "图表", "汇总", "生成", "绘制", "可视化", "撰写", "总结", "提示"]) {
        out.push(Category::Llm);
    }
    out
}

/// 把从句规整为子任务标签（去掉连接词/标点，保留业务语义）
fn normalize_label(clause: &str) -> String {
    let stop = ["然后", "接着", "再", "并", "最后", "首先", "先", "之后", "随后"];
    let mut s = clause.trim().to_string();
    for w in &stop {
        if let Some(pos) = s.find(w) {
            if pos == 0 {
                s = s[pos + w.len()..].trim().to_string();
            }
        }
    }
    // 截到合理长度
    if s.chars().count() > 16 {
        s = s.chars().take(16).collect();
    }
    if s.is_empty() {
        s = "处理".into();
    }
    s
}

fn detect_schedule(text: &str) -> Option<Schedule> {
    if contains_any(text, &["每天", "每日"]) {
        return Some(Schedule { cron: "0 0 * * *".into(), desc: "每日执行".into() });
    }
    if contains_any(text, &["每周", "每星期"]) {
        return Some(Schedule { cron: "0 0 * * 1".into(), desc: "每周一执行".into() });
    }
    if contains_any(text, &["每月"]) {
        return Some(Schedule { cron: "0 0 1 * *".into(), desc: "每月1日执行".into() });
    }
    if contains_any(text, &["每小时", "实时"]) {
        return Some(Schedule { cron: "0 * * * *".into(), desc: "每小时执行".into() });
    }
    if contains_any(text, &["定时", "周期", "调度", "cron", "Cron", "CRON"]) {
        return Some(Schedule { cron: "0 0 * * *".into(), desc: "周期调度".into() });
    }
    None
}

fn detect_policy(text: &str) -> DeliveryPolicy {
    if contains_any(text, &["紧急", "实时", "尽快", "立刻", "马上"]) {
        DeliveryPolicy::Urgent
    } else if contains_any(text, &["探索", "创新", "新业务", "研发", "尝试", "未知"]) {
        DeliveryPolicy::Exploratory
    } else {
        DeliveryPolicy::Balanced
    }
}

/// 抽取「关键词 X / Y / Z」式的并列列表
fn extract_list(text: &str, keywords: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for kw in keywords {
        if let Some(pos) = text.find(kw) {
            // 取关键词之后到下一个句号/换行之间的片段，再按 / 、， 拆分
            let rest = &text[pos + kw.len()..];
            let seg = rest
                .split(|c| "。；;\n".contains(c))
                .next()
                .unwrap_or("")
                .trim();
            for item in seg.split(|c| "、/,，".contains(c)) {
                let item = item.trim();
                if !item.is_empty() && item.len() <= 20 {
                    out.push(item.to_string());
                }
            }
        }
    }
    out
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| text.contains(k))
}

/// 由业务目标生成稳定的需求 ID（ascii，合法溯源主键）
fn slug(goal: &str) -> String {
    let mut s: String = goal
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if s.is_empty() {
        s = format!("req_{}", goal.chars().count());
    }
    // 中文目标无 ascii：用长度+哈希兜底
    if s.is_empty() {
        s = format!("req_{}", goal.len());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chinese_business_description() {
        let p = parse(
            "构建一个电商月度经营分析报告系统：每天抓取销售数据，清洗对账后生成图表报告。角色：运营、分析师。对接 PostgreSQL。",
        );
        assert!(!p.subtasks.is_empty(), "应解析出子任务");
        assert!(p.schedule.is_some(), "应识别每日调度");
        assert_eq!(p.schedule.as_ref().unwrap().cron, "0 0 * * *");
        assert!(p.roles.iter().any(|r| r.contains("运营")));
        assert!(p.external_systems.iter().any(|e| e.contains("PostgreSQL")));
        // 子任务类别覆盖抓取与报告
        let cats: Vec<Category> = p.subtasks.iter().map(|s| s.category).collect();
        assert!(cats.contains(&Category::Fetch));
        assert!(cats.contains(&Category::Llm));
    }

    #[test]
    fn urgent_policy_detected() {
        let p = parse("紧急实时风控：接入流数据，特征计算，模型推理，告警下发。");
        assert!(matches!(p.policy, DeliveryPolicy::Urgent));
        let cats: Vec<Category> = p.subtasks.iter().map(|s| s.category).collect();
        assert!(cats.contains(&Category::Shell), "应识别告警下发为 Shell");
    }

    #[test]
    fn exploratory_policy_detected() {
        let p = parse("探索性研发一个未知领域的智能客服工单聚类方案：抓取工单，文本向量化，聚类分析。");
        assert!(matches!(p.policy, DeliveryPolicy::Exploratory));
    }

    #[test]
    fn to_spec_feeds_pipeline() {
        let spec = parse("抓取销售数据，清洗对账，生成图表报告。").to_spec();
        assert!(!spec.subtasks.is_empty());
        // 同类意图复用同一 key，保证知识库命中
        let keys: Vec<&String> = spec.subtasks.iter().map(|s| &s.key).collect();
        assert!(keys.contains(&&"fetch".to_string()));
        assert!(keys.contains(&&"report".to_string()));
    }

    #[test]
    fn empty_falls_back_to_minimal_loop() {
        let spec = parse("帮我做点东西").to_spec();
        assert!(spec.subtasks.len() >= 2);
    }
}
