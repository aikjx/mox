// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 任务拆解器：将用户意图 + 提取到的实体，拆解为有序的执行步骤。
//!
//! ## 设计思路
//! - 基于**意图模板**的规则拆解：每种意图对应一套标准步骤序列
//! - 步骤带**依赖关系**（DAG），可并行的自动并行化
//! - 每步标注**所需能力 / 风险等级 / 确认策略**
//! - 实体注入到步骤参数中，生成可执行的任务计划
//!
//! ## 风险等级与确认策略
//! - `Low`：只读操作，免确认直接执行
//! - `Medium`：写操作但可撤回，执行前需用户确认
//! - `High`：对外发送 / 删除 / 权限变更，需二次确认

use ahash::RandomState;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::entity::Entity;

// ─── 核心类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// 只读，免确认
    Low,
    /// 可撤回的写操作，一次确认
    Medium,
    /// 对外发送/删除/权限变更，二次确认
    High,
}

impl RiskLevel {
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Low => "低风险",
            RiskLevel::Medium => "中风险",
            RiskLevel::High => "高风险",
        }
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, RiskLevel::Medium | RiskLevel::High)
    }

    pub fn requires_double_confirmation(&self) -> bool {
        matches!(self, RiskLevel::High)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 等待用户确认
    WaitingConfirmation,
    /// 已跳过
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// 步骤唯一 ID
    pub id: String,
    /// 步骤名称（用户可读）
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 所需能力 / Agent 类型
    pub capability: String,
    /// 风险等级
    pub risk: RiskLevel,
    /// 执行状态
    pub status: StepStatus,
    /// 依赖的前置步骤 ID 列表
    pub depends_on: Vec<String>,
    /// 注入的参数（从实体提取）
    pub params: HashMap<String, String, RandomState>,
    /// 预估耗时（秒）
    pub est_duration_sec: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// 计划 ID
    pub plan_id: String,
    /// 原始意图
    pub intent: String,
    /// 原始用户输入
    pub user_query: String,
    /// 步骤列表（按执行顺序排列）
    pub steps: Vec<TaskStep>,
    /// 是否需要用户整体确认（含高风险步骤时为 true）
    pub requires_overall_confirmation: bool,
    /// 可并行执行的步骤组（每组内的步骤可并行）
    pub parallel_groups: Vec<Vec<String>>,
    /// 总预估耗时（秒）
    pub total_est_duration_sec: u32,
}

// ─── 意图模板 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct StepTemplate {
    id_suffix: &'static str,
    name: &'static str,
    description: &'static str,
    capability: &'static str,
    risk: RiskLevel,
    depends_on: &'static [&'static str],
    est_duration_sec: u32,
    /// 从实体中提取的参数映射：(实体类型, 参数名)
    param_mappings: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone)]
struct IntentTemplate {
    intent: &'static str,
    steps: &'static [StepTemplate],
}

// ─── 任务拆解器 ──────────────────────────────────────────────────────────────

pub struct TaskDecomposer {
    /// 注册的意图模板
    templates: HashMap<String, IntentTemplate, RandomState>,
}

impl TaskDecomposer {
    pub fn new() -> Self {
        let mut templates: HashMap<String, IntentTemplate, RandomState> =
            HashMap::with_hasher(RandomState::new());

        // 注册内置模板
        for tpl in Self::builtin_templates() {
            templates.insert(tpl.intent.to_string(), tpl);
        }

        Self { templates }
    }

    /// 注册自定义意图模板
    pub fn register_template(&mut self, intent: &str, _steps: Vec<TaskStepTemplate>) {
        // P2 完善动态模板注册
        // 当前 P1 版本仅支持内置模板
        let _ = intent;
    }

    /// 拆解任务
    pub fn decompose(&self, intent: &str, entities: &[Entity], user_query: &str) -> TaskPlan {
        let template = self.templates.get(intent);

        let steps = if let Some(tpl) = template {
            self.build_steps_from_template(tpl, entities)
        } else {
            // 通用 fallback：单步对话式处理
            vec![TaskStep {
                id: "step-chat".into(),
                name: "AI 对话处理".into(),
                description: "AI 助手理解并回应用户需求".into(),
                capability: "chat".into(),
                risk: RiskLevel::Low,
                status: StepStatus::Pending,
                depends_on: vec![],
                params: HashMap::with_hasher(RandomState::new()),
                est_duration_sec: 5,
            }]
        };

        let requires_overall = steps.iter().any(|s| s.risk.requires_double_confirmation());
        let parallel_groups = Self::compute_parallel_groups(&steps);
        let total_est = steps.iter().map(|s| s.est_duration_sec).max().unwrap_or(0)
            + steps.iter().filter(|s| s.depends_on.is_empty()).map(|s| s.est_duration_sec).sum::<u32>();

        TaskPlan {
            plan_id: uuid::Uuid::now_v7().to_string(),
            intent: intent.into(),
            user_query: user_query.into(),
            steps,
            requires_overall_confirmation: requires_overall,
            parallel_groups,
            total_est_duration_sec: total_est,
        }
    }

