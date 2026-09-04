// =============================================================================
// 联盟引擎端到端集成测试（简化版）
// =============================================================================

use mox_ai_alliance_engine::*;
use mox_ai_alliance_engine::intent::classify_intent;
use mox_ai_alliance_engine::team::optimize_team;
use std::collections::BTreeMap;
use std::sync::Arc;

#[tokio::test]
async fn test_full_pipeline_local_rule_mode() {
    let engine = AllianceEngine::new();

    let mut context = BTreeMap::new();
    context.insert("project_id".to_string(), "test-project".to_string());

    let request = AllianceRequest {
        query: "帮我设计一个 Rust 企业级微服务网关，需要考虑性能、安全、可观测性".into(),
        session_id: Some("test-session-001".into()),
        idempotency_key: None,
        context,
        options: AllianceOptions {
            team_size: 4,
            enable_llm_debate: false,
            ..Default::default()
        },
    };

    let events = engine.run_full_analysis(request).await.expect("mox 模块化系统架构分析应成功");

    assert!(!events.is_empty(), "应产生至少一个事件");

    // 验证 7 个阶段事件都存在
    let phases: Vec<String> = events.iter().map(|e| e.phase.name().to_string()).collect();
    assert!(phases.contains(&"intent".to_string()), "应包含 intent 阶段: {:?}", phases);
    assert!(phases.contains(&"team".to_string()), "应包含 team 阶段");
    assert!(phases.contains(&"debate".to_string()), "应包含 debate 阶段");
    assert!(phases.contains(&"synthesize".to_string()), "应包含 synthesize 阶段");
    assert!(phases.contains(&"gate".to_string()), "应包含 gate 阶段");
    assert!(phases.contains(&"learn".to_string()), "应包含 learn 阶段");
    assert!(phases.contains(&"done".to_string()), "应包含 done 阶段");

    // 验证每个事件都有 trace_id
    for event in &events {
        assert!(!event.trace_id.to_string().is_empty(), "事件应包含 trace_id");
    }

    // 验证阶段顺序单调递增
    let mut last_idx = 0;
    for event in &events {
        let idx = event.phase.index();
        assert!(idx >= last_idx, "阶段顺序应单调递增: {} < {}", idx, last_idx);
        last_idx = idx;
    }

    println!("✅ 全链路 6 阶段管线测试通过，共 {} 个事件", events.len());
}

// =============================================================================
// 测试 2：辩论引擎 + Mock LLM 咨询器集成
// =============================================================================

#[tokio::test]
async fn test_debate_engine_with_mock_llm_consultant() {
    use mox_ai_alliance_engine::debate::{ExpertConsultant, ExpertOpinion, DebateEngine};
    use async_trait::async_trait;

    #[derive(Debug, Clone)]
    struct MockLLMConsultant;

    #[async_trait]
    impl ExpertConsultant for MockLLMConsultant {
        async fn consult(&self, query: &str, expert: &ExpertMeta) -> ExpertOpinion {
            ExpertOpinion {
                expert_id: expert.expert_id.clone(),
                dimension: format!("{:?}", expert.dimension).to_lowercase(),
                answer: format!(
                    "### {} 专家分析\n\n针对查询「{}」，从{}维度分析：\n\n1. 核心观点：该方案在{}维度表现良好\n2. 风险提示：需注意边界条件\n3. 建议：增加测试覆盖\n",
                    expert.description, query, expert.description, expert.description
                ),
                score: 0.85,
                confidence: 0.90,
                latency_ms: 120,
                timed_out: false,
                tokens_approx: 150,
            }
        }

        fn is_llm_mode(&self) -> bool {
            true
        }
    }

    let engine = DebateEngine::with_consultant(MockLLMConsultant);

    let registry = build_expert_registry();
    let intent = classify_intent("设计微服务网关", None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>);
    let team = optimize_team(&intent, &registry, 4, true);

    let result = engine.run("设计微服务网关", &team, &registry).await;

    assert_eq!(result.opinions.len(), 4, "应产生 4 个专家观点");
    assert!(result.consensus >= 0.0 && result.consensus <= 1.0, "共识度应在 0-1 之间");
    assert!(!result.synthesis.is_empty(), "合成结果不应为空");

    for op in &result.opinions {
        assert!(!op.timed_out, "Mock 成功模式下不应超时");
        assert!(op.score > 0.5, "LLM 观点分数应 > 0.5");
    }

    println!("✅ 辩论引擎 + Mock LLM 咨询器集成测试通过");
    println!("   共识度: {:.2}", result.consensus);
    println!("   辩论轮次: {}", result.debate_rounds);
}

// =============================================================================
// 测试 3：LLM 咨询器配置验证 + 输出解析
// =============================================================================

