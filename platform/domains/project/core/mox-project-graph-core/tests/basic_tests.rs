// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

use mox_project_graph_core::*;

// ─── Schema 枚举测试 ─────────────────────────────────────────────────────────

#[test]
fn project_status_variants_and_labels() {
    use ProjectStatus::*;
    assert_eq!(Planning.label(), "规划中");
    assert_eq!(InProgress.label(), "进行中");
    assert_eq!(Paused.label(), "已暂停");
    assert_eq!(Completed.label(), "已完成");
    assert_eq!(Cancelled.label(), "已取消");
    // Clone + Copy
    let s = InProgress;
    let s2 = s;
    assert_eq!(s, s2);
}

#[test]
fn requirement_status_progress_weights() {
    use RequirementStatus::*;
    assert_eq!(PendingReview.progress_weight(), 0.0);
    assert_eq!(Confirmed.progress_weight(), 0.1);
    assert_eq!(InDevelopment.progress_weight(), 0.4);
    assert_eq!(InTesting.progress_weight(), 0.7);
    assert_eq!(Released.progress_weight(), 1.0);
    assert_eq!(Rejected.progress_weight(), 0.0);
}

#[test]
fn task_status_progress_weights() {
    use TaskStatus::*;
    assert_eq!(Todo.progress_weight(), 0.0);
    assert_eq!(InProgress.progress_weight(), 0.5);
    assert_eq!(Completed.progress_weight(), 1.0);
    assert_eq!(Blocked.progress_weight(), 0.0);
    assert_eq!(Cancelled.progress_weight(), 0.0);
}

#[test]
fn priority_weights() {
    use Priority::*;
    assert_eq!(P0.weight(), 4);
    assert_eq!(P1.weight(), 3);
    assert_eq!(P2.weight(), 2);
    assert_eq!(P3.weight(), 1);
    assert_eq!(P0.label(), "紧急");
    assert_eq!(P1.label(), "高");
    assert_eq!(P2.label(), "中");
    assert_eq!(P3.label(), "低");
}

#[test]
fn risk_level_labels() {
    use RiskLevel::*;
    assert_eq!(Low.label(), "低");
    assert_eq!(Medium.label(), "中");
    assert_eq!(High.label(), "高");
    assert_eq!(Critical.label(), "紧急");
}

#[test]
fn issue_status_labels() {
    use IssueStatus::*;
    assert_eq!(Open.label(), "待处理");
    assert_eq!(Investigating.label(), "处理中");
    assert_eq!(Resolved.label(), "已解决");
    assert_eq!(Closed.label(), "已关闭");
}

#[test]
fn entity_types_constants() {
    assert_eq!(entity_types::PROJECT, "project");
    assert_eq!(entity_types::REQUIREMENT, "requirement");
    assert_eq!(entity_types::TASK, "task");
    assert_eq!(entity_types::MILESTONE, "milestone");
    assert_eq!(entity_types::PERSON, "person");
    assert_eq!(entity_types::ISSUE, "issue");
    assert_eq!(entity_types::DOCUMENT, "document");
    assert_eq!(entity_types::TAG, "tag");
}

#[test]
fn edge_types_constants() {
    assert_eq!(edge_types::CONTAINS, "contains");
    assert_eq!(edge_types::DECOMPOSES_INTO, "decomposes_into");
    assert_eq!(edge_types::ASSIGNED_TO, "assigned_to");
    assert_eq!(edge_types::DEPENDS_ON, "depends_on");
    assert_eq!(edge_types::BLOCKS, "blocks");
    assert_eq!(edge_types::TRACKS, "tracks");
    assert_eq!(edge_types::REPORTED_BY, "reported_by");
    assert_eq!(edge_types::RELATED_TO, "related_to");
    assert_eq!(edge_types::DESCRIBES, "describes");
    assert_eq!(edge_types::TAGGED_WITH, "tagged_with");
    assert_eq!(edge_types::MANAGES, "manages");
    assert_eq!(edge_types::BELONGS_TO, "belongs_to");
}

// ─── Props 结构体测试 ────────────────────────────────────────────────────────

#[test]
fn project_props_default_values() {
    let props = ProjectProps {
        name: "测试项目".to_string(),
        code: "TEST-001".to_string(),
        description: None,
        status: ProjectStatus::Planning,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    };
    assert_eq!(props.name, "测试项目");
    assert_eq!(props.code, "TEST-001");
    assert_eq!(props.status, ProjectStatus::Planning);
    assert_eq!(props.progress, 0.0);
    assert!(props.tags.is_empty());
    assert!(props.description.is_none());
}

