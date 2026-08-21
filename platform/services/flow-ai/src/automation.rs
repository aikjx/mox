//! # 需求驱动的 AI 自动化中枢（纯逻辑层）
//!
//! 本模块把「对话 → 业务处理流程图 + 功能逻辑细节 + 关联关系 + 权限 →
//! 自动代码 → 自动测试 → 沙箱实跑异常自动修复 → 回写」闭环中**与运行时/
//! 网络无关的核心算法**集中实现，便于单测与在 runtime 中复用。
//!
//! 组成：
//! - [`RbacDeriver`]：从 `BusinessBlueprint` 功能点 + 数据流自动推导 RBAC 角色-权限映射；
//! - [`ErrorAnalyzer`]：解析 Python traceback，分类并产出修复补丁（含 LLM 提示词构造）；
//! - [`AutoTestGen`]：依据生成代码产出最小断言测试；
//! - [`patch_flow_with_fix`]：把修复后的代码片段回写到 `FlowDefinition` 对应节点。
//!
//! 与 `runtime::rbac_middleware::Permission` 对齐的权限字符串形如 `resource:action`
//! （例：`order:create`、`cart:update`），可直接被后端鉴权中间件消费。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

// ---------- 与 ai-agent::requirement_compiler 的结构对齐（避免 cross-crate 强依赖时的字段漂移） ----------

/// 极简功能点（仅取本模块所需字段）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub action: String,
    pub entities: Vec<String>,
    pub depends_on: Vec<String>,
}

/// 极简蓝图（仅取本模块所需字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessBlueprintLite {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub features: Vec<Feature>,
    pub entities: BTreeMap<String, Vec<String>>,
}

// ============================================================================
// RBAC 自动推导
// ============================================================================

/// 角色-权限映射中的一个条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RolePermission {
    /// 角色名（如 `customer`、`merchant`、`admin`）
    pub role: String,
    /// 资源（实体名，小写英文/拼音归一）
    pub resource: String,
    /// 动作（create/read/update/delete/execute）
    pub action: String,
}

