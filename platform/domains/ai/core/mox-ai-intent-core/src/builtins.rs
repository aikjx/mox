// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 平台内置意图注册表：8 大 domain 的意图模式 + 任务模板映射。
//!
//! ## 8 大 Domain
//! - `data` — 数据域：数据探查 / 分析 / 报表 / ETL
//! - `kg` — 图谱域：图谱查询 / 构建 / 算法 / 融合
//! - `ai` — AI 域：对话 / 推理 / Agent / 生成
//! - `flow` — 流程域：工作流 / 算子 / 编排 / 执行
//! - `cloud` — 云资源域：存储 / 计算 / 卷 / 部署
//! - `voice` — 语音域：语音识别 / 合成 / 语音指令
//! - `market` — 商场域：模板 / Agent / 应用 / 安装
//! - `platform` — 平台域：项目 / 用户 / 权限 / 系统
//!
//! ## 设计原则
//! - 单一真源：所有 intent pattern 统一定义在此，上层直接引用
//! - 与 classifier 的 IntentPattern 对齐，可直接用于 Aho-Corasick 分类
//! - 每个意图绑定对应 task_decomp 模板名，管道直接查表拆解

use crate::classifier::IntentPattern;
use serde::{Deserialize, Serialize};

/// 意图定义：pattern + 元信息 + 任务模板映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDefinition {
    /// 意图 ID（唯一）
    pub id: String,
    /// 所属 domain
    pub domain: String,
    /// 意图名称（用户可读）
    pub name: String,
    /// 意图描述
    pub description: String,
    /// 分类模式（关键词）
    pub pattern: IntentPattern,
    /// 对应的任务拆解模板 ID
    pub task_template: String,
    /// 默认风险等级
    pub default_risk: String,
    /// 图标 emoji
    pub icon: String,
}

// ─── 注册表 ──────────────────────────────────────────────────────────────────

pub struct IntentRegistry {
    definitions: Vec<IntentDefinition>,
}

impl IntentRegistry {
    /// 获取所有内置意图定义
    pub fn all() -> Vec<IntentDefinition> {
        let mut all = Vec::new();
        all.extend(Self::data_intents());
        all.extend(Self::kg_intents());
        all.extend(Self::ai_intents());
        all.extend(Self::flow_intents());
        all.extend(Self::cloud_intents());
        all.extend(Self::voice_intents());
        all.extend(Self::market_intents());
        all.extend(Self::platform_intents());
        all
    }

    /// 获取所有 IntentPattern（用于构造分类器）
    pub fn all_patterns() -> Vec<IntentPattern> {
        Self::all().into_iter().map(|d| d.pattern).collect()
    }

    /// 按 domain 筛选
    pub fn by_domain(domain: &str) -> Vec<IntentDefinition> {
        Self::all().into_iter().filter(|d| d.domain == domain).collect()
    }

    /// 按意图 ID 查找
    pub fn find_by_id(id: &str) -> Option<IntentDefinition> {
        Self::all().into_iter().find(|d| d.id == id)
    }

    // ── data 域 ──────────────────────────────────────────────────────────

