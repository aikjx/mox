// Copyright (c) 2026 璇玑 RelGraph · mox 模块化系统架构归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 端到端集成测试
//!
//! 验证六大归一化体系 + 统一平台核心能力的协同工作

#[cfg(test)]
mod integration_tests {
    use mox_flow_unified_platform::*;
    use std::collections::HashMap;

    /// 测试1：平台完整启动与健康检查
    #[test]
    fn test_platform_bootstrap_health() {
        let platform = PlatformFacade::new();

        // 启动前状态
        assert!(!platform.is_ready());

        // 启动平台
        platform.bootstrap().unwrap();
        assert!(platform.is_ready());

        // 健康度检查
        let health = platform.health();
        assert_eq!(health.overall_status, PlatformModuleStatus::Running);
        assert_eq!(health.healthy_count, 9);
        assert_eq!(health.total_count, 9);
        assert_eq!(health.health_score(), 1.0);

        // 六大体系概览
        let systems = platform.systems_overview();
        assert_eq!(systems.len(), 6);
        for sys in &systems {
            assert_eq!(sys.status, PlatformModuleStatus::Running);
        }

        // 优雅关闭
        platform.shutdown().unwrap();
        assert!(!platform.is_ready());
    }

    /// 测试2：AI驱动的业务申请全流程编排
    #[test]
    fn test_ai_business_request_orchestration() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let context = OrchestrationContext {
            tenant_id: "tenant-acme".to_string(),
            user_id: "user-zhangsan".to_string(),
            original_request: "我想申请请假3天回家探亲".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = platform
            .orchestrate("ai-business-request", context)
            .unwrap();

        // 整体成功
        assert!(result.success);
        assert_eq!(result.total_steps, 6);
        assert_eq!(result.completed_steps, 6);
        assert_eq!(result.failed_steps, 0);

        // 验证六大体系全部被调用
        let mut systems_used = std::collections::HashSet::new();
        for step in &result.steps {
            systems_used.insert(step.step_type.system());
        }
        assert_eq!(systems_used.len(), 6);

        // 验证步骤顺序正确
        let step_types: Vec<OrchestrationStepType> =
            result.steps.iter().map(|s| s.step_type).collect();
        assert_eq!(step_types[0], OrchestrationStepType::AiIntent);
        assert_eq!(step_types[1], OrchestrationStepType::PermissionCheck);
        assert_eq!(step_types[2], OrchestrationStepType::LowcodeGenerate);
        assert_eq!(step_types[3], OrchestrationStepType::ProcessExecute);
        assert_eq!(step_types[4], OrchestrationStepType::FrontendRender);
        assert_eq!(step_types[5], OrchestrationStepType::ArchOutput);

        // 验证最终输出存在
        assert!(result.final_output.is_some());
        let output = result.final_output.unwrap();
        assert!(output.get("protocol").is_some());
        assert!(output.get("payload").is_some());
    }

    /// 测试3：事件驱动的跨系统联动全链路
    #[test]
    fn test_event_driven_full_chain() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let corr_id = "integration-test-chain-001";

        // Step 1: AI 发布意图识别事件
        let intent_event = PlatformEvent::new(
            EventType::IntentRecognized,
            NormalizationSystem::AiAssistant,
            "t1",
            serde_json::json!({
                "intent": "leave_application",
                "confidence": 0.95,
                "parameters": {"days": 3, "reason": "探亲"}
            }),
        )
        .with_user_id("u1")
        .with_correlation_id(corr_id);

        let r1 = platform.publish_event(intent_event);
        assert_eq!(r1.len(), 1); // 权限系统自动校验
        assert!(r1[0].success);

        // Step 2: 权限系统发布校验通过事件
        let perm_event = PlatformEvent::new(
            EventType::PermissionChecked,
            NormalizationSystem::Permission,
            "t1",
            serde_json::json!({
                "allowed": true,
                "reason": "user has leave_application permission",
                "role": "employee"
            }),
        )
        .with_correlation_id(corr_id);

        let r2 = platform.publish_event(perm_event);
        assert_eq!(r2.len(), 1); // 低代码自动生成表单
        assert!(r2[0].success);

        // Step 3: 低代码发布表单提交事件
        let form_event = PlatformEvent::new(
            EventType::FormSubmitted,
            NormalizationSystem::Lowcode,
            "t1",
            serde_json::json!({
                "form_id": "leave-001",
                "data": {
                    "title": "请假申请",
                    "days": 3,
                    "reason": "回家探亲"
                }
            }),
        )
        .with_correlation_id(corr_id);

        let r3 = platform.publish_event(form_event);
        assert_eq!(r3.len(), 1); // 流程自动启动
        assert!(r3[0].success);

        // Step 4: 流程发布完成事件
        let proc_event = PlatformEvent::new(
            EventType::ProcessCompleted,
            NormalizationSystem::ProcessAlgo,
            "t1",
            serde_json::json!({
                "process_id": "proc-leave-001",
                "status": "approved",
                "approver": "manager-lisi"
            }),
        )
        .with_correlation_id(corr_id);

