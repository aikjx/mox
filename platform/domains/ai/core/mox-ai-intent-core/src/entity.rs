// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 实体提取器：从自然语言中抽取结构化实体。
//!
//! ## 支持的实体类型
//! - **时间实体**：日期（今天/明天/上周/7月/2026年）、时间段（本周/上月/Q3）
//! - **数字实体**：整数、小数、百分比、货币
//! - **参数实体**：输出格式（PPT/Excel/图表）、收件人、目标对象
//! - **领域实体**：项目名、图谱名、数据集名、Agent 名（基于注册词典）
//!
//! ## 设计原则
//! - 纯规则 + 词典匹配，零外部依赖，毫秒级响应
//! - 提取结果带置信度和原文位置，方便上层校验
//! - 领域实体可运行时注册，支持热更新

use ahash::RandomState;
use hashbrown::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ─── 实体类型定义 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// 时间点（今天/明天/7月1日）
    TimePoint,
    /// 时间段（本周/上月/2026年Q3）
    TimeRange,
    /// 数字（123 / 3.14）
    Number,
    /// 百分比（80% / 百分之五十）
    Percentage,
    /// 货币（100元 / $50）
    Currency,
    /// 输出格式（PPT / Excel / 图表 / 报告）
    OutputFormat,
    /// 收件人 / 目标人（销售总监 / 张三）
    Recipient,
    /// 项目名
    Project,
    /// 图谱名
    Graph,
    /// 数据集名
    Dataset,
    /// Agent / 算子名
    Agent,
    /// 文件格式 / 扩展名
    FileFormat,
    /// 通用对象（兜底）
    Object,
}

impl EntityType {
    pub fn label(&self) -> &'static str {
        match self {
            EntityType::TimePoint => "时间点",
            EntityType::TimeRange => "时间段",
            EntityType::Number => "数字",
            EntityType::Percentage => "百分比",
            EntityType::Currency => "货币",
            EntityType::OutputFormat => "输出格式",
            EntityType::Recipient => "收件人",
            EntityType::Project => "项目",
            EntityType::Graph => "图谱",
            EntityType::Dataset => "数据集",
            EntityType::Agent => "Agent",
            EntityType::FileFormat => "文件格式",
            EntityType::Object => "对象",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// 实体类型
    pub etype: EntityType,
    /// 抽取到的文本
    pub text: String,
    /// 规范化值（如时间点规范化为 ISO 日期字符串）
    pub normalized: Option<String>,
    /// 置信度 0..1
    pub confidence: f32,
    /// 在原文中的起始字节位置
    pub start: usize,
    /// 在原文中的结束字节位置（exclusive）
    pub end: usize,
}

// ─── 实体提取器 ──────────────────────────────────────────────────────────────

pub struct EntityExtractor {
    /// 时间正则集
    time_patterns: Vec<(Regex, EntityType, f32)>,
    /// 数字正则集
    number_patterns: Vec<(Regex, EntityType, f32)>,
    /// 输出格式关键词词典
    output_formats: HashMap<String, (String, f32), RandomState>,
    /// 收件人触发词 + 抽取模式
    recipient_triggers: Vec<String>,
    /// 领域实体词典：type -> Vec<(name, alias_vec)>
    domain_entities: HashMap<EntityType, Vec<DomainEntity>, RandomState>,
    /// Aho-Corasick 用于领域实体匹配（动态构建）
    ac_built: bool,
}

#[derive(Debug, Clone)]
struct DomainEntity {
    name: String,
    aliases: Vec<String>,
}

impl EntityExtractor {
    pub fn new() -> Self {
        let time_patterns = Self::build_time_patterns();
        let number_patterns = Self::build_number_patterns();
        let output_formats = Self::build_output_formats();
        let recipient_triggers = vec![
            "发给".into(), "发送给".into(), "抄送".into(), "给".into(),
            "收件人".into(), "接收人".into(), "通知".into(), "告知".into(),
        ];

        Self {
            time_patterns,
            number_patterns,
            output_formats,
            recipient_triggers,
            domain_entities: HashMap::with_hasher(RandomState::new()),
            ac_built: false,
        }
    }