#[test]
fn requirement_props_construction() {
    let props = RequirementProps {
        title: "用户登录功能".to_string(),
        description: Some("支持邮箱/手机号登录".to_string()),
        status: RequirementStatus::PendingReview,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: Some("客户".to_string()),
        story_points: Some(8),
        acceptance_criteria: Some("登录成功返回 token".to_string()),
        created_by: Some("pm".to_string()),
        tags: vec!["auth".to_string()],
        metadata: None,
    };
    assert_eq!(props.title, "用户登录功能");
    assert_eq!(props.priority, Priority::P1);
    assert_eq!(props.story_points, Some(8));
    assert_eq!(props.tags.len(), 1);
}

#[test]
fn task_props_construction() {
    let props = TaskProps {
        title: "实现登录接口".to_string(),
        description: Some("后端 REST API".to_string()),
        status: TaskStatus::Todo,
        priority: Priority::P0,
        task_type: "开发".to_string(),
        estimate_hours: Some(4.0),
        actual_hours: None,
        due_date: Some("2026-09-01".to_string()),
        assignee_id: None,
        tags: vec!["backend".to_string()],
        metadata: None,
    };
    assert_eq!(props.priority, Priority::P0);
    assert_eq!(props.estimate_hours, Some(4.0));
    assert_eq!(props.task_type, "开发");
}

#[test]
fn person_props_and_milestone_props() {
    let person = PersonProps {
        name: "张三".to_string(),
        email: Some("zhangsan@example.com".to_string()),
        role: Some("开发工程师".to_string()),
        avatar: None,
        department: Some("技术部".to_string()),
        metadata: None,
    };
    assert_eq!(person.name, "张三");
    assert_eq!(person.role, Some("开发工程师".to_string()));

    let ms = MilestoneProps {
        name: "V1.0 发布".to_string(),
        description: Some("首个版本上线".to_string()),
        target_date: "2026-12-31".to_string(),
        is_completed: false,
        completed_date: None,
        progress: 0.0,
        metadata: None,
    };
    assert_eq!(ms.name, "V1.0 发布");
    assert!(!ms.is_completed);
}

#[test]
fn issue_and_document_and_tag_props() {
    let issue = IssueProps {
        title: "登录超时".to_string(),
        description: Some("5 分钟无操作自动登出".to_string()),
        status: IssueStatus::Open,
        risk_level: RiskLevel::Medium,
        reported_by: Some("qa".to_string()),
        assignee_id: None,
        tags: vec!["bug".to_string()],
        metadata: None,
    };
    assert_eq!(issue.risk_level, RiskLevel::Medium);
    assert_eq!(issue.status, IssueStatus::Open);

    let doc = DocumentProps {
        title: "需求文档".to_string(),
        doc_type: "PRD".to_string(),
        url: Some("https://example.com/prd".to_string()),
        content: None,
        author: Some("产品经理".to_string()),
        metadata: None,
    };
    assert_eq!(doc.doc_type, "PRD");

    let tag = TagProps {
        name: "高优先级".to_string(),
        color: Some("#ff0000".to_string()),
        description: Some("紧急处理".to_string()),
        category: Some("优先级".to_string()),
    };
    assert_eq!(tag.name, "高优先级");
}

// ─── 引擎基础测试 ────────────────────────────────────────────────────────────

#[tokio::test]
async fn engine_new_and_default() {
    let engine = ProjectGraphEngine::new();
    let projects = engine.list_projects().await;
    assert!(projects.is_empty());

    let engine2 = ProjectGraphEngine::default();
    let projects2 = engine2.list_projects().await;
    assert!(projects2.is_empty());
}

#[tokio::test]
async fn create_and_get_project() {
    let engine = ProjectGraphEngine::new();
    let props = ProjectProps {
        name: "电商平台".to_string(),
        code: "ECOM".to_string(),
        description: Some("一个完整的电商系统".to_string()),
        status: ProjectStatus::Planning,
        priority: Priority::P1,
        start_date: Some("2026-01-01".to_string()),
        end_date: Some("2026-12-31".to_string()),
        owner_id: Some("pm-001".to_string()),
        progress: 0.0,
        tags: vec!["电商".to_string()],
        metadata: None,
    };
    let id = engine.create_project(props).await;
    assert!(id.starts_with("project:"));
    assert!(id.contains("ECOM"));

    let (v, p) = engine.get_project(&id).await.unwrap();
    assert_eq!(v.id, id);
    assert_eq!(p.name, "电商平台");
    assert_eq!(p.code, "ECOM");
    assert_eq!(p.status, ProjectStatus::Planning);
    assert_eq!(p.tags, vec!["电商".to_string()]);
}