        let r4 = platform.publish_event(proc_event);
        assert_eq!(r4.len(), 2); // 前端通知 + 架构同步
        assert!(r4.iter().all(|r| r.success));

        // 验证事件溯源：完整链路可追溯
        let chain = platform.event_bus().query_by_correlation(corr_id);
        assert_eq!(chain.len(), 4);

        // 验证事件顺序
        assert_eq!(chain[0].event_type, EventType::IntentRecognized);
        assert_eq!(chain[1].event_type, EventType::PermissionChecked);
        assert_eq!(chain[2].event_type, EventType::FormSubmitted);
        assert_eq!(chain[3].event_type, EventType::ProcessCompleted);
    }

    /// 测试4：配置中心分层覆盖
    #[test]
    fn test_config_hierarchical_override() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let key = "frontend.theme";

        // 全局默认：light
        assert_eq!(
            platform.config().get_global(key).unwrap(),
            serde_json::json!("light")
        );

        // 租户 A 覆盖为 dark
        platform
            .config()
            .set(
                key,
                serde_json::json!("dark"),
                ConfigLevel::Tenant,
                Some("tenant-a"),
                None,
            )
            .unwrap();

        // 租户 A 的用户看到 dark
        assert_eq!(
            platform.config().get(key, Some("tenant-a"), None).unwrap(),
            serde_json::json!("dark")
        );

        // 租户 B 的用户仍看到 light（全局默认）
        assert_eq!(
            platform.config().get(key, Some("tenant-b"), None).unwrap(),
            serde_json::json!("light")
        );

        // 租户 A 内用户 X 进一步覆盖为 auto
        platform
            .config()
            .set(
                key,
                serde_json::json!("auto"),
                ConfigLevel::User,
                Some("tenant-a"),
                Some("user-x"),
            )
            .unwrap();

        // 用户 X 看到 auto（最高优先级）
        assert_eq!(
            platform
                .config()
                .get(key, Some("tenant-a"), Some("user-x"))
                .unwrap(),
            serde_json::json!("auto")
        );

        // 租户 A 内其他用户仍看到 dark
        assert_eq!(
            platform
                .config()
                .get(key, Some("tenant-a"), Some("user-y"))
                .unwrap(),
            serde_json::json!("dark")
        );
    }

    /// 测试5：配置校验机制
    #[test]
    fn test_config_validation_rules() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 枚举值校验失败
        assert!(platform
            .config()
            .set_global("frontend.theme", serde_json::json!("blue"))
            .is_err());

        // 数值范围校验失败
        assert!(platform
            .config()
            .set_global("arch.request_timeout_ms", serde_json::json!(500))
            .is_err()); // 低于最小值 1000

        assert!(platform
            .config()
            .set_global("arch.request_timeout_ms", serde_json::json!(500000))
            .is_err()); // 高于最大值 300000

        // 类型校验失败
        assert!(platform
            .config()
            .set_global("perm.rbac_enabled", serde_json::json!("yes"))
            .is_err()); // 应该是布尔值

        // 合法值通过
        assert!(platform
            .config()
            .set_global("frontend.theme", serde_json::json!("dark"))
            .is_ok());
        assert!(platform
            .config()
            .set_global("arch.request_timeout_ms", serde_json::json!(60000))
            .is_ok());
    }

    /// 测试6：配置变更监听
    #[test]
    fn test_config_change_listener() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let change_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = change_count.clone();

        platform.config().add_listener(Box::new(move |event| {
            assert_eq!(event.key, "frontend.theme");
            assert!(event.old_value.is_some());
            count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }));

        // 修改配置
        platform
            .config()
            .set_global("frontend.theme", serde_json::json!("dark"))
            .unwrap();

        assert_eq!(change_count.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    /// 测试7：平台指标监控
    #[test]
    fn test_platform_metrics_monitoring() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 初始状态
        let metrics = platform.metrics();
        assert_eq!(metrics.total_requests, 0);

        // 模拟请求
        platform.status_monitor().record_request(true, 50.0);
        platform.status_monitor().record_request(true, 30.0);
        platform.status_monitor().record_request(false, 100.0);

        let metrics = platform.metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 1);

        // 成功率
        assert!((platform.status_monitor().success_rate() - 2.0 / 3.0).abs() < 0.01);
    }

    /// 测试8：模板化编排 + 事件驱动混合模式
    #[test]
    fn test_mixed_orchestration_and_event() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let corr_id = "mixed-mode-001";

        // 第一阶段：使用编排模板执行前3步
        let ctx = OrchestrationContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            original_request: "我想申请报销".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = platform
            .orchestrate("ai-business-request", ctx)
            .unwrap();
        assert!(result.success);

        // 第二阶段：发布事件触发后续联动
        let proc_event = PlatformEvent::new(
            EventType::ProcessCompleted,
            NormalizationSystem::ProcessAlgo,
            "t1",
            serde_json::json!({"status": "approved"}),
        )
        .with_correlation_id(corr_id);

        let results = platform.publish_event(proc_event);
        assert_eq!(results.len(), 2); // 前端 + 架构

        // 验证事件总发布数
        assert_eq!(platform.event_bus().published_count(), 1);
    }

    /// 测试9：六大体系配置完整性
    #[test]
    fn test_all_systems_have_config() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 每个体系都应有配置项
        for system in NormalizationSystem::all() {
            let schemas = platform.config().list_schemas_by_system(system);
            assert!(
                !schemas.is_empty(),
                "system {:?} should have config schemas",
                system
            );
        }
    }

    /// 测试10：编排模板覆盖主要场景
    #[test]
    fn test_orchestration_templates_coverage() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let templates = platform.orchestration_templates();
        assert!(templates.contains(&"ai-business-request".to_string()));
        assert!(templates.contains(&"ai-query-only".to_string()));
        assert!(templates.contains(&"algo-analysis".to_string()));

        // 验证各模板能正常执行
        for template_id in &templates {
            let ctx = OrchestrationContext {
                tenant_id: "t1".to_string(),
                user_id: "u1".to_string(),
                original_request: "test".to_string(),
                variables: HashMap::new(),
                current_step: 0,
            };
            let result = platform.orchestrate(template_id, ctx).unwrap();
            assert!(result.success, "template {} should succeed", template_id);
        }
    }

    /// 测试11：AI智能问答轻量流程
    #[test]
    fn test_ai_query_only_lightweight() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let ctx = OrchestrationContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            original_request: "什么是知识图谱？".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = platform.orchestrate("ai-query-only", ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.total_steps, 2); // 意图理解 + 生成回答
    }

    /// 测试12：算法分析流程
    #[test]
    fn test_algo_analysis_flow() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let ctx = OrchestrationContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            original_request: "分析数据趋势".to_string(),
            variables: HashMap::new(),
            current_step: 0,
        };

        let result = platform.orchestrate("algo-analysis", ctx).unwrap();
        assert!(result.success);
        assert_eq!(result.total_steps, 3); // 权限 + 算法 + 可视化
    }

    /// 测试13：权限拒绝时的事件传播
    #[test]
    fn test_permission_denied_event_propagation() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 权限校验失败
        let perm_event = PlatformEvent::new(
            EventType::PermissionChecked,
            NormalizationSystem::Permission,
            "t1",
            serde_json::json!({
                "allowed": false,
                "reason": "insufficient_privileges"
            }),
        );

        let results = platform.publish_event(perm_event);
        assert_eq!(results.len(), 1); // lowcode-auto-form
        assert!(!results[0].success); // 应该失败
    }

    /// 测试14：事件历史溯源
    #[test]
    fn test_event_history_tracing() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        // 发布多个事件
        for i in 0..5 {
            platform.publish_event(PlatformEvent::new(
                EventType::UserAction,
                NormalizationSystem::Frontend,
                "t1",
                serde_json::json!({"action": i}),
            ));
        }

        // 查询历史
        let history = platform
            .event_bus()
            .query_history(&EventType::UserAction, 10);
        assert_eq!(history.len(), 5);

        // 验证发布计数
        assert!(platform.event_bus().published_count() >= 5);
    }

    /// 测试15：全平台模块注册完整性
    #[test]
    fn test_full_platform_module_registry() {
        let platform = PlatformFacade::new();
        platform.bootstrap().unwrap();

        let health = platform.health();
        assert_eq!(health.total_count, 9);

        // 9 个模块：
        // 架构归一化：unified-storage, unified-meta, unified-arch (3个)
        // 权限归一化：unified-perm (1个)
        // 算法联盟：algo-alliance (1个)
        // 流程算法归一：unified-process (1个)
        // 低代码：lowcode-core (1个)
        // 前端归一：unified-frontend (1个)
        // AI助手：ai-assistant (1个)
        let arch_modules = platform
            .lifecycle()
            .get_modules_by_system(NormalizationSystem::Architecture);
        assert_eq!(arch_modules.len(), 3);

        let perm_modules = platform
            .lifecycle()
            .get_modules_by_system(NormalizationSystem::Permission);
        assert_eq!(perm_modules.len(), 1);

        let lowcode_modules = platform
            .lifecycle()
            .get_modules_by_system(NormalizationSystem::Lowcode);
        assert_eq!(lowcode_modules.len(), 1);

        let process_modules = platform
            .lifecycle()
            .get_modules_by_system(NormalizationSystem::ProcessAlgo);
        assert_eq!(process_modules.len(), 2); // algo-alliance + unified-process

        let frontend_modules = platform
            .lifecycle()
            .get_modules_by_system(NormalizationSystem::Frontend);
        assert_eq!(frontend_modules.len(), 1);

        let ai_modules = platform
            .lifecycle()
            .get_modules_by_system(NormalizationSystem::AiAssistant);
        assert_eq!(ai_modules.len(), 1);
    }
}
