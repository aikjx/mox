use mox_platform_gateway_svc::alliance::{
    experts_collaboration::generate_expert_answer, experts_common::ExpertDescriptor,
};

#[tokio::test]
async fn missing_real_provider_never_returns_a_fabricated_answer() {
    // This process has no provider configuration; no fake network client is injected.
    assert!(mox_ai_expert_svc::llm::consultant::strict_llm_consultant_from_env().is_none());
    let expert = ExpertDescriptor::minimal("real-provider-required".into(), "Expert".into());
    let answer = generate_expert_answer(&expert, "Diagnose the actual system").await;
    assert_eq!(answer["status"], "failed");
    assert_eq!(answer["source"], "unavailable");
    assert_eq!(answer["solution"], "");
    assert!(answer["error"].is_string());
}