    // ── 模板构建 ──────────────────────────────────────────────────────────

    fn build_steps_from_template(&self, tpl: &IntentTemplate, entities: &[Entity]) -> Vec<TaskStep> {
        // 先建立 id_suffix → step_index 映射
        let suffix_to_idx: HashMap<&str, usize, RandomState> = tpl.steps
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id_suffix, i))
            .collect();

        let mut steps = Vec::with_capacity(tpl.steps.len());
        for (i, stpl) in tpl.steps.iter().enumerate() {
            let step_id = format!("step-{}-{}", i, stpl.id_suffix);

            // 注入实体参数
            let mut params: HashMap<String, String, RandomState> = HashMap::with_hasher(RandomState::new());
            for (etype_str, param_name) in stpl.param_mappings {
                // 从实体中找匹配（支持 label 中文 和 Debug 变体两种匹配方式，不区分大小写）
                let etype_lower = etype_str.to_lowercase();
                if let Some(ent) = entities.iter().find(|e| {
                    e.etype.label() == *etype_str
                        || format!("{:?}", e.etype).to_lowercase() == etype_lower
                }) {
                    params.insert(
                        (*param_name).to_string(),
                        ent.normalized.clone().unwrap_or_else(|| ent.text.clone()),
                    );
                }
            }

            let depends_on = stpl.depends_on
                .iter()
                .filter_map(|suffix| {
                    suffix_to_idx.get(suffix).map(|&idx| format!("step-{}-{}", idx, suffix))
                })
                .collect();

            steps.push(TaskStep {
                id: step_id,
                name: stpl.name.to_string(),
                description: stpl.description.to_string(),
                capability: stpl.capability.to_string(),
                risk: stpl.risk,
                status: StepStatus::Pending,
                depends_on,
                params,
                est_duration_sec: stpl.est_duration_sec,
            });
        }
        steps
    }

    // ── 并行分组计算（拓扑分层） ───────────────────────────────────────────

    fn compute_parallel_groups(steps: &[TaskStep]) -> Vec<Vec<String>> {
        if steps.is_empty() { return vec![]; }

        let mut remaining: HashSet<String, RandomState> = steps.iter()
            .map(|s| s.id.clone()).collect();
        let mut groups = Vec::new();

        while !remaining.is_empty() {
            let mut current_group = Vec::new();
            for step in steps {
                if !remaining.contains(&step.id) { continue; }
                let deps_met = step.depends_on.iter().all(|d| !remaining.contains(d));
                if deps_met {
                    current_group.push(step.id.clone());
                }
            }
            if current_group.is_empty() {
                // 有环，兜底：全放一组
                current_group = remaining.iter().cloned().collect();
            }
            for id in &current_group {
                remaining.remove(id);
            }
            groups.push(current_group);
        }
        groups
    }

    // ── 内置模板 ──────────────────────────────────────────────────────────

    fn builtin_templates() -> Vec<IntentTemplate> {
        vec![
            // ===== 数据报告生成 =====
            IntentTemplate {
                intent: "report_generate",
                steps: &[
                    StepTemplate {
                        id_suffix: "data_fetch",
                        name: "获取数据",
                        description: "从数据源获取分析所需数据",
                        capability: "data.query",
                        risk: RiskLevel::Low,
                        depends_on: &[],
                        est_duration_sec: 10,
                        param_mappings: &[
                            ("TimeRange", "time_range"),
                            ("Dataset", "dataset"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "analyze",
                        name: "数据分析",
                        description: "对数据进行统计分析和可视化计算",
                        capability: "data.analyze",
                        risk: RiskLevel::Low,
                        depends_on: &["data_fetch"],
                        est_duration_sec: 15,
                        param_mappings: &[
                            ("OutputFormat", "output_format"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "generate",
                        name: "生成报告",
                        description: "生成指定格式的报告文件",
                        capability: "report.generate",
                        risk: RiskLevel::Low,
                        depends_on: &["analyze"],
                        est_duration_sec: 8,
                        param_mappings: &[
                            ("OutputFormat", "format"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "send",
                        name: "发送报告",
                        description: "将报告发送给指定收件人",
                        capability: "email.send",
                        risk: RiskLevel::High,
                        depends_on: &["generate"],
                        est_duration_sec: 3,
                        param_mappings: &[
                            ("Recipient", "recipient"),
                        ],
                    },
                ],
            },
            // ===== 知识图谱查询 =====
            IntentTemplate {
                intent: "graph_query",
                steps: &[
                    StepTemplate {
                        id_suffix: "parse",
                        name: "理解查询意图",
                        description: "将自然语言转换为图谱查询语句",
                        capability: "ai.nl2cypher",
                        risk: RiskLevel::Low,
                        depends_on: &[],
                        est_duration_sec: 5,
                        param_mappings: &[
                            ("Graph", "graph_name"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "execute",
                        name: "执行图谱查询",
                        description: "在指定图谱上执行查询",
                        capability: "kg.query",
                        risk: RiskLevel::Low,
                        depends_on: &["parse"],
                        est_duration_sec: 8,
                        param_mappings: &[
                            ("Graph", "graph_name"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "visualize",
                        name: "可视化结果",
                        description: "以图形化方式展示查询结果",
                        capability: "kg.visualize",
                        risk: RiskLevel::Low,
                        depends_on: &["execute"],
                        est_duration_sec: 3,
                        param_mappings: &[],
                    },
                ],
            },
            // ===== 项目创建 =====
            IntentTemplate {
                intent: "project_create",
                steps: &[
                    StepTemplate {
                        id_suffix: "plan",
                        name: "规划项目结构",
                        description: "根据需求规划项目的数据源、图谱模型、算子流程",
                        capability: "ai.plan",
                        risk: RiskLevel::Low,
                        depends_on: &[],
                        est_duration_sec: 10,
                        param_mappings: &[
                            ("Project", "project_name"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "confirm",
                        name: "用户确认方案",
                        description: "展示项目方案供用户确认",
                        capability: "ui.confirm",
                        risk: RiskLevel::Medium,
                        depends_on: &["plan"],
                        est_duration_sec: 0,
                        param_mappings: &[],
                    },
                    StepTemplate {
                        id_suffix: "create",
                        name: "创建项目",
                        description: "创建项目并初始化资源",
                        capability: "project.create",
                        risk: RiskLevel::Medium,
                        depends_on: &["confirm"],
                        est_duration_sec: 5,
                        param_mappings: &[
                            ("Project", "project_name"),
                        ],
                    },
                ],
            },
            // ===== 工作流执行 =====
            IntentTemplate {
                intent: "workflow_execute",
                steps: &[
                    StepTemplate {
                        id_suffix: "validate",
                        name: "校验工作流",
                        description: "检查工作流配置完整性和依赖",
                        capability: "flow.validate",
                        risk: RiskLevel::Low,
                        depends_on: &[],
                        est_duration_sec: 3,
                        param_mappings: &[],
                    },
                    StepTemplate {
                        id_suffix: "execute",
                        name: "执行工作流",
                        description: "按 DAG 顺序执行算子流程",
                        capability: "flow.execute",
                        risk: RiskLevel::Medium,
                        depends_on: &["validate"],
                        est_duration_sec: 30,
                        param_mappings: &[
                            ("TimeRange", "time_range"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "notify",
                        name: "完成通知",
                        description: "执行完成后通知用户",
                        capability: "notify.user",
                        risk: RiskLevel::Low,
                        depends_on: &["execute"],
                        est_duration_sec: 2,
                        param_mappings: &[],
                    },
                ],
            },
            // ===== 数据分析 =====
            IntentTemplate {
                intent: "data_analysis",
                steps: &[
                    StepTemplate {
                        id_suffix: "explore",
                        name: "数据探查",
                        description: "了解数据结构和质量",
                        capability: "data.profile",
                        risk: RiskLevel::Low,
                        depends_on: &[],
                        est_duration_sec: 8,
                        param_mappings: &[
                            ("Dataset", "dataset"),
                            ("TimeRange", "time_range"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "analyze",
                        name: "深度分析",
                        description: "按分析目标执行统计/对比/预测",
                        capability: "data.analyze",
                        risk: RiskLevel::Low,
                        depends_on: &["explore"],
                        est_duration_sec: 20,
                        param_mappings: &[
                            ("OutputFormat", "output_format"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "visualize",
                        name: "生成图表",
                        description: "生成可视化图表",
                        capability: "chart.generate",
                        risk: RiskLevel::Low,
                        depends_on: &["analyze"],
                        est_duration_sec: 5,
                        param_mappings: &[
                            ("OutputFormat", "chart_type"),
                        ],
                    },
                ],
            },
            // ===== Agent 安装 =====
            IntentTemplate {
                intent: "agent_install",
                steps: &[
                    StepTemplate {
                        id_suffix: "search",
                        name: "搜索 Agent",
                        description: "在 Agent 商场中搜索匹配的 Agent",
                        capability: "market.search",
                        risk: RiskLevel::Low,
                        depends_on: &[],
                        est_duration_sec: 3,
                        param_mappings: &[
                            ("Agent", "agent_name"),
                        ],
                    },
                    StepTemplate {
                        id_suffix: "review",
                        name: "展示详情",
                        description: "展示 Agent 详情和权限说明",
                        capability: "market.detail",
                        risk: RiskLevel::Low,
                        depends_on: &["search"],
                        est_duration_sec: 2,
                        param_mappings: &[],
                    },
                    StepTemplate {
                        id_suffix: "install",
                        name: "安装 Agent",
                        description: "安装并配置 Agent",
                        capability: "market.install",
                        risk: RiskLevel::Medium,
                        depends_on: &["review"],
                        est_duration_sec: 10,
                        param_mappings: &[
                            ("Agent", "agent_name"),
                        ],
                    },
                ],
            },
        ]
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self { Self::new() }
}

// ─── 辅助类型（动态模板注册用，P1 占位） ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TaskStepTemplate {
    pub id_suffix: String,
    pub name: String,
    pub description: String,
    pub capability: String,
    pub risk: RiskLevel,
    pub depends_on: Vec<String>,
    pub est_duration_sec: u32,
    pub param_mappings: Vec<(String, String)>,
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityType};

    #[test]
    fn decomposes_known_intent() {
        let dec = TaskDecomposer::new();
        let entities = vec![
            Entity {
                etype: EntityType::TimeRange,
                text: "上个月".into(),
                normalized: Some("last_month".into()),
                confidence: 0.9,
                start: 0, end: 3,
            },
            Entity {
                etype: EntityType::OutputFormat,
                text: "PPT".into(),
                normalized: Some("ppt".into()),
                confidence: 0.95,
                start: 5, end: 8,
            },
            Entity {
                etype: EntityType::Recipient,
                text: "销售总监".into(),
                normalized: Some("销售总监".into()),
                confidence: 0.6,
                start: 10, end: 14,
            },
        ];
        let plan = dec.decompose("report_generate", &entities, "上个月的销售报告做成PPT发给销售总监");
        assert!(!plan.steps.is_empty());
        assert!(plan.requires_overall_confirmation); // 含发送步骤（High）
        assert_eq!(plan.steps.last().unwrap().risk, RiskLevel::High);
    }

    #[test]
    fn unknown_intent_falls_to_chat() {
        let dec = TaskDecomposer::new();
        let plan = dec.decompose("some_unknown_intent", &[], "随便说点什么");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].capability, "chat");
        assert_eq!(plan.steps[0].risk, RiskLevel::Low);
    }

    #[test]
    fn parallel_groups_computed_correctly() {
        let dec = TaskDecomposer::new();
        let plan = dec.decompose("graph_query", &[], "查一下图谱");
        // 3 个串行步骤 → 3 个组
        assert_eq!(plan.parallel_groups.len(), 3);
    }

    #[test]
    fn risk_levels_work() {
        assert!(!RiskLevel::Low.requires_confirmation());
        assert!(RiskLevel::Medium.requires_confirmation());
        assert!(!RiskLevel::Medium.requires_double_confirmation());
        assert!(RiskLevel::High.requires_double_confirmation());
    }

    #[test]
    fn params_injected_from_entities() {
        let dec = TaskDecomposer::new();
        let entities = vec![
            Entity {
                etype: EntityType::Graph,
                text: "金融风控图谱".into(),
                normalized: Some("金融风控图谱".into()),
                confidence: 0.95,
                start: 0, end: 6,
            },
        ];
        let plan = dec.decompose("graph_query", &entities, "查询金融风控图谱");
        let parse_step = &plan.steps[0];
        assert!(parse_step.params.contains_key("graph_name"));
    }

    #[test]
    fn project_create_has_confirmation_step() {
        let dec = TaskDecomposer::new();
        let plan = dec.decompose("project_create", &[], "创建一个项目");
        assert!(plan.steps.iter().any(|s| s.capability == "ui.confirm"));
        assert!(plan.steps.iter().any(|s| s.risk == RiskLevel::Medium));
    }
}
