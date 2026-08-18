//! AI智能对话引擎
//!
//! 实现自然语言理解、意图识别、算子推荐和多轮对话管理

// 预留公开 API / 未接入管线的能力面（如插件总线、算子目录、优化器 DAG、RBAC 之外的合规结构）：显式允许 dead_code 而非删除，避免破坏能力面；后续接入时自然消除。
#![allow(dead_code)]
use super::types::*;
use operator_core::Result;
use std::collections::HashMap;
use tracing;
use uuid::Uuid;

/// 对话引擎 - 智能交互核心
pub struct ConversationEngine {
    sessions: HashMap<String, ChatSession>,
    system_prompt: String,
    operator_knowledge: OperatorKnowledge,
}

/// 算子知识库 - 用于智能推荐
struct OperatorKnowledge {
    keywords: HashMap<String, Vec<String>>,
    category_keywords: HashMap<String, Vec<String>>,
}

impl ConversationEngine {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            system_prompt: Self::build_system_prompt(),
            operator_knowledge: Self::build_operator_knowledge(),
        }
    }

    fn build_system_prompt() -> String {
        r#"你是算子统一系统的AI助手，基于范畴论数学公理系统构建。
你拥有以下能力：
1. 算子编排与执行 - 组合恒等、线性、归一化、激活等算子构建工作流
2. 算法分析与归一化 - 将任意算法转化为标准算子流程图
3. 全资源管理 - CPU/内存/插件/算子资源统一调度
4. 插件互通 - WASM/内置/外部插件通过消息总线协作
5. 业务流程自动化 - BPMN风格工作流驱动AI智能体执行

回答应当专业、精确，并推荐合适的算子组合。"#.to_string()
    }

    fn build_operator_knowledge() -> OperatorKnowledge {
        let mut keywords = HashMap::new();
        let mut category_keywords = HashMap::new();

        // 核心算子关键词映射
        keywords.insert("identity".to_string(), vec!["恒等".to_string(), "直接".to_string(), "不变".to_string(), "passthrough".to_string()]);
        keywords.insert("linear".to_string(), vec!["线性".to_string(), "变换".to_string(), "缩放".to_string(), "矩阵".to_string(), "乘法".to_string()]);
        keywords.insert("normalize".to_string(), vec!["归一化".to_string(), "标准化".to_string(), "norm".to_string(), "单位向量".to_string()]);
        keywords.insert("relu".to_string(), vec!["relu".to_string(), "激活".to_string(), "整流".to_string(), "非线性".to_string()]);
        keywords.insert("sigmoid".to_string(), vec!["sigmoid".to_string(), "S型".to_string(), "概率".to_string(), "0到1".to_string()]);
        keywords.insert("tanh".to_string(), vec!["tanh".to_string(), "双曲正切".to_string(), "-1到1".to_string()]);
        keywords.insert("softmax".to_string(), vec!["softmax".to_string(), "指数归一化".to_string(), "分类".to_string(), "概率分布".to_string()]);
        keywords.insert("matmul".to_string(), vec!["矩阵乘法".to_string(), "matmul".to_string(), "线性变换".to_string()]);
        keywords.insert("conv2d".to_string(), vec!["卷积".to_string(), "conv".to_string(), "CNN".to_string(), "特征提取".to_string()]);
        keywords.insert("attention".to_string(), vec!["注意力".to_string(), "attention".to_string(), "transformer".to_string(), "自注意力".to_string()]);
        keywords.insert("adam".to_string(), vec!["adam".to_string(), "优化器".to_string(), "训练".to_string(), "梯度下降".to_string()]);

        // 分类关键词
        category_keywords.insert("core".to_string(), vec!["基础".to_string(), "核心".to_string()]);
        category_keywords.insert("activation".to_string(), vec!["激活".to_string(), "非线性".to_string()]);
        category_keywords.insert("math".to_string(), vec!["数学".to_string(), "计算".to_string()]);
        category_keywords.insert("ai".to_string(), vec!["AI".to_string(), "机器学习".to_string(), "深度学习".to_string(), "神经网络".to_string()]);
        category_keywords.insert("signal".to_string(), vec!["信号".to_string(), "图像处理".to_string()]);
        category_keywords.insert("optimizer".to_string(), vec!["优化".to_string(), "训练".to_string()]);

        OperatorKnowledge {
            keywords,
            category_keywords,
        }
    }

    /// 处理用户消息 - 主入口
    ///
    /// 注意：调用方负责把用户消息写入会话历史（两条调用路径
    /// `chat`/`chat_with_llm` 都会先 `add_user_message`）。本方法只负责
    /// 把生成的助手回复持久化进会话，从而实现真正的多轮对话记忆。
    pub async fn process_message(&mut self, session_id: &str, message: &str) -> Result<ChatResponse> {
        tracing::debug!("处理对话消息: session={}, msg={}", session_id, message);

        let intent = self.recognize_intent(message);
        let referenced_ops = self.extract_referenced_operators(message);
        let response_content = self.generate_response(message, &intent, &referenced_ops);
        let suggestions = self.generate_suggestions(&intent);
        let recommended_ops = self.recommend_operators(message, &intent);
        let actions = self.generate_actions(&intent);
        let workflow_suggestion = self.suggest_workflow(&intent, message);

        let msg = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: response_content,
            timestamp: chrono::Utc::now(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("intent".to_string(), serde_json::to_value(&intent).unwrap_or(serde_json::Value::Null));
                m
            },
            referenced_operators: referenced_ops.clone(),
        };

        // 持久化助手回复到会话历史，保证多轮对话上下文不丢失
        self.add_assistant_message(session_id, &msg.content);

        Ok(ChatResponse {
            message: msg,
            suggestions,
            recommended_operators: recommended_ops,
            actions,
            workflow_suggestion,
        })
    }

    /// 获取或创建会话
    pub fn get_or_create_session(&mut self, session_id: &str) -> &mut ChatSession {
        self.sessions.entry(session_id.to_string()).or_insert_with(|| {
            ChatSession {
                id: session_id.to_string(),
                messages: vec![ChatMessage::system(&self.system_prompt)],
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                context: SessionContext::default(),
            }
        })
    }

    /// 获取会话历史消息（用于LLM调用）
    pub fn get_session_history(&mut self, session_id: &str) -> Vec<ChatMessage> {
        let session = self.get_or_create_session(session_id);
        session.messages.clone()
    }

    /// 添加用户消息并返回
    pub fn add_user_message(&mut self, session_id: &str, content: &str) -> ChatMessage {
        let msg = ChatMessage::user(content);
        let session = self.get_or_create_session(session_id);
        session.messages.push(msg.clone());
        session.updated_at = chrono::Utc::now();
        msg
    }

    /// 添加助手消息并构建ChatResponse
    pub fn add_assistant_message(&mut self, session_id: &str, content: &str) -> ChatResponse {
        let msg = ChatMessage::assistant(content);
        let session = self.get_or_create_session(session_id);
        session.messages.push(msg.clone());
        session.updated_at = chrono::Utc::now();
        
        ChatResponse {
            message: msg,
            suggestions: vec![
                "继续提问".to_string(),
                "执行工作流".to_string(),
                "查看资源状态".to_string(),
            ],
            recommended_operators: vec![],
            actions: vec![],
            workflow_suggestion: None,
        }
    }

    /// 意图识别
    fn recognize_intent(&self, message: &str) -> UserIntent {
        let msg_lower = message.to_lowercase();

        if msg_lower.contains("状态") || msg_lower.contains("运行") || msg_lower.contains("status") {
            return UserIntent::QueryStatus;
        }
        if msg_lower.contains("算子") || msg_lower.contains("列表") || msg_lower.contains("operator") || msg_lower.contains("list") {
            return UserIntent::ListOperators;
        }
        if msg_lower.contains("执行") || msg_lower.contains("运行") || msg_lower.contains("execute") || msg_lower.contains("run") || msg_lower.contains("工作流") {
            let ops = self.extract_referenced_operators(message);
            if !ops.is_empty() {
                return UserIntent::ExecuteWorkflow { operators: ops };
            }
        }
        if msg_lower.contains("算法") || msg_lower.contains("分析") || msg_lower.contains("analyze") || msg_lower.contains("algorithm") || msg_lower.contains("流程图") {
            return UserIntent::AnalyzeAlgorithm { algo_type: "general".to_string() };
        }
        if msg_lower.contains("创建") && (msg_lower.contains("算子") || msg_lower.contains("operator")) {
            return UserIntent::CreateOperator;
        }
        if msg_lower.contains("资源") || msg_lower.contains("内存") || msg_lower.contains("cpu") || msg_lower.contains("resource") {
            return UserIntent::QueryResources;
        }
        if msg_lower.contains("插件") || msg_lower.contains("plugin") || msg_lower.contains("wasm") {
            return UserIntent::ManagePlugins;
        }
        if msg_lower.contains("图谱") || msg_lower.contains("图") || msg_lower.contains("graph") || msg_lower.contains("知识") {
            return UserIntent::ViewGraph;
        }
        if msg_lower.contains("推荐") || msg_lower.contains("建议") || msg_lower.contains("recommend") {
            return UserIntent::GetRecommendation;
        }
        if msg_lower.contains("流程") || msg_lower.contains("业务") || msg_lower.contains("workflow") || msg_lower.contains("编排") {
            return UserIntent::CreateWorkflow;
        }

        UserIntent::GeneralChat
    }

    /// 提取消息中提到的算子
    fn extract_referenced_operators(&self, message: &str) -> Vec<String> {
        let mut ops = Vec::new();
        let all_ops = [
            "identity", "linear", "normalize", "normalize_l1", "relu", "sigmoid", "tanh",
            "softmax", "scale", "add_bias", "matmul", "conv2d", "maxpool", "attention",
            "self_attention", "cross_attention", "feedforward", "embedding", "adam", "sgd"
        ];

        for op in &all_ops {
            if message.to_lowercase().contains(op) {
                ops.push(op.to_string());
            }
        }

        // 中文关键词匹配
        let cn_keywords: HashMap<&str, &str> = [
            ("恒等", "identity"), ("线性", "linear"), ("归一化", "normalize"),
            ("激活", "relu"), ("s型", "sigmoid"), ("双曲正切", "tanh"),
            ("softmax", "softmax"), ("卷积", "conv2d"), ("注意力", "attention"),
        ].iter().cloned().collect();

        for (cn, en) in &cn_keywords {
            if message.contains(cn) && !ops.contains(&en.to_string()) {
                ops.push(en.to_string());
            }
        }

        ops
    }

    /// 生成回复
    fn generate_response(&self, message: &str, intent: &UserIntent, referenced_ops: &[String]) -> String {
        match intent {
            UserIntent::QueryStatus => {
                "🚀 算子统一系统运行正常！\n\n系统状态：\n• 核心引擎：运行中\n• 知识图谱：已加载（34+算子节点，30+关系边）\n• 插件系统：就绪\n• AI智能体：活跃\n\n你可以：执行算子工作流、分析算法、管理资源、编排业务流程。".to_string()
            }
            UserIntent::ListOperators => {
                "📋 可用算子列表：\n\n【核心算子】\n• identity - 恒等算子\n• linear - 线性变换\n• normalize - L2归一化\n• normalize_l1 - L1归一化（概率分布）\n\n【激活函数】\n• relu - ReLU整流线性\n• sigmoid - S型激活\n• tanh - 双曲正切\n• softmax - 指数归一化\n\n【数学运算】\n• scale - 缩放算子\n• add_bias - 偏置加法\n• matmul - 矩阵乘法\n\n【AI算子】\n• conv2d - 2D卷积\n• attention - 注意力机制\n• adam - Adam优化器\n\n告诉我你想执行什么工作流，我会为你推荐算子组合！".to_string()
            }
            UserIntent::ExecuteWorkflow { operators } => {
                if operators.is_empty() {
                    "请告诉我你想执行的算子序列。例如：\n• \"linear -> relu -> normalize\" 执行前向传播\n• \"linear -> sigmoid -> softmax\" 执行分类\n\n或者描述你的需求，我来推荐算子组合！".to_string()
                } else {
                    format!("✅ 准备执行工作流: {} \n\n这是一个{}层的算子组合。正在为你准备执行...\n\n💡 提示：你可以提供输入数据向量来执行此工作流。",
                        operators.join(" → "),
                        operators.len()
                    )
                }
            }
            UserIntent::AnalyzeAlgorithm { .. } => {
                "🧠 算法分析与归一化系统就绪！\n\n我可以将任意算法转化为标准算子流程图：\n\n支持的算法类型：\n• 排序/搜索算法\n• 图算法（PageRank、最短路径、社区发现）\n• 机器学习流水线\n• 深度学习网络\n• 优化算法\n• 信号处理流程\n• 任意自定义算法\n\n请粘贴你的算法代码或描述算法流程，我会：\n1. 分析算法结构与复杂度\n2. 归一化为标准算子节点\n3. 生成可执行工作流\n4. 提供优化建议".to_string()
            }
            UserIntent::QueryResources => {
                "📊 资源全景监控：\n\n【计算资源】\n• CPU: 可用\n• 内存: 受系统监控（默认上限1GB）\n• GPU: 待检测\n\n【系统资源】\n• 算子池: 10+内置算子\n• 插件系统: WASM热加载就绪\n• 知识图谱: 活跃\n• 工作流引擎: 运行中\n\n使用 /api/ai/resources 可获取详细资源使用数据。".to_string()
            }
            UserIntent::ManagePlugins => {
                "🔌 插件互通总线就绪！\n\n插件系统支持：\n• WASM插件 - 沙箱执行，安全隔离\n• 内置算子 - 高性能原生执行\n• 外部服务 - HTTP/gRPC集成\n• AI模型 - 模型即插件\n• 数据源 - 统一数据接入\n\n插件间通过发布-订阅消息总线通信，支持：\n• 点对点消息\n• 主题广播\n• 请求-响应模式\n• 事件驱动协作\n\n将WASM文件放入 ./plugins 目录即可自动加载！".to_string()
            }
            UserIntent::ViewGraph => {
                "🕸️ 知识图谱系统：\n\n图谱包含：\n• 34+算子节点（核心/激活/数学/AI/图/优化器等分类）\n• 30+关系边（transforms/activation/composes/implements等语义关系）\n\n高级分析功能：\n• PageRank中心性计算\n• 社区发现算法\n• 激活传播（模拟信号流）\n• 智能推荐（基于上下文）\n• 最短路径查询\n\n访问 /api/graph 获取完整图谱数据！".to_string()
            }
            UserIntent::CreateWorkflow => {
                "🎯 业务流程驱动的AI智能体就绪！\n\n工作流引擎支持：\n\n【节点类型】\n• Start/End - 起止节点\n• Operator - 算子执行\n• Condition - 条件分支\n• Parallel - 并行分支\n• SubWorkflow - 子流程\n• UserTask - 人工任务\n• AiTask - AI任务\n• PluginCall - 插件调用\n• Delay - 延时等待\n\n【合并策略】\n• AllComplete - 全部完成\n• AnyComplete - 任一完成\n• FirstSuccess - 首个成功\n• VoteMajority - 多数投票\n\n你可以通过拖拽方式在前端可视化编排工作流，或描述业务流程让我为你生成！".to_string()
            }
            UserIntent::GetRecommendation => {
                "💡 基于当前上下文，我推荐：\n\n【经典工作流】\n1. 神经网络前向传播: linear → relu → linear → softmax\n2. 特征提取归一化: conv2d → relu → maxpool → normalize\n3. 注意力机制: embedding → positional → attention → feedforward\n\n【优化建议】\n• 大批量数据优先使用并行算子\n• 长时间运行流程考虑资源限制\n• 关键路径可通过知识图谱优化\n\n告诉我你的具体任务，我给出精准推荐！".to_string()
            }
            UserIntent::CreateOperator => {
                "⚙️ 创建自定义算子：\n\n你可以通过以下方式创建算子：\n1. WASM插件 - 编译为.wasm放入plugins目录\n2. 函数算子 - 通过FunctionOperator包装\n3. 线性算子 - 提供变换矩阵\n4. 组合算子 - 通过Workflow组合现有算子\n\n算子开发需遵循：\n• 范畴论态射规则（输入输出类型匹配）\n• 资源声明（CPU/内存预估）\n• 守恒律可选检查\n\n请描述你需要的算子功能！".to_string()
            }
            UserIntent::GeneralChat | UserIntent::Unknown => {
                if !referenced_ops.is_empty() {
                    format!("你提到了算子: {}。\n\n我可以帮你将这些算子组合成工作流执行。\n\n例如尝试：\n• \"执行 {} 工作流\"\n• \"分析包含{}的算法流程\"\n• \"推荐{}之后的算子\"",
                        referenced_ops.join(", "),
                        referenced_ops.join("→"),
                        referenced_ops.last().unwrap_or(&String::new()),
                        referenced_ops.first().unwrap_or(&String::new())
                    )
                } else {
                    let prefix = if message.contains("你好") || message.contains("hi") || message.contains("hello") {
                        "欢迎来到璇玑开发专家系统！\n\n".to_string()
                    } else {
                        format!("关于「{}」，", message)
                    };
                    format!("你好！我是算子统一系统的AI智能体 🤖\n\n{}我能帮你完成：\n\n1. 🧮 **算子编排执行** - 描述需求，我组合算子执行\n2. 📊 **算法归一化** - 将任意算法转化为标准流程图\n3. 📈 **资源管理** - 全维资源监控与调度\n4. 🔌 **插件互通** - 多插件通过消息总线协作\n5. 🎯 **流程自动化** - 业务流程驱动AI执行\n\n试试说：\n• \"列出所有算子\"\n• \"执行 linear → relu → normalize 工作流\"\n• \"分析排序算法\"\n• \"创建一个AI训练流程\"", prefix)
                }
            }
        }
    }

    /// 生成快捷建议
    fn generate_suggestions(&self, intent: &UserIntent) -> Vec<String> {
        match intent {
            UserIntent::GeneralChat | UserIntent::Unknown => {
                vec![
                    "列出所有算子".to_string(),
                    "执行神经网络前向传播".to_string(),
                    "分析快速排序算法".to_string(),
                    "查看系统资源状态".to_string(),
                    "创建AI训练工作流".to_string(),
                ]
            }
            UserIntent::ExecuteWorkflow { .. } => {
                vec![
                    "查看算子详情".to_string(),
                    "优化此工作流".to_string(),
                    "添加更多算子".to_string(),
                ]
            }
            UserIntent::AnalyzeAlgorithm { .. } => {
                vec![
                    "分析快速排序".to_string(),
                    "分析PageRank算法".to_string(),
                    "分析Transformer注意力".to_string(),
                    "分析梯度下降".to_string(),
                ]
            }
            _ => {
                vec![
                    "执行linear→relu→normalize".to_string(),
                    "查看知识图谱".to_string(),
                    "创建业务流程".to_string(),
                ]
            }
        }
    }

    /// 推荐算子
    fn recommend_operators(&self, message: &str, intent: &UserIntent) -> Vec<String> {
        let mut recommended = Vec::new();
        let msg_lower = message.to_lowercase();

        // 基于意图推荐
        match intent {
            UserIntent::ExecuteWorkflow { operators } => {
                recommended.extend(operators.clone());
                // 补充后续算子
                if let Some(last) = operators.last() {
                    let successors: HashMap<&str, Vec<&str>> = [
                        ("linear", vec!["relu", "sigmoid", "normalize"]),
                        ("relu", vec!["normalize", "dropout", "linear"]),
                        ("conv2d", vec!["relu", "maxpool", "batchnorm"]),
                        ("normalize", vec!["softmax", "linear"]),
                    ].iter().cloned().collect();
                    if let Some(succ) = successors.get(last.as_str()) {
                        for s in succ {
                            if !recommended.contains(&s.to_string()) {
                                recommended.push(s.to_string());
                            }
                        }
                    }
                }
            }
            UserIntent::AnalyzeAlgorithm { .. } => {
                recommended = vec!["identity".to_string(), "linear".to_string(), "normalize".to_string(), "relu".to_string()];
            }
            _ => {
                // 基于关键词推荐
                if msg_lower.contains("神经") || msg_lower.contains("network") || msg_lower.contains("深度学习") {
                    recommended = vec!["linear".to_string(), "relu".to_string(), "softmax".to_string(), "adam".to_string()];
                } else if msg_lower.contains("分类") || msg_lower.contains("classif") {
                    recommended = vec!["linear".to_string(), "sigmoid".to_string(), "softmax".to_string()];
                } else if msg_lower.contains("图像") || msg_lower.contains("image") || msg_lower.contains("卷积") {
                    recommended = vec!["conv2d".to_string(), "relu".to_string(), "maxpool".to_string(), "normalize".to_string()];
                } else if msg_lower.contains("注意力") || msg_lower.contains("attention") || msg_lower.contains("transformer") {
                    recommended = vec!["embedding".to_string(), "attention".to_string(), "feedforward".to_string(), "normalize".to_string()];
                }
            }
        }

        recommended.into_iter().take(8).collect()
    }

    /// 生成建议动作
    fn generate_actions(&self, intent: &UserIntent) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();

        match intent {
            UserIntent::ExecuteWorkflow { operators } if !operators.is_empty() => {
                actions.push(SuggestedAction {
                    id: "exec-workflow".to_string(),
                    label: "▶️ 执行此工作流".to_string(),
                    action_type: ActionType::ExecuteWorkflow,
                    payload: serde_json::json!({ "operators": operators }),
                });
            }
            UserIntent::ListOperators | UserIntent::GeneralChat => {
                actions.push(SuggestedAction {
                    id: "view-graph".to_string(),
                    label: "🕸️ 查看知识图谱".to_string(),
                    action_type: ActionType::ShowGraph,
                    payload: serde_json::json!({}),
                });
                actions.push(SuggestedAction {
                    id: "show-resources".to_string(),
                    label: "📊 资源监控".to_string(),
                    action_type: ActionType::ShowResources,
                    payload: serde_json::json!({}),
                });
            }
            UserIntent::AnalyzeAlgorithm { .. } => {
                actions.push(SuggestedAction {
                    id: "analyze-algo".to_string(),
                    label: "🔬 开始算法分析".to_string(),
                    action_type: ActionType::AnalyzeAlgorithm,
                    payload: serde_json::json!({}),
                });
            }
            UserIntent::CreateWorkflow => {
                actions.push(SuggestedAction {
                    id: "create-wf".to_string(),
                    label: "🎯 创建业务流程".to_string(),
                    action_type: ActionType::CreateWorkflow,
                    payload: serde_json::json!({}),
                });
            }
            _ => {}
        }

        actions
    }

    /// 建议工作流
    fn suggest_workflow(&self, intent: &UserIntent, message: &str) -> Option<Vec<String>> {
        let msg_lower = message.to_lowercase();

        if msg_lower.contains("前向") || msg_lower.contains("forward") || msg_lower.contains("神经网络") {
            return Some(vec!["linear".to_string(), "relu".to_string(), "linear".to_string(), "softmax".to_string()]);
        }
        if msg_lower.contains("卷积") || msg_lower.contains("cnn") || msg_lower.contains("图像") {
            return Some(vec!["conv2d".to_string(), "relu".to_string(), "maxpool".to_string(), "normalize".to_string()]);
        }
        if msg_lower.contains("transformer") || msg_lower.contains("注意力") {
            return Some(vec!["embedding".to_string(), "attention".to_string(), "feedforward".to_string(), "normalize".to_string()]);
        }
        if msg_lower.contains("归一化") || msg_lower.contains("概率") {
            return Some(vec!["linear".to_string(), "normalize_l1".to_string()]);
        }

        match intent {
            UserIntent::ExecuteWorkflow { operators } if !operators.is_empty() => Some(operators.clone()),
            _ => None,
        }
    }
}