    // ── 注册领域实体 ──────────────────────────────────────────────────────

    pub fn register_domain_entity(&mut self, etype: EntityType, name: &str, aliases: &[&str]) {
        let entry = DomainEntity {
            name: name.to_string(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
        };
        self.domain_entities.entry(etype).or_default().push(entry);
        self.ac_built = false;
    }

    pub fn register_project(&mut self, name: &str, aliases: &[&str]) {
        self.register_domain_entity(EntityType::Project, name, aliases);
    }

    pub fn register_graph(&mut self, name: &str, aliases: &[&str]) {
        self.register_domain_entity(EntityType::Graph, name, aliases);
    }

    pub fn register_dataset(&mut self, name: &str, aliases: &[&str]) {
        self.register_domain_entity(EntityType::Dataset, name, aliases);
    }

    pub fn register_agent(&mut self, name: &str, aliases: &[&str]) {
        self.register_domain_entity(EntityType::Agent, name, aliases);
    }

    // ── 主提取接口 ────────────────────────────────────────────────────────

    pub fn extract(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        // 1. 时间实体
        self.extract_time(text, &mut entities);

        // 2. 数字 / 百分比 / 货币
        self.extract_numbers(text, &mut entities);

        // 3. 输出格式
        self.extract_output_format(text, &mut entities);

        // 4. 收件人
        self.extract_recipient(text, &mut entities);

        // 5. 领域实体（基于词典）
        self.extract_domain(text, &mut entities);

        // 按位置排序，去重（同位置取置信度高的）
        entities.sort_by(|a, b| {
            a.start.cmp(&b.start)
                .then(b.end.cmp(&a.end))
                .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });
        Self::dedupe_overlapping(&mut entities);

        entities
    }

    // ── 分类型提取 ────────────────────────────────────────────────────────

    fn extract_time(&self, text: &str, out: &mut Vec<Entity>) {
        for (re, etype, conf) in &self.time_patterns {
            for m in re.find_iter(text) {
                let matched = m.as_str().to_string();
                let normalized = Self::normalize_time(&matched, *etype);
                out.push(Entity {
                    etype: *etype,
                    text: matched,
                    normalized,
                    confidence: *conf,
                    start: m.start(),
                    end: m.end(),
                });
            }
        }
    }

    fn extract_numbers(&self, text: &str, out: &mut Vec<Entity>) {
        for (re, etype, conf) in &self.number_patterns {
            for m in re.find_iter(text) {
                let matched = m.as_str().to_string();
                let normalized = Self::normalize_number(&matched, *etype);
                out.push(Entity {
                    etype: *etype,
                    text: matched,
                    normalized,
                    confidence: *conf,
                    start: m.start(),
                    end: m.end(),
                });
            }
        }
    }

    fn extract_output_format(&self, text: &str, out: &mut Vec<Entity>) {
        let lower = text.to_lowercase();
        for (kw, (norm, conf)) in &self.output_formats {
            let kw_lower = kw.to_lowercase();
            let mut start = 0;
            while let Some(pos) = lower[start..].find(&kw_lower) {
                let abs_start = start + pos;
                let abs_end = abs_start + kw.len();
                out.push(Entity {
                    etype: EntityType::OutputFormat,
                    text: text[abs_start..abs_end].to_string(),
                    normalized: Some(norm.clone()),
                    confidence: *conf,
                    start: abs_start,
                    end: abs_end,
                });
                start = abs_end;
            }
        }
    }

    fn extract_recipient(&self, text: &str, out: &mut Vec<Entity>) {
        // 简单模式：触发词 + 后面 2-6 个字符或直到标点/空格
        for trigger in &self.recipient_triggers {
            let mut start = 0;
            while let Some(pos) = text[start..].find(trigger) {
                let trigger_end = start + pos + trigger.len();
                // 提取 trigger 后到标点或空格前的内容
                let rest = &text[trigger_end..];
                let end_offset = rest
                    .find(|c: char| c == '，' || c == '。' || c == '、' || c == '；' || c == ' ' || c == '\n' || c == ',')
                    .unwrap_or(rest.len().min(12));
                if end_offset > 0 {
                    let name = rest[..end_offset].trim().to_string();
                    if !name.is_empty() && name.len() <= 12 {
                        out.push(Entity {
                            etype: EntityType::Recipient,
                            text: name.clone(),
                            normalized: Some(name),
                            confidence: 0.6,
                            start: trigger_end,
                            end: trigger_end + end_offset,
                        });
                    }
                }
                start = trigger_end;
            }
        }
    }

    fn extract_domain(&self, text: &str, out: &mut Vec<Entity>) {
        // 简单子串匹配（P1 阶段，后续可用 Aho-Corasick 优化）
        for (etype, entries) in &self.domain_entities {
            for entry in entries {
                // 匹配主名
                if let Some(pos) = text.find(&entry.name) {
                    out.push(Entity {
                        etype: *etype,
                        text: entry.name.clone(),
                        normalized: Some(entry.name.clone()),
                        confidence: 0.95,
                        start: pos,
                        end: pos + entry.name.len(),
                    });
                }
                // 匹配别名
                for alias in &entry.aliases {
                    if let Some(pos) = text.find(alias) {
                        out.push(Entity {
                            etype: *etype,
                            text: alias.clone(),
                            normalized: Some(entry.name.clone()),
                            confidence: 0.8,
                            start: pos,
                            end: pos + alias.len(),
                        });
                    }
                }
            }
        }
    }

    // ── 去重：重叠时保留置信度高的 ────────────────────────────────────────

    fn dedupe_overlapping(entities: &mut Vec<Entity>) {
        if entities.is_empty() { return; }
        let mut result = Vec::with_capacity(entities.len());
        let mut last_end = 0;
        for e in entities.drain(..) {
            if e.start >= last_end {
                last_end = e.end;
                result.push(e);
            } else if e.confidence > result.last().map(|x| x.confidence).unwrap_or(0.0)
                && e.end - e.start >= result.last().map(|x| x.end - x.start).unwrap_or(0)
            {
                // 重叠但置信度更高且长度不短，替换
                if let Some(last) = result.last_mut() {
                    *last = e;
                    last_end = last.end;
                }
            }
        }
        *entities = result;
    }

    // ── 正则构建 ──────────────────────────────────────────────────────────

    fn build_time_patterns() -> Vec<(Regex, EntityType, f32)> {
        use EntityType::*;
        vec![
            // 相对日期（高置信度）
            (Regex::new(r"今天|今日|当天").unwrap(), TimePoint, 0.95),
            (Regex::new(r"明天|明日").unwrap(), TimePoint, 0.95),
            (Regex::new(r"昨天|昨日").unwrap(), TimePoint, 0.95),
            (Regex::new(r"后天").unwrap(), TimePoint, 0.9),
            (Regex::new(r"前天").unwrap(), TimePoint, 0.9),
            // 相对周
            (Regex::new(r"本周|这周|这一周").unwrap(), TimeRange, 0.9),
            (Regex::new(r"上周|上一周").unwrap(), TimeRange, 0.9),
            (Regex::new(r"下周|下一周").unwrap(), TimeRange, 0.9),
            // 相对月
            (Regex::new(r"本月|这个月|当月").unwrap(), TimeRange, 0.9),
            (Regex::new(r"上月|上个月").unwrap(), TimeRange, 0.9),
            (Regex::new(r"下月|下个月").unwrap(), TimeRange, 0.85),
            // 相对季度
            (Regex::new(r"本季度|这一季度").unwrap(), TimeRange, 0.85),
            (Regex::new(r"上季度|上一季度").unwrap(), TimeRange, 0.85),
            (Regex::new(r"Q[1-4]|第[一二三四1-4]季度").unwrap(), TimeRange, 0.8),
            // 年
            (Regex::new(r"今年|本年度").unwrap(), TimeRange, 0.9),
            (Regex::new(r"去年|上一年|上年度").unwrap(), TimeRange, 0.9),
            (Regex::new(r"明年|下一年").unwrap(), TimeRange, 0.85),
            // 具体年份
            (Regex::new(r"20\d{2}年?").unwrap(), TimeRange, 0.9),
            // 具体月份
            (Regex::new(r"([1-9]|1[0-2])月").unwrap(), TimeRange, 0.85),
            // 具体日期 X月X日
            (Regex::new(r"([1-9]|1[0-2])月([1-9]|[12]\d|3[01])日?").unwrap(), TimePoint, 0.9),
            // 星期
            (Regex::new(r"周[一二三四五六日天]|星期[一二三四五六日天]").unwrap(), TimePoint, 0.9),
            // 年初/年末/月底
            (Regex::new(r"年初|年底|年末|月初|月底|月末").unwrap(), TimePoint, 0.7),
        ]
    }

    fn build_number_patterns() -> Vec<(Regex, EntityType, f32)> {
        use EntityType::*;
        vec![
            // 百分比
            (Regex::new(r"\d+(\.\d+)?%").unwrap(), Percentage, 0.95),
            (Regex::new(r"百分之[一二三四五六七八九十百千万\d]+").unwrap(), Percentage, 0.9),
            // 货币
            (Regex::new(r"[¥￥$€£]\d+(\.\d+)?").unwrap(), Currency, 0.95),
            (Regex::new(r"\d+(\.\d+)?[元块美元欧元英镑]").unwrap(), Currency, 0.9),
            // 小数
            (Regex::new(r"\d+\.\d+").unwrap(), Number, 0.85),
            // 整数（带单位）
            (Regex::new(r"\d+[万亿千百万kK]?").unwrap(), Number, 0.7),
        ]
    }

    fn build_output_formats() -> HashMap<String, (String, f32), RandomState> {
        let mut m: HashMap<String, (String, f32), RandomState> = HashMap::with_hasher(RandomState::new());
        // PPT / 演示文稿
        m.insert("PPT".into(), ("ppt".into(), 0.95));
        m.insert("ppt".into(), ("ppt".into(), 0.95));
        m.insert("演示文稿".into(), ("ppt".into(), 0.9));
        m.insert("幻灯片".into(), ("ppt".into(), 0.9));
        // Excel / 表格
        m.insert("Excel".into(), ("excel".into(), 0.95));
        m.insert("excel".into(), ("excel".into(), 0.95));
        m.insert("表格".into(), ("excel".into(), 0.8));
        m.insert("电子表格".into(), ("excel".into(), 0.85));
        // Word / 文档
        m.insert("Word".into(), ("word".into(), 0.95));
        m.insert("word".into(), ("word".into(), 0.95));
        m.insert("文档".into(), ("word".into(), 0.75));
        m.insert("报告".into(), ("report".into(), 0.8));
        // PDF
        m.insert("PDF".into(), ("pdf".into(), 0.95));
        m.insert("pdf".into(), ("pdf".into(), 0.95));
        // 图表
        m.insert("图表".into(), ("chart".into(), 0.85));
        m.insert("折线图".into(), ("line_chart".into(), 0.95));
        m.insert("柱状图".into(), ("bar_chart".into(), 0.95));
        m.insert("饼图".into(), ("pie_chart".into(), 0.95));
        m.insert("散点图".into(), ("scatter_chart".into(), 0.95));
        m.insert("雷达图".into(), ("radar_chart".into(), 0.95));
        // 邮件
        m.insert("邮件".into(), ("email".into(), 0.85));
        m.insert("Email".into(), ("email".into(), 0.9));
        m.insert("email".into(), ("email".into(), 0.9));
        m
    }

    // ── 规范化辅助 ────────────────────────────────────────────────────────

    fn normalize_time(text: &str, etype: EntityType) -> Option<String> {
        // P1 基础规范化：相对时间映射为语义标签
        // P2 可接入 chrono 做绝对日期计算
        let t = text.trim();
        let result = match etype {
            EntityType::TimePoint => match t {
                "今天" | "今日" | "当天" => "today".into(),
                "明天" | "明日" => "tomorrow".into(),
                "昨天" | "昨日" => "yesterday".into(),
                "后天" => "day_after_tomorrow".into(),
                "前天" => "day_before_yesterday".into(),
                _ if t.contains("周") || t.contains("星期") => format!("weekday:{}", t),
                _ if t.contains("月") && t.contains("日") => format!("date:{}", t),
                _ => t.to_string(),
            },
            EntityType::TimeRange => match t {
                "本周" | "这周" | "这一周" => "this_week".into(),
                "上周" | "上一周" => "last_week".into(),
                "下周" | "下一周" => "next_week".into(),
                "本月" | "这个月" | "当月" => "this_month".into(),
                "上月" | "上个月" => "last_month".into(),
                "下月" | "下个月" => "next_month".into(),
                "今年" | "本年度" => "this_year".into(),
                "去年" | "上一年" | "上年度" => "last_year".into(),
                "明年" | "下一年" => "next_year".into(),
                _ if t.contains("季度") || t.starts_with('Q') => format!("quarter:{}", t),
                _ if t.contains("年") => format!("year:{}", t),
                _ if t.contains("月") => format!("month:{}", t),
                _ => t.to_string(),
            },
            _ => t.to_string(),
        };
        Some(result)
    }

    fn normalize_number(text: &str, etype: EntityType) -> Option<String> {
        match etype {
            EntityType::Percentage => {
                // 去掉 % 或 "百分之"，保留数值
                let cleaned = text.trim_end_matches('%').trim_start_matches("百分之");
                Some(format!("pct:{}", cleaned))
            }
            EntityType::Currency => {
                Some(format!("currency:{}", text))
            }
            EntityType::Number => {
                Some(format!("num:{}", text))
            }
            _ => None,
        }
    }
}

impl Default for EntityExtractor {
    fn default() -> Self { Self::new() }
}

// ─── 便捷函数 ────────────────────────────────────────────────────────────────

pub fn extract_entities(text: &str) -> Vec<Entity> {
    let ext = EntityExtractor::new();
    ext.extract(text)
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_time_points() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("明天帮我生成报告");
        let time_ents: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::TimePoint).collect();
        assert!(!time_ents.is_empty());
        assert_eq!(time_ents[0].text, "明天");
    }

