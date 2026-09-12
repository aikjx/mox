//! 组织机构管理 API 端点

use crate::organization::*;
use crate::enterprise::api_response::*;
use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 组织机构状态
pub struct OrganizationState {
    pub companies: Arc<RwLock<HashMap<String, Company>>>,
    pub departments: Arc<RwLock<HashMap<String, Department>>>,
    pub positions: Arc<RwLock<HashMap<String, Position>>>,
    pub employees: Arc<RwLock<HashMap<String, Employee>>>,
}

impl OrganizationState {
    pub fn new() -> Self {
        let mut companies = HashMap::new();
        for c in sample_companies() { companies.insert(c.company_id.clone(), c); }

        let mut departments = HashMap::new();
        for d in sample_departments() { departments.insert(d.dept_id.clone(), d); }

        let mut positions = HashMap::new();
        for p in sample_positions() { positions.insert(p.position_id.clone(), p); }

        let mut employees = HashMap::new();
        for e in sample_employees() { employees.insert(e.employee_id.clone(), e); }

        Self {
            companies: Arc::new(RwLock::new(companies)),
            departments: Arc::new(RwLock::new(departments)),
            positions: Arc::new(RwLock::new(positions)),
            employees: Arc::new(RwLock::new(employees)),
        }
    }
}

impl Default for OrganizationState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 公司管理 ====================

/// GET /api/enterprise/org/companies —— 获取公司列表
pub async fn list_companies_handler(
    State(state): State<Arc<OrganizationState>>,
) -> Response {
    let companies = state.companies.read().await;
    let mut list: Vec<&Company> = companies.values().collect();
    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    success(list)
}

/// GET /api/enterprise/org/companies/:id —— 获取公司详情
pub async fn get_company_handler(
    State(state): State<Arc<OrganizationState>>,
    Path(id): Path<String>,
) -> Response {
    let companies = state.companies.read().await;
    match companies.get(&id) {
        Some(c) => success(c),
        None => not_found("公司不存在"),
    }
}

// ==================== 部门管理 ====================

/// GET /api/enterprise/org/departments —— 获取部门列表
pub async fn list_departments_handler(
    State(state): State<Arc<OrganizationState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let depts = state.departments.read().await;
    let mut list: Vec<&Department> = depts.values().collect();

    if let Some(company_id) = params.get("company_id") {
        list.retain(|d| d.company_id == *company_id);
    }
    if let Some(parent_id) = params.get("parent_dept_id") {
        list.retain(|d| d.parent_dept_id.as_deref() == Some(parent_id.as_str()));
    }

    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    success(list)
}

/// GET /api/enterprise/org/departments/tree —— 获取部门树
pub async fn get_department_tree_handler(
    State(state): State<Arc<OrganizationState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let depts = state.departments.read().await;
    let company_id = params.get("company_id").cloned();

    let mut roots: Vec<OrgTreeNode> = Vec::new();
    let mut children_map: HashMap<String, Vec<OrgTreeNode>> = HashMap::new();

    for d in depts.values() {
        if let Some(cid) = &company_id {
            if &d.company_id != cid { continue; }
        }
        let node = OrgTreeNode {
            id: d.dept_id.clone(),
            node_type: "department".to_string(),
            name: d.dept_name.clone(),
            parent_id: d.parent_dept_id.clone(),
            children: Vec::new(),
        };
        match &d.parent_dept_id {
            Some(pid) => children_map.entry(pid.clone()).or_insert_with(Vec::new).push(node),
            None => roots.push(node),
        }
    }

    // 构建树
    fn build_tree(node: &mut OrgTreeNode, children_map: &HashMap<String, Vec<OrgTreeNode>>) {
        if let Some(children) = children_map.get(&node.id) {
            node.children = children.clone();
            for child in &mut node.children {
                build_tree(child, children_map);
            }
        }
    }

    for root in &mut roots {
        build_tree(root, &children_map);
    }

    success(roots)
}