#[tokio::test]
async fn update_project() {
    let engine = ProjectGraphEngine::new();
    let id = engine.create_project(ProjectProps {
        name: "旧名称".to_string(),
        code: "OLD".to_string(),
        description: None,
        status: ProjectStatus::Planning,
        priority: Priority::P3,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let updated = engine.update_project(&id, ProjectProps {
        name: "新名称".to_string(),
        code: "OLD".to_string(),
        description: Some("更新后的描述".to_string()),
        status: ProjectStatus::InProgress,
        priority: Priority::P1,
        start_date: Some("2026-06-01".to_string()),
        end_date: None,
        owner_id: Some("new-owner".to_string()),
        progress: 0.5,
        tags: vec!["important".to_string()],
        metadata: None,
    }).await;
    assert!(updated);

    let (_, p) = engine.get_project(&id).await.unwrap();
    assert_eq!(p.name, "新名称");
    assert_eq!(p.status, ProjectStatus::InProgress);
    assert_eq!(p.priority, Priority::P1);
}

#[tokio::test]
async fn create_requirement_under_project() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "测试项目".to_string(),
        code: "TP".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let req_id = engine.create_requirement(&project_id, RequirementProps {
        title: "登录功能".to_string(),
        description: Some("用户登录".to_string()),
        status: RequirementStatus::Confirmed,
        priority: Priority::P0,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: Some(5),
        acceptance_criteria: None,
        created_by: None,
        tags: vec!["auth".to_string()],
        metadata: None,
    }).await;

    assert!(req_id.starts_with("req:"));

    let reqs = engine.list_requirements(&project_id).await;
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].1.title, "登录功能");
    assert_eq!(reqs[0].1.priority, Priority::P0);
}

#[tokio::test]
async fn create_task_under_requirement() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "项目A".to_string(),
        code: "PA".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let req_id = engine.create_requirement(&project_id, RequirementProps {
        title: "需求1".to_string(),
        description: None,
        status: RequirementStatus::InDevelopment,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: None,
        acceptance_criteria: None,
        created_by: None,
        tags: vec![],
        metadata: None,
    }).await;

    let task_id = engine.create_task(&req_id, "requirement", TaskProps {
        title: "任务1".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(8.0),
        actual_hours: None,
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    assert!(task_id.starts_with("task:"));

    let tasks = engine.list_tasks_of_requirement(&req_id).await;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].1.title, "任务1");

    // 也能在项目级看到任务
    let proj_tasks = engine.list_tasks_of_project(&project_id).await;
    assert_eq!(proj_tasks.len(), 1);
}

#[tokio::test]
async fn assign_task_to_person() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "P".to_string(),
        code: "P".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let req_id = engine.create_requirement(&project_id, RequirementProps {
        title: "R".to_string(),
        description: None,
        status: RequirementStatus::InDevelopment,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: None,
        acceptance_criteria: None,
        created_by: None,
        tags: vec![],
        metadata: None,
    }).await;

    let task_id = engine.create_task(&req_id, "requirement", TaskProps {
        title: "T".to_string(),
        description: None,
        status: TaskStatus::InProgress,
        priority: Priority::P0,
        task_type: "开发".to_string(),
        estimate_hours: Some(4.0),
        actual_hours: Some(2.0),
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    let person_id = engine.create_person(PersonProps {
        name: "李四".to_string(),
        email: None,
        role: None,
        avatar: None,
        department: None,
        metadata: None,
    }).await;

    engine.assign_task(&task_id, &person_id).await;

    let person_tasks = engine.list_person_tasks(&person_id, None).await;
    assert_eq!(person_tasks.len(), 1);
    assert_eq!(person_tasks[0].1.title, "T");

    // 按状态过滤
    let in_progress = engine.list_person_tasks(&person_id, Some(TaskStatus::InProgress)).await;
    assert_eq!(in_progress.len(), 1);
    let completed = engine.list_person_tasks(&person_id, Some(TaskStatus::Completed)).await;
    assert_eq!(completed.len(), 0);
}

#[tokio::test]
async fn project_progress_calculation() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "进度测试".to_string(),
        code: "PROG".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    // 空项目进度为 0
    let progress = engine.recalc_project_progress(&project_id).await;
    assert_eq!(progress, 0.0);

    // 添加一个已完成的需求（P1 优先级，进度权重 1.0）
    engine.create_requirement(&project_id, RequirementProps {
        title: "已完成需求".to_string(),
        description: None,
        status: RequirementStatus::Released,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: None,
        acceptance_criteria: None,
        created_by: None,
        tags: vec![],
        metadata: None,
    }).await;

    let progress = engine.recalc_project_progress(&project_id).await;
    // 只有一个需求，已发布(1.0)，P1 权重 3 => 进度 = 1.0
    assert!((progress - 1.0).abs() < 0.01);
}