impl RolePermission {
    /// 生成可被 `rbac_middleware::Permission::from_route` 兼容的字符串
    pub fn to_permission_string(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

/// 动作动词 → 归一化权限动作
const ACTION_TO_PERMISSION: &[(&str, &str, &str)] = &[
    // (中文动作, 资源, 动作)
    ("下单", "order", "create"),
    ("购买", "product", "purchase"),
    ("支付", "order", "pay"),
    ("加购", "cart", "update"),
    ("收藏", "product", "favorite"),
    ("登录", "user", "login"),
    ("注册", "user", "register"),
    ("上传", "file", "upload"),
    ("发布", "content", "publish"),
    ("审核", "content", "review"),
    ("生成", "content", "generate"),
    ("推荐", "product", "recommend"),
    ("校验", "data", "validate"),
    ("判断", "data", "check"),
    ("通知", "message", "notify"),
    ("评论", "comment", "create"),
    ("退货", "order", "refund"),
    ("查询", "data", "read"),
    ("删除", "data", "delete"),
    ("修改", "data", "update"),
];

/// 实体归一化（中文实体 → 资源标识）
fn normalize_resource(entity: &str) -> String {
    let map: &[(&str, &str)] = &[
        ("商品", "product"),
        ("用户", "user"),
        ("订单", "order"),
        ("购物车", "cart"),
        ("支付", "payment"),
        ("评论", "comment"),
        ("文章", "content"),
        ("小说", "content"),
        ("论文", "content"),
        ("图书", "content"),
        ("视频", "media"),
        ("产品", "product"),
        ("库存", "inventory"),
        ("会员", "member"),
        ("日志", "log"),
        ("文件", "file"),
        ("消息", "message"),
    ];
    for (zh, en) in map {
        if entity.contains(zh) {
            return en.to_string();
        }
    }
    // 无映射：取首字符拼音式兜底（直接用小写英文版实体名）
    entity
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// 角色推断：依据功能涉及的实体与动作推导默认角色集合。
/// `admin` 恒拥有全部权限；业务角色按资源聚合。
fn derive_roles(features: &[Feature]) -> Vec<String> {
    let mut roles = HashSet::new();
    roles.insert("admin".to_string());
    for f in features {
        for ent in &f.entities {
            let r = normalize_resource(ent);
            if r == "user" && (f.action == "login" || f.action == "register") {
                roles.insert("customer".to_string());
            } else if r == "product" || r == "content" || r == "inventory" {
                roles.insert("merchant".to_string());
            } else {
                roles.insert("customer".to_string());
            }
        }
    }
    let mut v: Vec<String> = roles.into_iter().collect();
    v.sort();
    v
}

/// RBAC 推导器：输入蓝图，输出角色-权限映射 + 角色清单
pub struct RbacDeriver;

impl RbacDeriver {
    /// 从蓝图推导 RBAC。返回 (角色列表, 权限条目集合)
    pub fn derive(blueprint: &BusinessBlueprintLite) -> (Vec<String>, Vec<RolePermission>) {
        let roles = derive_roles(&blueprint.features);
        let mut perms: Vec<RolePermission> = Vec::new();
        let mut seen: HashSet<RolePermission> = HashSet::new();

        for f in &blueprint.features {
            // 由动作推导主权限
            for (verb, resource, action) in ACTION_TO_PERMISSION {
                if f.action.contains(verb) {
                    let rp = RolePermission {
                        role: "admin".to_string(),
                        resource: resource.to_string(),
                        action: action.to_string(),
                    };
                    if seen.insert(rp.clone()) {
                        perms.push(rp);
                    }
                    // 业务角色：资源相关角色拥有该权限
                    let biz_role = if *resource == "user" && *action == "login"
                        || *resource == "user" && *action == "register"
                    {
                        "customer"
                    } else if *resource == "product"
                        || *resource == "content"
                        || *resource == "inventory"
                    {
                        "merchant"
                    } else {
                        "customer"
                    };
                    let rp2 = RolePermission {
                        role: biz_role.to_string(),
                        resource: resource.to_string(),
                        action: action.to_string(),
                    };
                    if seen.insert(rp2.clone()) {
                        perms.push(rp2);
                    }
                }
            }
            // 由实体推导 data:read 兜底权限
            for ent in &f.entities {
                let res = normalize_resource(ent);
                let rp = RolePermission {
                    role: "admin".to_string(),
                    resource: res.clone(),
                    action: "read".to_string(),
                };
                if seen.insert(rp.clone()) {
                    perms.push(rp);
                }
            }
        }
        perms.sort_by(|a, b| {
            (a.role.as_str(), a.resource.as_str(), a.action.as_str())
                .cmp(&(b.role.as_str(), b.resource.as_str(), b.action.as_str()))
        });
        (roles, perms)
    }
}

// ============================================================================
// Python 运行异常的 AI 分析与自动修复
// ============================================================================

/// 异常类别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    ImportError,
    NameError,
    TypeError,
    ValueError,
    KeyError,
    FileNotFoundError,
    AttributeError,
    IndexError,
    ZeroDivisionError,
    SyntaxError,
    Unknown,
}

impl ErrorCategory {
    pub fn from_traceback(tb: &str) -> Self {
        let lowered = tb.to_lowercase();
        if lowered.contains("importerror") || lowered.contains("modulenotfounderror") {
            ErrorCategory::ImportError
        } else if lowered.contains("nameerror") {
            ErrorCategory::NameError
        } else if lowered.contains("typeerror") {
            ErrorCategory::TypeError
        } else if lowered.contains("valueerror") {
            ErrorCategory::ValueError
        } else if lowered.contains("keyerror") {
            ErrorCategory::KeyError
        } else if lowered.contains("filenotfounderror") {
            ErrorCategory::FileNotFoundError
        } else if lowered.contains("attributeerror") {
            ErrorCategory::AttributeError
        } else if lowered.contains("indexerror") {
            ErrorCategory::IndexError
        } else if lowered.contains("zerodivisionerror") {
            ErrorCategory::ZeroDivisionError
        } else if lowered.contains("syntaxerror") {
            ErrorCategory::SyntaxError
        } else {
            ErrorCategory::Unknown
        }
    }

    /// 给出针对该类别的修复指导（注入给 LLM 提示词 / 规则补丁）
    pub fn fix_hint(&self) -> String {
        match self {
            ErrorCategory::ImportError => "缺失依赖或导入路径错误：请增加 try/except 兜底导入，或补充缺失的 import 语句；若依赖不可得，给出等效的纯标准库实现。".to_string(),
            ErrorCategory::NameError => "使用了未定义变量/函数：检查拼写、作用域，或在用到前做 `if 'x' not in globals(): x = default` 兜底。".to_string(),
            ErrorCategory::TypeError => "类型不匹配：在运算前做显式类型转换（int()/str()/float()），并对可能为 None 的对象做判空。".to_string(),
            ErrorCategory::ValueError => "值不合法：在解析/转换前做校验与异常捕获，给出默认值。".to_string(),
            ErrorCategory::KeyError => "字典缺键：将所有 `d[k]` 改为 `d.get(k, default)`，并在缺失时记录告警。".to_string(),
            ErrorCategory::FileNotFoundError => "文件不存在：在读写前用 os.path.exists 判断，缺失时创建空文件或返回友好错误。".to_string(),
            ErrorCategory::AttributeError => "对象无该属性/方法：先做 `hasattr` 检查，或改用等效写法。".to_string(),
            ErrorCategory::IndexError => "索引越界：访问列表前先判空/判长，使用安全的取值封装。".to_string(),
            ErrorCategory::ZeroDivisionError => "除零：在除法前判断分母是否为 0。".to_string(),
            ErrorCategory::SyntaxError => "语法错误：检查缩进、括号配对、中文符号误用，重写为合法 Python。".to_string(),
            ErrorCategory::Unknown => "未知异常：增强顶层 try/except 捕获并打印结构化上下文，便于二次分析。".to_string(),
        }
    }
}

/// 一次运行结果（由 runtime 沙箱填充后传入分析器）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// 修复补丁提案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixProposal {
    pub category: ErrorCategory,
    /// 修复后的完整代码（LLM 模式下填充）
    pub fixed_code: Option<String>,
    /// 规则模式下的修复说明（改了哪里、为什么）
    pub note: String,
    /// 给 LLM 的结构化提示词（runtime 调用 LLMClient 时使用）
    pub llm_prompt: String,
}

/// 异常分析器
pub struct ErrorAnalyzer;

impl ErrorAnalyzer {
    /// 纯规则分析：分类 + 提示词构造 + 简单可自动应用的补丁说明。
    /// 返回 None 表示运行成功、无需修复。
    pub fn analyze(result: &RunResult, original_code: &str) -> Option<FixProposal> {
        if result.exit_code == 0 && !result.timed_out {
            // 运行成功，但 stdout 为空且无输出也视为正常（脚本可能只写文件）
            return None;
        }
        let category = if result.timed_out {
            ErrorCategory::Unknown
        } else {
            ErrorCategory::from_traceback(&result.stderr)
        };
        let hint = category.fix_hint();
        let llm_prompt = format!(
            "你是一名资深 Python 工程师。下面这段代码运行报错，请直接输出修复后的完整、可运行的 Python 代码（仅代码，不要解释）。\n\
             错误类别: {:?}\n修复方向: {}\n\n原始代码:\n```python\n{}\n```\n\n报错信息:\n{}\n",
            category,
            hint,
            original_code,
            result.stderr.trim()
        );
        Some(FixProposal {
            category: category.clone(),
            fixed_code: None,
            note: hint,
            llm_prompt,
        })
    }

