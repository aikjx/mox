use mox_platform_gateway_svc::alliance::experts_orchestration::{execute_plan, generate_plan};

#[tokio::test]
async fn execution_without_real_experts_never_completes() {
    let mut plan = generate_plan("Actual analysis", "analysis", &[], "weighted");
    let result = execute_plan(&mut plan, None, &[]).await;
    assert_eq!(result["overall_status"], "failed");
    assert_ne!(plan.status, "completed");
    assert!(plan.steps.iter().all(|step| step.status != "completed"));
}