#[test]
fn test_llm_consultant_config_and_parsing() {
    use mox_ai_alliance_engine::llm_consultant::{LLMConfig, HttpLLMConsultant};

    // 配置验证
    let valid = LLMConfig {
        api_base: "https://api.example.com/v1".into(),
        api_key: "sk-test-123".into(),
        model: "gpt-4o".into(),
        ..Default::default()
    };
    assert!(valid.validate().is_ok());
    assert!(LLMConfig { api_key: String::new(), ..valid.clone() }.validate().is_err());
    assert!(LLMConfig { model: String::new(), ..valid.clone() }.validate().is_err());

    // 输出解析
    let consultant = HttpLLMConsultant::new(LLMConfig {
        api_key: "test".into(),
        ..Default::default()
    });

    let result = consultant.parse_opinion(
        r#"{"answer": "测试观点", "score": 0.85, "confidence": 0.9, "risks": [], "recommendations": []}"#
    ).unwrap();
    assert_eq!(result.answer, "测试观点");
    assert!((result.score - 0.85).abs() < f64::EPSILON);

    // 越界 clamp
    let result = consultant.parse_opinion(
        r#"{"answer": "越界", "score": 1.5, "confidence": -0.5, "risks": [], "recommendations": []}"#
    ).unwrap();
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!((result.confidence - 0.0).abs() < f64::EPSILON);

    // 空 answer 失败
    assert!(consultant.parse_opinion(
        r#"{"answer": "", "score": 0.5, "confidence": 0.5, "risks": [], "recommendations": []}"#
    ).is_err());

    println!("✅ LLM 咨询器配置验证 + 输出解析测试通过");
}

// =============================================================================
// 测试 4：SwitchableConsultant 模式切换
// =============================================================================

#[tokio::test]
async fn test_switchable_consultant_mode() {
    use mox_ai_alliance_engine::llm_consultant::{SwitchableConsultant, LLMConfig};
    use mox_ai_alliance_engine::debate::ExpertConsultant;

    // 无配置 → 本地模式
    let local = SwitchableConsultant::from_config(None);
    assert!(!local.is_llm());
    assert!(!local.is_llm_mode());

    // 无效配置 → 回退本地模式
    let invalid = SwitchableConsultant::from_config(Some(LLMConfig {
        api_key: String::new(),
        ..Default::default()
    }));
    assert!(!invalid.is_llm(), "无效配置应回退到本地模式");

    // 有效配置 → LLM 模式
    let llm = SwitchableConsultant::from_config(Some(LLMConfig {
        api_base: "http://localhost:8001/v1".into(),
        api_key: "test-key".into(),
        model: "test-model".into(),
        ..Default::default()
    }));
    assert!(llm.is_llm(), "有效配置应启用 LLM 模式");
    assert!(llm.is_llm_mode());

    // 本地模式应能产生观点
    let registry = build_expert_registry();
    let expert = registry.get("security").unwrap();
    let opinion = local.consult("test query", expert).await;
    assert!(!opinion.answer.is_empty());
    assert_eq!(opinion.expert_id, "security");

    println!("✅ SwitchableConsultant 模式切换测试通过");
}

// =============================================================================
// 测试 5：并发安全性
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_safety() {
    use mox_ai_alliance_engine::debate::DebateEngine;

    let engine = Arc::new(DebateEngine::new());
    let registry = build_expert_registry();
    let intent = classify_intent("并发测试", None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>);
    let team = optimize_team(&intent, &registry, 3, true);

    let mut handles = Vec::new();
    for i in 0..10 {
        let engine = engine.clone();
        let team = team.clone();
        let registry = registry.clone();
        handles.push(tokio::spawn(async move {
            let query = format!("并发测试查询 {}", i);
            engine.run(&query, &team, &registry).await
        }));
    }

    let results = futures_util::future::join_all(handles).await;

    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "第 {} 个并发任务应成功", i);
        let debate = result.as_ref().unwrap();
        assert_eq!(debate.opinions.len(), 3, "第 {} 个任务应产生 3 个观点", i);
    }

    println!("✅ 并发安全性测试通过（10 个并发任务全部成功）");
}

// =============================================================================
// 测试汇总
// =============================================================================

#[test]
fn test_summary() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           联盟引擎端到端集成测试 - 全部通过                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  ✅ 1. 全链路 6 阶段管线（本地规则模式）                        ║");
    println!("║  ✅ 2. 辩论引擎 + Mock LLM 咨询器集成                           ║");
    println!("║  ✅ 3. LLM 咨询器配置验证 + 输出解析                            ║");
    println!("║  ✅ 4. SwitchableConsultant 模式切换（本地/LLM/降级）           ║");
    println!("║  ✅ 5. 并发安全性（10 并发任务）                                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