    fn data_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "data.analysis".into(),
                domain: "data".into(),
                name: "数据分析".into(),
                description: "对数据进行统计分析、对比分析、趋势分析".into(),
                pattern: IntentPattern {
                    intent: "data_analysis".into(),
                    keywords: vec![
                        "分析".into(), "数据分析".into(), "统计".into(), "对比".into(),
                        "趋势".into(), "占比".into(), "环比".into(), "同比".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "data_analysis".into(),
                default_risk: "low".into(),
                icon: "📊".into(),
            },
            IntentDefinition {
                id: "data.report".into(),
                domain: "data".into(),
                name: "生成报告".into(),
                description: "生成数据报告并可选发送".into(),
                pattern: IntentPattern {
                    intent: "report_generate".into(),
                    keywords: vec![
                        "报告".into(), "生成报告".into(), "做报告".into(), "周报".into(),
                        "月报".into(), "日报".into(), "总结".into(), "汇总".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "report_generate".into(),
                default_risk: "medium".into(),
                icon: "📑".into(),
            },
            IntentDefinition {
                id: "data.query".into(),
                domain: "data".into(),
                name: "数据查询".into(),
                description: "查询、筛选、检索数据".into(),
                pattern: IntentPattern {
                    intent: "data_query".into(),
                    keywords: vec![
                        "查询".into(), "查一下".into(), "搜索".into(), "筛选".into(),
                        "找".into(), "查询数据".into(), "看一下数据".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "data_query".into(),
                default_risk: "low".into(),
                icon: "🔍".into(),
            },
            IntentDefinition {
                id: "data.etl".into(),
                domain: "data".into(),
                name: "数据处理".into(),
                description: "数据清洗、转换、导入、导出".into(),
                pattern: IntentPattern {
                    intent: "data_etl".into(),
                    keywords: vec![
                        "清洗".into(), "数据清洗".into(), "导入".into(), "导出".into(),
                        "ETL".into(), "数据转换".into(), "同步".into(), "数据同步".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "data_etl".into(),
                default_risk: "medium".into(),
                icon: "🔄".into(),
            },
        ]
    }

    // ── kg 域 ────────────────────────────────────────────────────────────

    fn kg_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "kg.query".into(),
                domain: "kg".into(),
                name: "图谱查询".into(),
                description: "查询知识图谱中的节点、关系、路径".into(),
                pattern: IntentPattern {
                    intent: "graph_query".into(),
                    keywords: vec![
                        "图谱".into(), "知识图谱".into(), "图查询".into(), "查图谱".into(),
                        "路径".into(), "关联".into(), "关系".into(), "节点".into(),
                        "实体".into(), "图谱分析".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "graph_query".into(),
                default_risk: "low".into(),
                icon: "🕸️".into(),
            },
            IntentDefinition {
                id: "kg.build".into(),
                domain: "kg".into(),
                name: "图谱构建".into(),
                description: "从数据构建知识图谱".into(),
                pattern: IntentPattern {
                    intent: "graph_build".into(),
                    keywords: vec![
                        "建图谱".into(), "构建图谱".into(), "创建图谱".into(), "图谱建模".into(),
                        "本体".into(), "schema".into(), "Schema".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "graph_build".into(),
                default_risk: "medium".into(),
                icon: "🏗️".into(),
            },
            IntentDefinition {
                id: "kg.algo".into(),
                domain: "kg".into(),
                name: "图谱算法".into(),
                description: "运行图算法：社区发现、中心性、最短路径等".into(),
                pattern: IntentPattern {
                    intent: "graph_algo".into(),
                    keywords: vec![
                        "图算法".into(), "图谱算法".into(), "社区发现".into(), "中心性".into(),
                        "最短路径".into(), "PageRank".into(), "pagerank".into(), "聚类".into(),
                        "图计算".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "graph_algo".into(),
                default_risk: "low".into(),
                icon: "⚡".into(),
            },
            IntentDefinition {
                id: "kg.fusion".into(),
                domain: "kg".into(),
                name: "图谱融合".into(),
                description: "多源图谱融合、实体对齐、关系融合".into(),
                pattern: IntentPattern {
                    intent: "graph_fusion".into(),
                    keywords: vec![
                        "融合".into(), "图谱融合".into(), "实体对齐".into(), "合并图谱".into(),
                        "知识融合".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "graph_fusion".into(),
                default_risk: "medium".into(),
                icon: "🔗".into(),
            },
        ]
    }

    // ── ai 域 ────────────────────────────────────────────────────────────

    fn ai_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "ai.chat".into(),
                domain: "ai".into(),
                name: "AI 对话".into(),
                description: "通用 AI 对话、问答、闲聊".into(),
                pattern: IntentPattern {
                    intent: "ai_chat".into(),
                    keywords: vec![
                        "你好".into(), "hi".into(), "hello".into(), "聊天".into(),
                        "问问".into(), "请教".into(), "帮我".into(),
                    ],
                    capability: Some("chat".into()),
                },
                task_template: "chat".into(),
                default_risk: "low".into(),
                icon: "💬".into(),
            },
            IntentDefinition {
                id: "ai.reasoning".into(),
                domain: "ai".into(),
                name: "深度推理".into(),
                description: "复杂逻辑推理、因果分析、问题诊断".into(),
                pattern: IntentPattern {
                    intent: "reasoning".into(),
                    keywords: vec![
                        "为什么".into(), "原因".into(), "推理".into(), "分析原因".into(),
                        "诊断".into(), "根因".into(), "因果".into(),
                    ],
                    capability: Some("reasoning".into()),
                },
                task_template: "reasoning".into(),
                default_risk: "low".into(),
                icon: "🧠".into(),
            },
            IntentDefinition {
                id: "ai.agent".into(),
                domain: "ai".into(),
                name: "Agent 调度".into(),
                description: "调用 AI Agent 完成复杂任务".into(),
                pattern: IntentPattern {
                    intent: "agent_invoke".into(),
                    keywords: vec![
                        "Agent".into(), "agent".into(), "智能体".into(), "助手".into(),
                        "帮我做".into(), "自动".into(), "自动化".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "agent_invoke".into(),
                default_risk: "medium".into(),
                icon: "🤖".into(),
            },
            IntentDefinition {
                id: "ai.generate".into(),
                domain: "ai".into(),
                name: "内容生成".into(),
                description: "生成文本、代码、图像等内容".into(),
                pattern: IntentPattern {
                    intent: "content_generate".into(),
                    keywords: vec![
                        "生成".into(), "写".into(), "创作".into(), "画".into(),
                        "写代码".into(), "生成代码".into(), "写文章".into(), "生成报告".into(),
                    ],
                    capability: Some("reasoning".into()),
                },
                task_template: "content_generate".into(),
                default_risk: "low".into(),
                icon: "✨".into(),
            },
        ]
    }

    // ── flow 域 ──────────────────────────────────────────────────────────

    fn flow_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "flow.execute".into(),
                domain: "flow".into(),
                name: "执行工作流".into(),
                description: "执行已有的算子工作流 / DAG".into(),
                pattern: IntentPattern {
                    intent: "workflow_execute".into(),
                    keywords: vec![
                        "工作流".into(), "执行".into(), "运行".into(), "跑一下".into(),
                        "执行流程".into(), "运行工作流".into(), "算子".into(),
                    ],
                    capability: Some("workflow".into()),
                },
                task_template: "workflow_execute".into(),
                default_risk: "medium".into(),
                icon: "▶️".into(),
            },
            IntentDefinition {
                id: "flow.create".into(),
                domain: "flow".into(),
                name: "创建工作流".into(),
                description: "新建 / 编排一个算子工作流".into(),
                pattern: IntentPattern {
                    intent: "workflow_create".into(),
                    keywords: vec![
                        "创建工作流".into(), "新建工作流".into(), "编排".into(), "流程编排".into(),
                        "设计流程".into(), "搭个流程".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "workflow_create".into(),
                default_risk: "medium".into(),
                icon: "🧩".into(),
            },
            IntentDefinition {
                id: "flow.optimize".into(),
                domain: "flow".into(),
                name: "优化工作流".into(),
                description: "优化工作流性能、成本、准确性".into(),
                pattern: IntentPattern {
                    intent: "workflow_optimize".into(),
                    keywords: vec![
                        "优化".into(), "性能优化".into(), "提速".into(), "调优".into(),
                        "工作流优化".into(), "流程优化".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "workflow_optimize".into(),
                default_risk: "medium".into(),
                icon: "🚀".into(),
            },
        ]
    }

    // ── cloud 域 ─────────────────────────────────────────────────────────

    fn cloud_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "cloud.storage".into(),
                domain: "cloud".into(),
                name: "云存储管理".into(),
                description: "管理云存储、卷、文件".into(),
                pattern: IntentPattern {
                    intent: "cloud_storage".into(),
                    keywords: vec![
                        "存储".into(), "云存储".into(), "卷".into(), "volume".into(),
                        "文件".into(), "上传".into(), "下载".into(), "s3".into(), "S3".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "cloud_storage".into(),
                default_risk: "low".into(),
                icon: "💾".into(),
            },
            IntentDefinition {
                id: "cloud.compute".into(),
                domain: "cloud".into(),
                name: "计算资源".into(),
                description: "管理计算资源、弹性伸缩".into(),
                pattern: IntentPattern {
                    intent: "cloud_compute".into(),
                    keywords: vec![
                        "计算".into(), "算力".into(), "扩容".into(), "缩容".into(),
                        "弹性".into(), "资源".into(), "实例".into(), "节点".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "cloud_compute".into(),
                default_risk: "medium".into(),
                icon: "⚙️".into(),
            },
            IntentDefinition {
                id: "cloud.deploy".into(),
                domain: "cloud".into(),
                name: "应用部署".into(),
                description: "部署应用、发布版本".into(),
                pattern: IntentPattern {
                    intent: "cloud_deploy".into(),
                    keywords: vec![
                        "部署".into(), "发布".into(), "上线".into(), "发版".into(),
                        "应用部署".into(), "服务部署".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "cloud_deploy".into(),
                default_risk: "high".into(),
                icon: "🚢".into(),
            },
        ]
    }

    // ── voice 域 ─────────────────────────────────────────────────────────

    fn voice_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "voice.asr".into(),
                domain: "voice".into(),
                name: "语音识别".into(),
                description: "将语音转为文字".into(),
                pattern: IntentPattern {
                    intent: "voice_asr".into(),
                    keywords: vec![
                        "语音识别".into(), "转文字".into(), "语音转文字".into(), "ASR".into(),
                        "asr".into(), "识别语音".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "voice_asr".into(),
                default_risk: "low".into(),
                icon: "🎤".into(),
            },
            IntentDefinition {
                id: "voice.tts".into(),
                domain: "voice".into(),
                name: "语音合成".into(),
                description: "将文字转为语音".into(),
                pattern: IntentPattern {
                    intent: "voice_tts".into(),
                    keywords: vec![
                        "语音合成".into(), "转语音".into(), "文字转语音".into(), "TTS".into(),
                        "tts".into(), "朗读".into(), "读出来".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "voice_tts".into(),
                default_risk: "low".into(),
                icon: "🔊".into(),
            },
            IntentDefinition {
                id: "voice.command".into(),
                domain: "voice".into(),
                name: "语音指令".into(),
                description: "通过语音控制平台操作".into(),
                pattern: IntentPattern {
                    intent: "voice_command".into(),
                    keywords: vec![
                        "语音控制".into(), "语音指令".into(), "声控".into(),
                    ],
                    capability: Some("workflow".into()),
                },
                task_template: "voice_command".into(),
                default_risk: "medium".into(),
                icon: "🎙️".into(),
            },
        ]
    }

    // ── market 域 ────────────────────────────────────────────────────────

    fn market_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "market.search".into(),
                domain: "market".into(),
                name: "搜索应用".into(),
                description: "在应用商场中搜索模板 / Agent / 应用".into(),
                pattern: IntentPattern {
                    intent: "market_search".into(),
                    keywords: vec![
                        "商场".into(), "商店".into(), "应用市场".into(), "搜索应用".into(),
                        "找应用".into(), "模板".into(), "插件".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "market_search".into(),
                default_risk: "low".into(),
                icon: "🛒".into(),
            },
            IntentDefinition {
                id: "market.install".into(),
                domain: "market".into(),
                name: "安装应用".into(),
                description: "从商场安装模板 / Agent / 应用".into(),
                pattern: IntentPattern {
                    intent: "agent_install".into(),
                    keywords: vec![
                        "安装".into(), "安装应用".into(), "安装Agent".into(), "下载".into(),
                        "添加应用".into(), "装一个".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "agent_install".into(),
                default_risk: "medium".into(),
                icon: "📦".into(),
            },
            IntentDefinition {
                id: "market.publish".into(),
                domain: "market".into(),
                name: "发布到商场".into(),
                description: "将应用 / 模板发布到商场".into(),
                pattern: IntentPattern {
                    intent: "market_publish".into(),
                    keywords: vec![
                        "发布".into(), "上架".into(), "发布到商场".into(), "提交".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "market_publish".into(),
                default_risk: "medium".into(),
                icon: "📤".into(),
            },
        ]
    }

    // ── platform 域 ──────────────────────────────────────────────────────

    fn platform_intents() -> Vec<IntentDefinition> {
        vec![
            IntentDefinition {
                id: "platform.project_create".into(),
                domain: "platform".into(),
                name: "创建项目".into(),
                description: "创建一个新的项目".into(),
                pattern: IntentPattern {
                    intent: "project_create".into(),
                    keywords: vec![
                        "创建项目".into(), "新建项目".into(), "建个项目".into(), "新项目".into(),
                        "创建".into(),
                    ],
                    capability: Some("expert".into()),
                },
                task_template: "project_create".into(),
                default_risk: "medium".into(),
                icon: "📁".into(),
            },
            IntentDefinition {
                id: "platform.project_manage".into(),
                domain: "platform".into(),
                name: "项目管理".into(),
                description: "管理项目设置、成员、权限".into(),
                pattern: IntentPattern {
                    intent: "project_manage".into(),
                    keywords: vec![
                        "项目设置".into(), "项目管理".into(), "管理项目".into(), "项目成员".into(),
                        "项目权限".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "project_manage".into(),
                default_risk: "medium".into(),
                icon: "⚙️".into(),
            },
            IntentDefinition {
                id: "platform.user".into(),
                domain: "platform".into(),
                name: "用户管理".into(),
                description: "管理用户、角色、权限".into(),
                pattern: IntentPattern {
                    intent: "user_manage".into(),
                    keywords: vec![
                        "用户".into(), "用户管理".into(), "角色".into(), "权限".into(),
                        "成员管理".into(), "加人".into(), "踢人".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "user_manage".into(),
                default_risk: "high".into(),
                icon: "👥".into(),
            },
            IntentDefinition {
                id: "platform.system".into(),
                domain: "platform".into(),
                name: "系统设置".into(),
                description: "系统配置、监控、运维".into(),
                pattern: IntentPattern {
                    intent: "system_config".into(),
                    keywords: vec![
                        "系统设置".into(), "系统配置".into(), "监控".into(), "运维".into(),
                        "设置".into(), "系统".into(),
                    ],
                    capability: Some("graph".into()),
                },
                task_template: "system_config".into(),
                default_risk: "high".into(),
                icon: "🔧".into(),
            },
            IntentDefinition {
                id: "platform.help".into(),
                domain: "platform".into(),
                name: "帮助 / 导航".into(),
                description: "获取帮助、查找功能、引导".into(),
                pattern: IntentPattern {
                    intent: "help".into(),
                    keywords: vec![
                        "帮助".into(), "怎么".into(), "如何".into(), "在哪".into(),
                        "找不到".into(), "使用说明".into(), "教程".into(), "引导".into(),
                    ],
                    capability: Some("chat".into()),
                },
                task_template: "help".into(),
                default_risk: "low".into(),
                icon: "❓".into(),
            },
        ]
    }
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_intents_load() {
        let all = IntentRegistry::all();
        assert!(all.len() >= 25, "至少应有 25+ 内置意图，实际: {}", all.len());
    }

    #[test]
    fn each_domain_has_intents() {
        let domains = ["data", "kg", "ai", "flow", "cloud", "voice", "market", "platform"];
        for d in domains {
            let intents = IntentRegistry::by_domain(d);
            assert!(!intents.is_empty(), "domain {} 应至少有 1 个意图", d);
        }
    }

    #[test]
    fn intent_ids_are_unique() {
        use std::collections::HashSet;
        let all = IntentRegistry::all();
        let mut ids = HashSet::new();
        for def in &all {
            assert!(ids.insert(def.id.clone()), "重复意图 ID: {}", def.id);
        }
    }

    #[test]
    fn patterns_are_compatible_with_classifier() {
        let patterns = IntentRegistry::all_patterns();
        assert!(!patterns.is_empty());
        // 每个 pattern 至少有 1 个关键词
        for p in &patterns {
            assert!(!p.keywords.is_empty(), "intent {} 无关键词", p.intent);
        }
    }

    #[test]
    fn find_by_id_works() {
        let def = IntentRegistry::find_by_id("data.analysis");
        assert!(def.is_some());
        assert_eq!(def.unwrap().domain, "data");
    }

    #[test]
    fn find_by_id_returns_none_for_unknown() {
        assert!(IntentRegistry::find_by_id("nonexistent").is_none());
    }

    #[test]
    fn all_have_task_template() {
        for def in IntentRegistry::all() {
            assert!(!def.task_template.is_empty(), "intent {} 缺少 task_template", def.id);
        }
    }

    #[test]
    fn all_have_icon() {
        for def in IntentRegistry::all() {
            assert!(!def.icon.is_empty(), "intent {} 缺少 icon", def.id);
        }
    }
}
