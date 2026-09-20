//! 组织机构完整管理模块
//!
//! 支持：公司管理 / 部门管理（树形） / 岗位管理 / 人员管理 / 组织架构树

pub mod api;

use serde::{Deserialize, Serialize};

/// 公司
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub company_id: String,
    pub company_name: String,
    pub company_code: String,
    pub company_type: String, // group/company/subsidiary/branch
    pub parent_company_id: Option<String>,
    pub legal_person: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: String, // normal/disabled
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 部门
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Department {
    pub dept_id: String,
    pub company_id: String,
    pub dept_name: String,
    pub dept_code: String,
    pub parent_dept_id: Option<String>,
    pub dept_type: String, // root/division/department/team/group
    pub leader_id: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 岗位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub position_id: String,
    pub position_name: String,
    pub position_code: String,
    pub dept_id: Option<String>,
    pub position_level: Option<String>, // P1-P10 / M1-M5
    pub job_family: Option<String>, // tech/product/design/marketing/hr/finance
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub created_at: String,
}

/// 员工
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub employee_id: String,
    pub user_id: Option<String>,
    pub employee_no: String,
    pub name: String,
    pub gender: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company_id: String,
    pub dept_id: String,
    pub position_id: Option<String>,
    pub leader_id: Option<String>,
    pub employment_type: String, // fulltime/parttime/intern/contractor
    pub status: String, // active/leave/resigned
    pub entry_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 组织树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgTreeNode {
    pub id: String,
    pub node_type: String, // company/department/position/employee
    pub name: String,
    pub parent_id: Option<String>,
    pub children: Vec<OrgTreeNode>,
}

/// 内置示例公司
pub fn sample_companies() -> Vec<Company> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        Company {
            company_id: "comp_001".to_string(),
            company_name: "XX科技集团".to_string(),
            company_code: "XX_GROUP".to_string(),
            company_type: "group".to_string(),
            parent_company_id: None,
            legal_person: Some("张总".to_string()),
            address: Some("北京市海淀区".to_string()),
            phone: Some("010-12345678".to_string()),
            email: Some("contact@xx.com".to_string()),
            status: "normal".to_string(),
            sort_order: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        Company {
            company_id: "comp_002".to_string(),
            company_name: "XX电网公司".to_string(),
            company_code: "XX_GRID".to_string(),
            company_type: "subsidiary".to_string(),
            parent_company_id: Some("comp_001".to_string()),
            legal_person: Some("李总".to_string()),
            address: Some("广州市天河区".to_string()),
            phone: Some("020-87654321".to_string()),
            email: Some("grid@xx.com".to_string()),
            status: "normal".to_string(),
            sort_order: 2,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
    ]
}

/// 内置示例部门
pub fn sample_departments() -> Vec<Department> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        Department {
            dept_id: "dept_001".to_string(),
            company_id: "comp_001".to_string(),
            dept_name: "总部".to_string(),
            dept_code: "HQ".to_string(),
            parent_dept_id: None,
            dept_type: "root".to_string(),
            leader_id: None,
            phone: None,
            email: None,
            status: "normal".to_string(),
            sort_order: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        Department {
            dept_id: "dept_002".to_string(),
            company_id: "comp_001".to_string(),
            dept_name: "技术研发中心".to_string(),
            dept_code: "TECH".to_string(),
            parent_dept_id: Some("dept_001".to_string()),
            dept_type: "division".to_string(),
            leader_id: Some("emp_001".to_string()),
            phone: Some("010-12345679".to_string()),
            email: Some("tech@xx.com".to_string()),
            status: "normal".to_string(),
            sort_order: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        Department {
            dept_id: "dept_003".to_string(),
            company_id: "comp_001".to_string(),
            dept_name: "人力资源部".to_string(),
            dept_code: "HR".to_string(),
            parent_dept_id: Some("dept_001".to_string()),
            dept_type: "department".to_string(),
            leader_id: Some("emp_002".to_string()),
            phone: None,
            email: Some("hr@xx.com".to_string()),
            status: "normal".to_string(),
            sort_order: 2,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
    ]
}

/// 内置示例岗位
pub fn sample_positions() -> Vec<Position> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        Position {
            position_id: "pos_001".to_string(),
            position_name: "技术总监".to_string(),
            position_code: "TECH_DIRECTOR".to_string(),
            dept_id: Some("dept_002".to_string()),
            position_level: Some("M4".to_string()),
            job_family: Some("tech".to_string()),
            description: Some("负责技术团队管理和架构设计".to_string()),
            status: "normal".to_string(),
            sort_order: 1,
            created_at: now.clone(),
        },
        Position {
            position_id: "pos_002".to_string(),
            position_name: "高级工程师".to_string(),
            position_code: "SENIOR_ENGINEER".to_string(),
            dept_id: Some("dept_002".to_string()),
            position_level: Some("P6".to_string()),
            job_family: Some("tech".to_string()),
            description: None,
            status: "normal".to_string(),
            sort_order: 2,
            created_at: now.clone(),
        },
        Position {
            position_id: "pos_003".to_string(),
            position_name: "HR经理".to_string(),
            position_code: "HR_MANAGER".to_string(),
            dept_id: Some("dept_003".to_string()),
            position_level: Some("M3".to_string()),
            job_family: Some("hr".to_string()),
            description: None,
            status: "normal".to_string(),
            sort_order: 1,
            created_at: now,
        },
    ]
}

/// 内置示例员工
pub fn sample_employees() -> Vec<Employee> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        Employee {
            employee_id: "emp_001".to_string(),
            user_id: Some("user_001".to_string()),
            employee_no: "E001".to_string(),
            name: "张三".to_string(),
            gender: Some("男".to_string()),
            phone: Some("13800138001".to_string()),
            email: Some("zhangsan@xx.com".to_string()),
            company_id: "comp_001".to_string(),
            dept_id: "dept_002".to_string(),
            position_id: Some("pos_001".to_string()),
            leader_id: None,
            employment_type: "fulltime".to_string(),
            status: "active".to_string(),
            entry_date: Some("2020-01-15".to_string()),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        Employee {
            employee_id: "emp_002".to_string(),
            user_id: Some("user_002".to_string()),
            employee_no: "E002".to_string(),
            name: "李四".to_string(),
            gender: Some("女".to_string()),
            phone: Some("13800138002".to_string()),
            email: Some("lisi@xx.com".to_string()),
            company_id: "comp_001".to_string(),
            dept_id: "dept_003".to_string(),
            position_id: Some("pos_003".to_string()),
            leader_id: None,
            employment_type: "fulltime".to_string(),
            status: "active".to_string(),
            entry_date: Some("2021-03-20".to_string()),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
    ]
}