/// GET /api/enterprise/org/departments/:id —— 获取部门详情
pub async fn get_department_handler(
    State(state): State<Arc<OrganizationState>>,
    Path(id): Path<String>,
) -> Response {
    let depts = state.departments.read().await;
    match depts.get(&id) {
        Some(d) => success(d),
        None => not_found("部门不存在"),
    }
}

// ==================== 岗位管理 ====================

/// GET /api/enterprise/org/positions —— 获取岗位列表
pub async fn list_positions_handler(
    State(state): State<Arc<OrganizationState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let positions = state.positions.read().await;
    let mut list: Vec<&Position> = positions.values().collect();

    if let Some(dept_id) = params.get("dept_id") {
        list.retain(|p| p.dept_id.as_deref() == Some(dept_id.as_str()));
    }
    if let Some(job_family) = params.get("job_family") {
        list.retain(|p| p.job_family.as_deref() == Some(job_family.as_str()));
    }

    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    success(list)
}

// ==================== 员工管理 ====================

/// GET /api/enterprise/org/employees —— 获取员工列表
pub async fn list_employees_handler(
    State(state): State<Arc<OrganizationState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let employees = state.employees.read().await;
    let mut list: Vec<&Employee> = employees.values().collect();

    if let Some(company_id) = params.get("company_id") {
        list.retain(|e| e.company_id == *company_id);
    }
    if let Some(dept_id) = params.get("dept_id") {
        list.retain(|e| e.dept_id == *dept_id);
    }
    if let Some(position_id) = params.get("position_id") {
        list.retain(|e| e.position_id.as_deref() == Some(position_id.as_str()));
    }
    if let Some(status) = params.get("status") {
        list.retain(|e| e.status == *status);
    }
    if let Some(keyword) = params.get("keyword") {
        list.retain(|e| e.name.contains(keyword) || e.employee_no.contains(keyword));
    }

    list.sort_by(|a, b| a.employee_no.cmp(&b.employee_no));

    let (page, page_size) = parse_pagination(&params);
    let pagination = Pagination::new(page, page_size, list.len());
    let page_items = pagination.paginate(&list);
    success_list(page_items, &pagination)
}

/// GET /api/enterprise/org/employees/:id —— 获取员工详情
pub async fn get_employee_handler(
    State(state): State<Arc<OrganizationState>>,
    Path(id): Path<String>,
) -> Response {
    let employees = state.employees.read().await;
    match employees.get(&id) {
        Some(e) => success(e),
        None => not_found("员工不存在"),
    }
}

/// GET /api/enterprise/org/stats —— 组织统计
pub async fn org_stats_handler(
    State(state): State<Arc<OrganizationState>>,
) -> Response {
    let companies = state.companies.read().await;
    let departments = state.departments.read().await;
    let positions = state.positions.read().await;
    let employees = state.employees.read().await;

    let active_employees = employees.values().filter(|e| e.status == "active").count();

    success(json!({
        "total_companies": companies.len(),
        "total_departments": departments.len(),
        "total_positions": positions.len(),
        "total_employees": employees.len(),
        "active_employees": active_employees,
        "by_company": companies.values().map(|c| {
            let count = employees.values().filter(|e| e.company_id == c.company_id).count();
            json!({"company_id": c.company_id, "company_name": c.company_name, "employee_count": count})
        }).collect::<Vec<_>>(),
    }))
}

/// 构建组织机构路由（泛型版本）
pub fn build_organization_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<OrganizationState>: axum::extract::FromRef<S>,
{
    use axum::routing::get;

    axum::Router::new()
        .route("/companies", get(list_companies_handler))
        .route("/companies/:id", get(get_company_handler))
        .route("/departments", get(list_departments_handler))
        .route("/departments/tree", get(get_department_tree_handler))
        .route("/departments/:id", get(get_department_handler))
        .route("/positions", get(list_positions_handler))
        .route("/employees", get(list_employees_handler))
        .route("/employees/:id", get(get_employee_handler))
        .route("/stats", get(org_stats_handler))
}