#[tokio::test]
async fn person_workload_calculation() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "W".to_string(),
        code: "W".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let req_id = engine.create_requirement(&project_id, RequirementProps {
        title: "R".to_string(),
        description: None,
        status: RequirementStatus::InDevelopment,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: None,
        acceptance_criteria: None,
        created_by: None,
        tags: vec![],
        metadata: None,
    }).await;

    let person_id = engine.create_person(PersonProps {
        name: "王五".to_string(),
        email: None,
        role: None,
        avatar: None,
        department: None,
        metadata: None,
    }).await;

    // 分配 2 个任务：1 个 P0 进行中，1 个 P1 已完成
    let t1 = engine.create_task(&req_id, "requirement", TaskProps {
        title: "T1".to_string(),
        description: None,
        status: TaskStatus::InProgress,
        priority: Priority::P0,
        task_type: "开发".to_string(),
        estimate_hours: Some(8.0),
        actual_hours: Some(3.0),
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;
    engine.assign_task(&t1, &person_id).await;

    let t2 = engine.create_task(&req_id, "requirement", TaskProps {
        title: "T2".to_string(),
        description: None,
        status: TaskStatus::Completed,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(4.0),
        actual_hours: Some(4.0),
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;
    engine.assign_task(&t2, &person_id).await;

    let wl = engine.person_workload(&person_id).await;
    assert_eq!(wl.total_tasks, 2);
    assert_eq!(wl.in_progress, 1);
    assert_eq!(wl.completed, 1);
    assert_eq!(wl.p0_count, 1);
    assert_eq!(wl.p1_count, 1);
    assert_eq!(wl.total_estimate_hours, 12.0);
    assert_eq!(wl.total_actual_hours, 7.0);
}

#[tokio::test]
async fn impact_analysis_with_dependencies() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "IA".to_string(),
        code: "IA".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let req_id = engine.create_requirement(&project_id, RequirementProps {
        title: "R".to_string(),
        description: None,
        status: RequirementStatus::InDevelopment,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: None,
        acceptance_criteria: None,
        created_by: None,
        tags: vec![],
        metadata: None,
    }).await;

    // 任务 A 依赖任务 B（A 必须等 B 完成）
    let task_a = engine.create_task(&req_id, "requirement", TaskProps {
        title: "A".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(2.0),
        actual_hours: None,
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    let task_b = engine.create_task(&req_id, "requirement", TaskProps {
        title: "B".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(3.0),
        actual_hours: None,
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    // A depends_on B => 变更 B 会影响 A
    engine.add_dependency(&task_a, &task_b).await;

    let affected = engine.analyze_impact(&task_b).await;
    assert!(affected.contains(&task_a), "A 应该受 B 变更影响");
}

#[tokio::test]
async fn critical_path_identification() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "CP".to_string(),
        code: "CP".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let req_id = engine.create_requirement(&project_id, RequirementProps {
        title: "R".to_string(),
        description: None,
        status: RequirementStatus::InDevelopment,
        priority: Priority::P1,
        requirement_type: "功能需求".to_string(),
        source: None,
        story_points: None,
        acceptance_criteria: None,
        created_by: None,
        tags: vec![],
        metadata: None,
    }).await;

    // 创建 3 个任务形成链路: A -> B -> C
    // C 依赖 B，B 依赖 A
    let task_a = engine.create_task(&req_id, "requirement", TaskProps {
        title: "A".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(2.0),
        actual_hours: None,
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    let task_b = engine.create_task(&req_id, "requirement", TaskProps {
        title: "B".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(3.0),
        actual_hours: None,
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    let task_c = engine.create_task(&req_id, "requirement", TaskProps {
        title: "C".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: Priority::P1,
        task_type: "开发".to_string(),
        estimate_hours: Some(1.0),
        actual_hours: None,
        due_date: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;

    // B 依赖 A, C 依赖 B
    engine.add_dependency(&task_b, &task_a).await;
    engine.add_dependency(&task_c, &task_b).await;

    let path = engine.critical_path(&project_id).await;
    assert_eq!(path.len(), 3);
    // 路径应该是 A -> B -> C（A 先，C 最后）
    assert_eq!(path[0], task_a);
    assert_eq!(path[2], task_c);
}

#[tokio::test]
async fn milestone_and_issue_creation() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "M".to_string(),
        code: "M".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    let ms_id = engine.create_milestone(&project_id, MilestoneProps {
        name: "M1".to_string(),
        description: Some("首个里程碑".to_string()),
        target_date: "2026-09-30".to_string(),
        is_completed: false,
        completed_date: None,
        progress: 0.0,
        metadata: None,
    }).await;
    assert!(ms_id.starts_with("milestone:"));

    let (_, ms) = engine.get_milestone(&ms_id).await.unwrap();
    assert_eq!(ms.name, "M1");
    assert_eq!(ms.target_date, "2026-09-30");

    let issue_id = engine.create_issue(&project_id, IssueProps {
        title: "Bug-001".to_string(),
        description: None,
        status: IssueStatus::Open,
        risk_level: RiskLevel::High,
        reported_by: None,
        assignee_id: None,
        tags: vec![],
        metadata: None,
    }).await;
    assert!(issue_id.starts_with("issue:"));
}

#[tokio::test]
async fn project_stats_summary() {
    let engine = ProjectGraphEngine::new();
    let project_id = engine.create_project(ProjectProps {
        name: "Stats".to_string(),
        code: "ST".to_string(),
        description: None,
        status: ProjectStatus::InProgress,
        priority: Priority::P2,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;

    // 2 个需求，3 个任务
    let r1 = engine.create_requirement(&project_id, RequirementProps {
        title: "R1".to_string(), description: None,
        status: RequirementStatus::Released, priority: Priority::P1,
        requirement_type: "功能需求".to_string(), source: None,
        story_points: None, acceptance_criteria: None,
        created_by: None, tags: vec![], metadata: None,
    }).await;

    engine.create_requirement(&project_id, RequirementProps {
        title: "R2".to_string(), description: None,
        status: RequirementStatus::InDevelopment, priority: Priority::P2,
        requirement_type: "功能需求".to_string(), source: None,
        story_points: None, acceptance_criteria: None,
        created_by: None, tags: vec![], metadata: None,
    }).await;

    engine.create_task(&r1, "requirement", TaskProps {
        title: "T1".to_string(), description: None,
        status: TaskStatus::Completed, priority: Priority::P1,
        task_type: "开发".to_string(), estimate_hours: Some(2.0),
        actual_hours: None, due_date: None, assignee_id: None,
        tags: vec![], metadata: None,
    }).await;

    let stats = engine.project_stats(&project_id).await;
    assert_eq!(stats.requirement_count, 2);
    assert_eq!(stats.task_count, 1);
    assert_eq!(stats.issue_count, 0);
    assert!(stats.progress > 0.0);
}

#[tokio::test]
async fn document_creation_and_linking() {
    let engine = ProjectGraphEngine::new();
    let doc_id = engine.create_document(DocumentProps {
        title: "设计文档".to_string(),
        doc_type: "设计文档".to_string(),
        url: Some("https://example.com/design".to_string()),
        content: None,
        author: Some("设计师".to_string()),
        metadata: None,
    }).await;
    assert!(doc_id.starts_with("doc:"));

    // 链接到项目
    let project_id = engine.create_project(ProjectProps {
        name: "D".to_string(),
        code: "D".to_string(),
        description: None,
        status: ProjectStatus::Planning,
        priority: Priority::P3,
        start_date: None,
        end_date: None,
        owner_id: None,
        progress: 0.0,
        tags: vec![],
        metadata: None,
    }).await;
    engine.link_document_to(&doc_id, &project_id).await;
    // 验证：从项目出发的入边 describes 应该能找到文档
    use mox_kg_core::TraverseDirection;
    let result = engine.traverse(&project_id, TraverseDirection::In, Some(vec![edge_types::DESCRIBES.to_string()]), 1).await;
    assert_eq!(result.vertices.len(), 2); // 项目 + 文档
}