    #[test]
    fn extracts_time_ranges() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("上个月的销售数据");
        let range_ents: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::TimeRange).collect();
        assert!(!range_ents.is_empty());
        assert_eq!(range_ents[0].normalized.as_deref(), Some("last_month"));
    }

    #[test]
    fn extracts_percentages() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("增长了85.5%");
        let pct: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::Percentage).collect();
        assert!(!pct.is_empty());
        assert_eq!(pct[0].text, "85.5%");
    }

    #[test]
    fn extracts_currency() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("预算是¥10000");
        let cur: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::Currency).collect();
        assert!(!cur.is_empty());
    }

    #[test]
    fn extracts_output_format() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("输出 PPT 和折线图");
        let fmts: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::OutputFormat).collect();
        assert!(fmts.len() >= 2);
        assert!(fmts.iter().any(|e| e.normalized.as_deref() == Some("ppt")));
        assert!(fmts.iter().any(|e| e.normalized.as_deref() == Some("line_chart")));
    }

    #[test]
    fn extracts_recipient() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("发给销售总监");
        let recs: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::Recipient).collect();
        assert!(!recs.is_empty());
    }

    #[test]
    fn extracts_domain_entities() {
        let mut ext = EntityExtractor::new();
        ext.register_project("金融风控知识图谱", &["风控项目", "风控KG"]);
        let ents = ext.extract("帮我看看风控项目的数据");
        let projs: Vec<_> = ents.iter().filter(|e| e.etype == EntityType::Project).collect();
        assert!(!projs.is_empty());
        assert_eq!(projs[0].normalized.as_deref(), Some("金融风控知识图谱"));
    }

    #[test]
    fn empty_input_returns_empty() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("");
        assert!(ents.is_empty());
    }

    #[test]
    fn multiple_entities_sorted_by_position() {
        let ext = EntityExtractor::new();
        let ents = ext.extract("昨天增长了80%，今天要做PPT");
        assert!(!ents.is_empty());
        for w in ents.windows(2) {
            assert!(w[0].start <= w[1].start);
        }
    }
}
