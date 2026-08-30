// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 快速启动示例
//!
//! 演示统一平台的核心使用方式：
//! 1. 平台启动与健康检查
//! 2. AI 驱动的业务申请编排
//! 3. 事件驱动的跨系统联动
//! 4. 配置中心分层配置
//! 5. 各子系统直接访问

use mox_flow_unified_platform::*;

fn main() {
    println!("=== 璇玑 RelGraph · 全维归一化统一平台 快速启动 ===\n");

    // ========== 1. 启动平台 ==========
    println!("【1】启动统一平台...");
    let platform = PlatformFacade::new();
    platform.bootstrap().expect("平台启动失败");
    assert!(platform.is_ready());
    println!("  ✓ 平台启动成功\n");

    // ========== 2. 健康检查 ==========
    println!("【2】平台健康状态：");
    let health = platform.health();
    println!("  整体状态: {:?}", health.overall_status);
    println!("  健康模块: {}/{}", health.healthy_count, health.total_count);
    println!("  健康得分: {:.1}%", health.health_score() * 100.0);

    println!("\n  六大归一化体系概览：");
    for sys in platform.systems_overview() {
        println!(
            "    [{}] {} - {}/{} 模块正常",
            if sys.status == PlatformModuleStatus::Running {
                "✓"
            } else {
                "✗"
            },
            sys.name,
            sys.healthy_count,
            sys.module_count
        );
    }
    println!();

    // ========== 3. AI 驱动的业务申请编排 ==========
    println!("【3】AI 驱动业务申请编排：");
    let context = OrchestrationContext {
        tenant_id: "tenant-acme".to_string(),
        user_id: "user-zhangsan".to_string(),
        original_request: "我想申请请假3天回家探亲".to_string(),
        variables: std::collections::HashMap::new(),
        current_step: 0,
    };

    let result = platform
        .orchestrate("ai-business-request", context)
        .expect("编排执行失败");

    println!("  编排 ID: {}", result.orchestration_id);
    println!("  执行结果: {}", if result.success { "成功" } else { "失败" });
    println!("  完成步骤: {}/{}", result.completed_steps, result.total_steps);

    println!("\n  执行步骤详情：");
    for step in &result.steps {
        let status_icon = match step.status {
            StepStatus::Success => "✓",
            StepStatus::Failed => "✗",
            StepStatus::Skipped => "→",
            _ => " ",
        };
        println!(
            "    [{}] Step {}: {} ({})",
            status_icon,
            step.order + 1,
            step.name,
            step.step_type.name()
        );
    }
    println!();

    // ========== 4. 事件驱动的跨系统联动 ==========
    println!("【4】事件驱动跨系统联动：");
    let corr_id = "demo-chain-001";

    // Step 1: AI 发布意图识别事件 → 自动触发权限校验
    let intent_event = PlatformEvent::new(
        EventType::IntentRecognized,
        NormalizationSystem::AiAssistant,
        "tenant-acme",
        serde_json::json!({
            "intent": "leave_application",
            "confidence": 0.95,
            "parameters": {"days": 3}
        }),
    )
    .with_user_id("user-zhangsan")
    .with_correlation_id(corr_id);

    let r1 = platform.publish_event(intent_event);
    println!("  发布 IntentRecognized 事件 → {} 个订阅者响应", r1.len());
    for r in &r1 {
        println!("    - {}: {}", r.handler_id, if r.success { "成功" } else { "失败" });
    }

    // Step 2: 权限系统发布校验通过事件 → 自动触发表单生成
    let perm_event = PlatformEvent::new(
        EventType::PermissionChecked,
        NormalizationSystem::Permission,
        "tenant-acme",
        serde_json::json!({
            "allowed": true,
            "reason": "employee has leave permission",
            "role": "employee"
        }),
    )
    .with_correlation_id(corr_id);

    let r2 = platform.publish_event(perm_event);
    println!("  发布 PermissionChecked 事件 → {} 个订阅者响应", r2.len());
    for r in &r2 {
        println!("    - {}: {}", r.handler_id, if r.success { "成功" } else { "失败" });
    }

    // Step 3: 低代码发布表单提交事件 → 自动启动流程
    let form_event = PlatformEvent::new(
        EventType::FormSubmitted,
        NormalizationSystem::Lowcode,
        "tenant-acme",
        serde_json::json!({
            "form_id": "leave-001",
            "data": {"title": "请假申请", "days": 3, "reason": "回家探亲"}
        }),
    )
    .with_correlation_id(corr_id);

    let r3 = platform.publish_event(form_event);
    println!("  发布 FormSubmitted 事件 → {} 个订阅者响应", r3.len());
    for r in &r3 {
        println!("    - {}: {}", r.handler_id, if r.success { "成功" } else { "失败" });
    }

    // Step 4: 流程发布完成事件 → 前端通知 + 架构同步
    let proc_event = PlatformEvent::new(
        EventType::ProcessCompleted,
        NormalizationSystem::ProcessAlgo,
        "tenant-acme",
        serde_json::json!({
            "process_id": "proc-leave-001",
            "status": "approved",
            "approver": "manager-lisi"
        }),
    )
    .with_correlation_id(corr_id);

    let r4 = platform.publish_event(proc_event);
    println!("  发布 ProcessCompleted 事件 → {} 个订阅者响应", r4.len());
    for r in &r4 {
        println!("    - {}: {}", r.handler_id, if r.success { "成功" } else { "失败" });
    }

    // 事件溯源：查询完整事件链
    let chain = platform.event_bus().query_by_correlation(corr_id);
    println!("\n  事件溯源：完整链路共 {} 个事件", chain.len());
    println!();

    // ========== 5. 配置中心分层配置 ==========
    println!("【5】统一配置中心：");

    // 读取全局默认配置
    let global_theme = platform.config().get_global("frontend.theme").unwrap();
    println!("  全局默认主题: {}", global_theme);

    // 读取架构超时配置
    let timeout = platform
        .config()
        .get_global("arch.request_timeout_ms")
        .unwrap();
    println!("  全局请求超时: {}ms", timeout);

    // 租户级覆盖
    platform
        .config()
        .set(
            "frontend.theme",
            serde_json::json!("dark"),
            ConfigLevel::Tenant,
            Some("tenant-acme"),
            None,
        )
        .unwrap();
    println!("  租户 tenant-acme 覆盖主题为 dark");

    let tenant_theme = platform
        .config()
        .get("frontend.theme", Some("tenant-acme"), None)
        .unwrap();
    println!("  租户 tenant-acme 当前主题: {}", tenant_theme);

    // 用户级进一步覆盖
    platform
        .config()
        .set(
            "frontend.theme",
            serde_json::json!("auto"),
            ConfigLevel::User,
            Some("tenant-acme"),
            Some("user-zhangsan"),
        )
        .unwrap();
    println!("  用户 user-zhangsan 覆盖主题为 auto");

    let user_theme = platform
        .config()
        .get("frontend.theme", Some("tenant-acme"), Some("user-zhangsan"))
        .unwrap();
    println!("  用户 user-zhangsan 当前主题: {}", user_theme);

    // 验证其他用户仍继承租户配置
    let other_user_theme = platform
        .config()
        .get("frontend.theme", Some("tenant-acme"), Some("user-lisi"))
        .unwrap();
    println!("  用户 user-lisi 当前主题: {} (继承租户)", other_user_theme);

    // 配置校验：非法值会被拒绝
    let invalid_result = platform
        .config()
        .set_global("frontend.theme", serde_json::json!("blue"));
    println!("  设置非法主题值: {:?}", invalid_result.err().unwrap().to_string());
    println!();

    // ========== 6. 平台指标 ==========
    println!("【6】平台运行指标：");
    let metrics = platform.metrics();
    println!("  总请求数: {}", metrics.total_requests);
    println!("  成功率: {:.1}%", platform.status_monitor().success_rate() * 100.0);
    println!("  事件发布总数: {}", platform.event_bus().published_count());
    println!("  可用编排模板: {}", platform.orchestration_templates().len());
    println!("  内置配置项: {}", platform.config().schema_count());
    println!();

    // ========== 7. 直接访问子系统 ==========
    println!("【7】子系统直接访问：");

    // 访问权限系统
    println!("  权限系统 - RBAC + ABAC 混合模型");
    println!("    - 角色管理: perm::rbac::RoleManager");
    println!("    - 策略引擎: perm::policy_engine::PolicyEngine");
    println!("    - SSO 支持: perm::sso::SsoManager");

    // 访问低代码系统
    println!("  低代码系统 - 元数据驱动开发");
    println!("    - 实体建模: lowcode::metadata::EntityMetadata");
    println!("    - 表单引擎: lowcode::form_engine::FormEngine");
    println!("    - 表达式求值: lowcode::expression::ExpressionEngine");

    // 访问流程系统
    println!("  流程系统 - 规则+流程+算法融合");
    println!("    - 规则引擎: process::rule_engine::RuleEngine");
    println!("    - 决策表: process::decision_table::DecisionTable");
    println!("    - 流程引擎: process::process_engine::ProcessEngine");

    println!();

    // ========== 8. 优雅关闭 ==========
    println!("【8】优雅关闭平台...");
    platform.shutdown().expect("平台关闭失败");
    assert!(!platform.is_ready());
    println!("  ✓ 平台已关闭\n");

    println!("=== 快速启动演示完成 ===");
}