impl Default for ConversationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_or_create_session_creates_and_persists() {
        let mut engine = ConversationEngine::new();
        {
            let s = engine.get_or_create_session("s1");
            assert_eq!(s.id, "s1");
            // 首条为 system prompt
            assert_eq!(s.messages.len(), 1);
            assert_eq!(s.messages[0].role, MessageRole::System);
        }
        // 再次获取同一 session 不应新建
        let s = engine.get_or_create_session("s1");
        assert_eq!(s.messages.len(), 1);
    }

    #[test]
    fn test_add_user_and_assistant_messages() {
        let mut engine = ConversationEngine::new();
        let u = engine.add_user_message("sess", "你好");
        assert_eq!(u.role, MessageRole::User);
        assert_eq!(u.content, "你好");

        let a = engine.add_assistant_message("sess", "世界");
        assert_eq!(a.message.role, MessageRole::Assistant);
        assert_eq!(a.message.content, "世界");

        let s = engine.get_or_create_session("sess");
        // system + user + assistant
        assert_eq!(s.messages.len(), 3);
    }

    #[tokio::test]
    async fn test_process_message_list_operators_intent() {
        let mut engine = ConversationEngine::new();
        let resp = engine.process_message("列出所有算子", "sess-1").await.unwrap();
        assert!(!resp.message.content.is_empty());
        assert!(!resp.suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_process_message_execute_intent_with_operators() {
        let mut engine = ConversationEngine::new();
        let resp = engine.process_message("sess-2", "执行 linear relu normalize 工作流").await.unwrap();
        // 应识别到执行意图，响应提到工作流且含推荐算子
        assert!(resp.message.content.contains("工作流") || resp.message.content.contains("linear"));
        assert!(resp.actions.iter().any(|a| matches!(a.action_type, ActionType::ExecuteWorkflow)));
        assert!(resp.recommended_operators.iter().any(|o| o == "linear" || o == "relu" || o == "normalize"));
    }

    #[tokio::test]
    async fn test_process_message_analyze_algorithm_intent() {
        let mut engine = ConversationEngine::new();
        let resp = engine.process_message("sess-3", "分析快速排序算法").await.unwrap();
        assert!(resp.message.content.contains("算法") || resp.message.content.contains("归一化"));
        assert!(resp.actions.iter().any(|a| matches!(a.action_type, ActionType::AnalyzeAlgorithm)));
    }

    #[tokio::test]
    async fn test_process_message_vision_recommends_conv2d() {
        let mut engine = ConversationEngine::new();
        let resp = engine.process_message("sess-4", "处理图像卷积任务").await.unwrap();
        // recommend_operators 应基于中文关键词「卷积」提取 conv2d
        assert!(resp.recommended_operators.contains(&"conv2d".to_string()));
    }
}
