// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! codegen 端点冒烟测试：定义实体 → run_codegen（TPL-01/03/05）→ 产物断言 + 错误路径。

use mox_platform_enterprise_svc::app_state::AppState;
use mox_platform_enterprise_svc::routes::{run_codegen, CodegenRequest};
use mox_platform_meta_core::{FieldDef, FieldType};

async fn build_state() -> AppState {
    AppState::open_memory_or_file(":memory:", &["common"])
        .await
        .expect("AppState init")
}

fn project_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            code: "title".into(),
            name: "标题".into(),
            r#type: FieldType::String,
            required: true,
            indexed: true,
            searchable: true,
            sortable: true,
            filterable: true,
            ui_component: None,
            options_inline: None,
        },
        FieldDef {
            code: "amount".into(),
            name: "金额".into(),
            r#type: FieldType::Decimal,
            required: false,
            indexed: false,
            searchable: false,
            sortable: true,
            filterable: true,
            ui_component: None,
            options_inline: None,
        },
    ]
}

fn req(entity: &str, template: &str, detail: Option<&str>) -> CodegenRequest {
    CodegenRequest {
        tenant_id: Some("t-codegen".into()),
        entity_code: entity.into(),
        detail_entity_code: detail.map(std::string::ToString::to_string),
        template: Some(template.into()),
    }
}

#[tokio::test]
async fn t01_codegen_tpl01_crud() {
    let s = build_state().await;
    s.meta
        .define_entity(
            Some("t-codegen".into()),
            "project".into(),
            "项目".into(),
            project_fields(),
        )
        .expect("define entity");

    let resp = run_codegen(&s.meta, &req("project", "TPL-01", None)).expect("codegen ok");
    assert_eq!(resp.template, "TPL-01");
    assert!(resp.artifacts.contains_key("ddl/biz_project.sql"));
    assert!(resp.artifacts.contains_key("vue/ProjectList.vue"));
    assert!(resp.artifacts["ddl/biz_project.sql"].contains("title"));
}

#[tokio::test]
async fn t02_codegen_tpl03_master_detail() {
    let s = build_state().await;
    s.meta
        .define_entity(
            Some("t-codegen".into()),
            "order".into(),
            "订单".into(),
            project_fields(),
        )
        .expect("define master");
    s.meta
        .define_entity(
            Some("t-codegen".into()),
            "item".into(),
            "明细".into(),
            project_fields(),
        )
        .expect("define detail");

    let resp = run_codegen(&s.meta, &req("order", "TPL-03", Some("item"))).expect("codegen ok");
    assert_eq!(resp.template, "TPL-03");
    assert!(resp.artifacts.contains_key("ddl/biz_order.sql"));
    assert!(resp.artifacts.contains_key("ddl/biz_item_detail.sql"));
}

#[tokio::test]
async fn t03_codegen_tpl05_workflow_and_unknown_template() {
    let s = build_state().await;
    s.meta
        .define_entity(
            Some("t-codegen".into()),
            "leave".into(),
            "请假".into(),
            project_fields(),
        )
        .expect("define entity");

    let resp = run_codegen(&s.meta, &req("leave", "TPL-05", None)).expect("codegen ok");
    assert!(resp.artifacts.contains_key("wf/leave.workflow.json"));

    let err = run_codegen(&s.meta, &req("leave", "TPL-99", None)).expect_err("reject unknown");
    assert!(err.contains("unsupported template"), "got: {err}");

    let err = run_codegen(&s.meta, &req("missing", "TPL-01", None)).expect_err("reject missing");
    assert!(err.contains("not found"), "got: {err}");

    let err =
        run_codegen(&s.meta, &req("leave", "TPL-03", None)).expect_err("reject missing detail");
    assert!(err.contains("detail_entity_code"), "got: {err}");
}