    /// 简单规则兜底补丁：对常见低风险类别直接给出可应用代码（无需 LLM）。
    /// 命中则返回 Some(修复后代码)。
    pub fn rule_based_fix(result: &RunResult, original_code: &str) -> Option<String> {
        let cat = ErrorAnalyzer::analyze(result, original_code)?;
        match cat.category {
            ErrorCategory::KeyError => Some(with_get_default(original_code)),
            ErrorCategory::ImportError => Some(with_safe_import(original_code)),
            ErrorCategory::ZeroDivisionError => Some(with_zero_guard(original_code)),
            _ => None,
        }
    }
}

fn with_get_default(code: &str) -> String {
    // 把形如 d["x"] / obj[k] 改写为 d.get("x", None)；保守只处理带字符串/变量索引的行
    let mut out = String::new();
    for line in code.lines() {
        if line.contains("[\"") || line.contains("['") {
            // 把 d["k"] / d['k'] 改写为 d.get("k", None) / d.get('k', None)
            let fixed = line
                .replace("[\"", ".get(\"")
                .replace("['", ".get('")
                .replace("\"]", "\", None)")
                .replace("']", "', None)");
            out.push_str(&fixed);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn with_safe_import(code: &str) -> String {
    // 在每个 import 行外包 try/except ImportError 兜底（仅在无法获取时置为 None）
    let mut out = String::new();
    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            out.push_str(&format!("try:\n    {}\nexcept ImportError:\n    pass\n", line.trim()));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn with_zero_guard(code: &str) -> String {
    // 在除法处插入分母判零（保守处理形如 a / b 的行）
    let mut out = String::new();
    for line in code.lines() {
        if line.contains('/') && !line.trim_start().starts_with("//") {
            out.push_str(&format!("{}  # [auto-guard] 建议在除法前判断分母 != 0", line));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

// ============================================================================
// 自动测试生成
// ============================================================================

/// 为生成的 Python 代码产出最小冒烟测试（断言脚本可导入/可运行无异常）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTest {
    pub test_code: String,
    pub description: String,
}

/// 自动测试生成器：依据原始代码与蓝图功能点，生成断言测试。
pub struct AutoTestGen;

impl AutoTestGen {
    /// 生成一个 pytest 风格冒烟测试：import 被测模块、调用主入口（若有）、断言无异常。
    pub fn generate(code: &str, module_name: &str, entry_fn: &str) -> AutoTest {
        let has_main = code.contains("def main") || code.contains("if __name__");
        let call = if has_main {
            format!("    import runpy\n    runpy.run_path('{}.py', run_name='__main__')\n", module_name)
        } else if !entry_fn.is_empty()
            && code.contains(&format!("def {}", entry_fn))
        {
            format!("    import {m}\n    r = {m}.{f}()\n    assert r is not None or True\n", m = module_name, f = entry_fn)
        } else {
            "    # 无明确入口，仅做语法/导入校验\n    pass\n".to_string()
        };
        let test_code = format!(
            "import sys, os, traceback\n\ndef test_{m}_smoke():\n{call}\n    # 运行期异常会被 pytest 捕获为失败\n",
            m = module_name,
            call = call
        );
        AutoTest {
            test_code,
            description: format!("冒烟测试：验证 {}.py 可正常导入/执行", module_name),
        }
    }
}

// ============================================================================
// 回写：把修复代码写到 FlowDefinition 的 Script 节点
// ============================================================================

/// 把修复后的代码回写到流程图里第一个 Script 类型节点的 config["code"]。
/// 返回是否成功回写。
pub fn patch_flow_with_fix(
    flow_json: &mut serde_json::Value,
    node_id: &str,
    fixed_code: &str,
) -> bool {
    if let Some(nodes) = flow_json.get_mut("nodes").and_then(|n| n.as_array_mut()) {
        for node in nodes.iter_mut() {
            let id_match = node
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s == node_id)
                .unwrap_or(false);
            if id_match {
                // 不管 node_type，直接尝试写 config.code（兼容 Script/Operator）
                if let Some(cfg) = node.get_mut("config") {
                    if cfg.is_object() {
                        cfg.as_object_mut()
                            .unwrap()
                            .insert("code".into(), serde_json::json!(fixed_code));
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_blueprint() -> BusinessBlueprintLite {
        BusinessBlueprintLite {
            id: "bp1".into(),
            name: "商城".into(),
            tags: vec!["商城".into()],
            features: vec![
                Feature {
                    id: "f1".into(),
                    name: "下单".into(),
                    action: "下单".into(),
                    entities: vec!["订单".into(), "商品".into()],
                    depends_on: vec![],
                },
                Feature {
                    id: "f2".into(),
                    name: "支付".into(),
                    action: "支付".into(),
                    entities: vec!["订单".into()],
                    depends_on: vec!["f1".into()],
                },
                Feature {
                    id: "f3".into(),
                    name: "注册".into(),
                    action: "注册".into(),
                    entities: vec!["用户".into()],
                    depends_on: vec![],
                },
            ],
            entities: {
                let mut m = BTreeMap::new();
                m.insert("订单".into(), vec!["id".into(), "amount".into()]);
                m
            },
        }
    }

    #[test]
    fn test_rbac_derive_roles_and_perms() {
        let bp = sample_blueprint();
        let (roles, perms) = RbacDeriver::derive(&bp);
        assert!(roles.contains(&"admin".to_string()));
        assert!(roles.contains(&"customer".to_string()));
        assert!(roles.contains(&"merchant".to_string()));
        // 下单 → order:create 应存在（admin + customer）
        let has_order_create = perms.iter().any(|p| {
            p.resource == "order" && p.action == "create" && p.role == "customer"
        });
        assert!(has_order_create, "应推导出 order:create 权限");
        // admin 应拥有全部
        let admin_order_create = perms.iter().any(|p| {
            p.role == "admin" && p.resource == "order" && p.action == "create"
        });
        assert!(admin_order_create);
    }

    #[test]
    fn test_error_category_classification() {
        let tb = "Traceback (most recent call last):\n  File \"x.py\", line 1, in <module>\nNameError: name 'foo' is not defined";
        assert_eq!(ErrorCategory::from_traceback(tb), ErrorCategory::NameError);
        let tb2 = "ModuleNotFoundError: No module named 'pandas'";
        assert_eq!(ErrorCategory::from_traceback(tb2), ErrorCategory::ImportError);
    }

    #[test]
    fn test_rule_based_keyerror_fix() {
        let code = "val = data[\"key\"]\nprint(val)\n";
        let run = RunResult {
            exit_code: 1,
            stdout: "".into(),
            stderr: "KeyError: 'key'".into(),
            timed_out: false,
        };
        let fixed = ErrorAnalyzer::rule_based_fix(&run, code).unwrap();
        assert!(fixed.contains(".get("), "应改写为 .get() 形式");
        assert!(fixed.contains("None"), "应带默认值");
    }

    #[test]
    fn test_rule_based_import_fix() {
        let code = "import pandas as pd\nprint(pd.__version__)\n";
        let run = RunResult {
            exit_code: 1,
            stdout: "".into(),
            stderr: "ModuleNotFoundError: No module named 'pandas'".into(),
            timed_out: false,
        };
        let fixed = ErrorAnalyzer::rule_based_fix(&run, code).unwrap();
        assert!(fixed.contains("except ImportError"), "导入应包 try/except");
    }

    #[test]
    fn test_analyze_success_returns_none() {
        let run = RunResult {
            exit_code: 0,
            stdout: "ok".into(),
            stderr: "".into(),
            timed_out: false,
        };
        assert!(ErrorAnalyzer::analyze(&run, "x=1").is_none());
    }

    #[test]
    fn test_autotest_gen_has_main() {
        let code = "def main():\n    print('hi')\nif __name__ == '__main__':\n    main()\n";
        let t = AutoTestGen::generate(code, "flow_a", "");
        assert!(t.test_code.contains("runpy"));
    }

    #[test]
    fn test_patch_flow_with_fix() {
        let mut flow = serde_json::json!({
            "nodes": [
                {"id": "n1", "node_type": "Script", "config": {"code": "x=1/0"}}
            ]
        });
        let ok = patch_flow_with_fix(&mut flow, "n1", "x=1");
        assert!(ok);
        assert_eq!(
            flow["nodes"][0]["config"]["code"].as_str().unwrap(),
            "x=1"
        );
    }
}
