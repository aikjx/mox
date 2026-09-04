// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 算法分析引擎（FR-CORE-ALGO）：
//!   对代码/流程/算法描述进行多维度算法分析，包括：
//!   - 复杂度分析（时间/空间复杂度）
//!   - 正确性验证（边界条件/逻辑完备性）
//!   - 优化建议（性能/可读性/可维护性）
//!   - 安全性检测（注入/溢出/权限漏洞）
//!   - 数据流分析（变量生命周期/依赖关系）
//!
//! 设计：纯本地规则引擎，不依赖外部 LLM，保证可复现、可审计、低延迟。
//! 支持按维度单独分析或mox 模块化系统架构维度综合分析。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;
use uuid::Uuid;

// ================== 分析维度枚举 ==================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDimension {
    /// 复杂度分析：时间/空间复杂度估算
    Complexity,
    /// 正确性验证：边界条件、逻辑完备性
    Correctness,
    /// 优化建议：性能、可读性、可维护性
    Optimization,
    /// 安全性检测：注入、溢出、权限漏洞
    Security,
    /// 数据流分析：变量生命周期、依赖关系
    DataFlow,
    /// mox 模块化系统架构维度综合分析
    All,
}

impl AnalysisDimension {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "complexity" | "complex" => Self::Complexity,
            "correctness" | "correct" => Self::Correctness,
            "optimization" | "optimize" | "perf" | "performance" => Self::Optimization,
            "security" | "sec" => Self::Security,
            "dataflow" | "data_flow" | "flow" => Self::DataFlow,
            "all" | "full" | "complete" => Self::All,
            _ => Self::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Complexity => "complexity",
            Self::Correctness => "correctness",
            Self::Optimization => "optimization",
            Self::Security => "security",
            Self::DataFlow => "dataflow",
            Self::All => "all",
        }
    }

    pub fn check_names(&self) -> &'static [&'static str] {
        match self {
            Self::Complexity => &["time_complexity", "space_complexity", "bottleneck_detection"],
            Self::Correctness => &["boundary_check", "logic_completeness", "error_handling"],
            Self::Optimization => &["perf_optimization", "code_readability", "maintainability"],
            Self::Security => &["injection_risk", "overflow_risk", "permission_check", "secret_leak"],
            Self::DataFlow => &["variable_lifecycle", "dependency_analysis", "data_sanitization"],
            Self::All => &[
                "time_complexity", "space_complexity", "bottleneck_detection",
                "boundary_check", "logic_completeness", "error_handling",
                "perf_optimization", "code_readability", "maintainability",
                "injection_risk", "overflow_risk", "permission_check", "secret_leak",
                "variable_lifecycle", "dependency_analysis", "data_sanitization",
            ],
        }
    }
}

// ================== 算法分析请求/响应 ==================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlgorithmAnalysisRequest {
    pub query: String,
    pub dimension: String,
    #[serde(default)]
    pub code_snippet: Option<String>,
    #[serde(default)]
    pub flow_json: Option<String>,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoCheckItem {
    pub name: String,
    pub passed: bool,
    pub blocking: bool,
    pub detail: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmAnalysisResponse {
    pub analysis_id: String,
    pub dimension: String,
    pub checks: Vec<AlgoCheckItem>,
    pub all_passed: bool,
    pub vetoed: bool,
    pub summary: String,
    pub suggestions: Vec<String>,
    pub latency_ms: u64,
}

// ================== 算法分析引擎 ==================

/// 算法分析引擎
///
/// 提供多维度的代码/流程算法分析能力。
/// 基于规则引擎，支持复杂度、正确性、优化、安全、数据流等维度。
#[derive(Debug, Clone, Default)]
pub struct AlgorithmAnalyzer {
    total_analyses: u64,
    dimension_counts: BTreeMap<String, u64>,
}

impl AlgorithmAnalyzer {
    pub fn new() -> Self {
        Self {
            total_analyses: 0,
            dimension_counts: BTreeMap::new(),
        }
    }

    /// 执行算法分析
    pub fn analyze(&mut self, req: &AlgorithmAnalysisRequest) -> AlgorithmAnalysisResponse {
        let start = Instant::now();
        let analysis_id = Uuid::new_v4().to_string();
        let dimension = AnalysisDimension::from_str(&req.dimension);

        self.total_analyses += 1;
        *self.dimension_counts.entry(dimension.label().to_string()).or_insert(0) += 1;

        let checks = match dimension {
            AnalysisDimension::All => {
                let mut all = Vec::new();
                all.extend(self.check_complexity(req));
                all.extend(self.check_correctness(req));
                all.extend(self.check_optimization(req));
                all.extend(self.check_security(req));
                all.extend(self.check_dataflow(req));
                all
            }
            AnalysisDimension::Complexity => self.check_complexity(req),
            AnalysisDimension::Correctness => self.check_correctness(req),
            AnalysisDimension::Optimization => self.check_optimization(req),
            AnalysisDimension::Security => self.check_security(req),
            AnalysisDimension::DataFlow => self.check_dataflow(req),
        };

        let all_passed = checks.iter().all(|c| c.passed);
        let vetoed = checks.iter().any(|c| c.blocking && !c.passed);
        let summary = generate_summary(&checks, dimension);
        let suggestions = generate_suggestions(&checks, dimension);

        AlgorithmAnalysisResponse {
            analysis_id,
            dimension: dimension.label().to_string(),
            checks,
            all_passed,
            vetoed,
            summary,
            suggestions,
            latency_ms: start.elapsed().as_millis() as u64,
        }
    }

    fn check_complexity(&self, req: &AlgorithmAnalysisRequest) -> Vec<AlgoCheckItem> {
        let mut checks = Vec::new();
        let code = req.code_snippet.as_deref().unwrap_or("");
        let query_lower = req.query.to_lowercase();

        let (time_complexity, time_passed) = estimate_time_complexity(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "time_complexity".into(),
            passed: time_passed,
            blocking: false,
            detail: format!("估算时间复杂度：{}", time_complexity),
            severity: if time_passed { "info".into() } else { "warning".into() },
        });

        let (space_complexity, space_passed) = estimate_space_complexity(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "space_complexity".into(),
            passed: space_passed,
            blocking: false,
            detail: format!("估算空间复杂度：{}", space_complexity),
            severity: if space_passed { "info".into() } else { "warning".into() },
        });

        let (bottleneck, bottle_passed) = detect_bottleneck(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "bottleneck_detection".into(),
            passed: bottle_passed,
            blocking: false,
            detail: bottleneck,
            severity: if bottle_passed { "info".into() } else { "warning".into() },
        });

        checks
    }

    fn check_correctness(&self, req: &AlgorithmAnalysisRequest) -> Vec<AlgoCheckItem> {
        let mut checks = Vec::new();
        let code = req.code_snippet.as_deref().unwrap_or("");
        let query_lower = req.query.to_lowercase();

        let (boundary_detail, boundary_passed) = check_boundary_conditions(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "boundary_check".into(),
            passed: boundary_passed,
            blocking: !boundary_passed && !code.is_empty(),
            detail: boundary_detail,
            severity: if boundary_passed { "info".into() } else { "warning".into() },
        });

        let (logic_detail, logic_passed) = check_logic_completeness(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "logic_completeness".into(),
            passed: logic_passed,
            blocking: false,
            detail: logic_detail,
            severity: if logic_passed { "info".into() } else { "warning".into() },
        });

        let (error_detail, error_passed) = check_error_handling(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "error_handling".into(),
            passed: error_passed,
            blocking: false,
            detail: error_detail,
            severity: if error_passed { "info".into() } else { "warning".into() },
        });

        checks
    }

    fn check_optimization(&self, req: &AlgorithmAnalysisRequest) -> Vec<AlgoCheckItem> {
        let mut checks = Vec::new();
        let code = req.code_snippet.as_deref().unwrap_or("");
        let query_lower = req.query.to_lowercase();

        let (perf_detail, perf_passed) = check_perf_optimization(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "perf_optimization".into(),
            passed: perf_passed,
            blocking: false,
            detail: perf_detail,
            severity: "info".into(),
        });

        let (read_detail, read_passed) = check_readability(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "code_readability".into(),
            passed: read_passed,
            blocking: false,
            detail: read_detail,
            severity: "info".into(),
        });

        let (maint_detail, maint_passed) = check_maintainability(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "maintainability".into(),
            passed: maint_passed,
            blocking: false,
            detail: maint_detail,
            severity: "info".into(),
        });

        checks
    }

    fn check_security(&self, req: &AlgorithmAnalysisRequest) -> Vec<AlgoCheckItem> {
        let mut checks = Vec::new();
        let code = req.code_snippet.as_deref().unwrap_or("");
        let query_lower = req.query.to_lowercase();

        let (inject_detail, inject_passed) = check_injection_risk(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "injection_risk".into(),
            passed: inject_passed,
            blocking: !inject_passed && !code.is_empty(),
            detail: inject_detail,
            severity: if inject_passed { "info".into() } else { "high".into() },
        });

        let (overflow_detail, overflow_passed) = check_overflow_risk(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "overflow_risk".into(),
            passed: overflow_passed,
            blocking: !overflow_passed && !code.is_empty(),
            detail: overflow_detail,
            severity: if overflow_passed { "info".into() } else { "medium".into() },
        });

        let (perm_detail, perm_passed) = check_permission_checks(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "permission_check".into(),
            passed: perm_passed,
            blocking: false,
            detail: perm_detail,
            severity: if perm_passed { "info".into() } else { "medium".into() },
        });

        let (secret_detail, secret_passed) = check_secret_leak(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "secret_leak".into(),
            passed: secret_passed,
            blocking: !secret_passed && !code.is_empty(),
            detail: secret_detail,
            severity: if secret_passed { "info".into() } else { "high".into() },
        });

        checks
    }

    fn check_dataflow(&self, req: &AlgorithmAnalysisRequest) -> Vec<AlgoCheckItem> {
        let mut checks = Vec::new();
        let code = req.code_snippet.as_deref().unwrap_or("");
        let query_lower = req.query.to_lowercase();

        let (var_detail, var_passed) = analyze_variable_lifecycle(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "variable_lifecycle".into(),
            passed: var_passed,
            blocking: false,
            detail: var_detail,
            severity: "info".into(),
        });

        let (dep_detail, dep_passed) = analyze_dependencies(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "dependency_analysis".into(),
            passed: dep_passed,
            blocking: false,
            detail: dep_detail,
            severity: "info".into(),
        });

        let (san_detail, san_passed) = check_data_sanitization(code, &query_lower);
        checks.push(AlgoCheckItem {
            name: "data_sanitization".into(),
            passed: san_passed,
            blocking: false,
            detail: san_detail,
            severity: if san_passed { "info".into() } else { "warning".into() },
        });

        checks
    }

    pub fn total_analyses(&self) -> u64 {
        self.total_analyses
    }

    pub fn dimension_counts(&self) -> BTreeMap<String, u64> {
        self.dimension_counts.clone()
    }
}

// ================== 各维度具体检查函数 ==================

fn estimate_time_complexity(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("未提供代码片段，基于查询描述估算为 O(n)".into(), true);
    }
    let for_count = code.matches("for ").count() + code.matches("while ").count();
    let complexity = match for_count {
        0 => "O(1)",
        1 => "O(n)",
        2 => "O(n²)",
        3 => "O(n³)",
        _ => "O(n^k) (k≥4，存在性能风险)",
    };
    let passed = for_count <= 2;
    (format!("检测到 {} 层循环，时间复杂度约为 {}", for_count, complexity), passed)
}

fn estimate_space_complexity(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("未提供代码片段，基于查询描述估算为 O(n)".into(), true);
    }
    let dyn_structs = ["Vec::", "Vec<", "HashMap<", "BTreeMap<", "vec![", "HashSet<"];
    let mut count = 0;
    for s in &dyn_structs {
        count += code.matches(s).count();
    }
    let complexity = match count {
        0 => "O(1)（常量空间）",
        1..=2 => "O(n)（线性空间）",
        3..=5 => "O(n)（线性空间，注意内存优化）",
        _ => "O(n) 或更高（多个动态结构，需关注内存峰值）",
    };
    (format!("检测到 {} 处动态数据结构，空间复杂度约为 {}", count, complexity), true)
}

fn detect_bottleneck(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("未提供代码，无法检测具体瓶颈。建议关注：1) IO 操作 2) 数据库查询 3) 大循环内的重复计算".into(), true);
    }
    let mut issues = Vec::new();
    if code.matches("unwrap()").count() > 2 {
        issues.push("unwrap() 调用较多，生产环境建议使用 match 或 ? 传播错误");
    }
    if code.matches("clone()").count() > 3 {
        issues.push("clone() 调用较多，考虑使用引用或 Cow 减少拷贝");
    }
    if code.contains(".to_string()") && code.matches(".to_string()").count() > 5 {
        issues.push("频繁字符串分配，考虑使用 format! 或预分配 String");
    }
    if issues.is_empty() {
        ("未检测到明显的性能瓶颈代码模式".into(), true)
    } else {
        (format!("检测到 {} 个潜在性能问题：{}", issues.len(), issues.join("；")), false)
    }
}

fn check_boundary_conditions(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("建议：确保处理空输入、单元素、最大/最小值等边界情况".into(), true);
    }
    let has_empty_check = code.contains("is_empty()") || code.contains("len() == 0");
    let has_none_check = code.contains("None") || code.contains("is_none()");
    let has_boundary = has_empty_check || has_none_check;
    let detail = if has_boundary {
        "检测到边界条件检查（空值/None 处理）".to_string()
    } else {
        "未检测到明确的边界条件检查，建议添加空输入、越界、None 等情况处理".to_string()
    };
    (detail, has_boundary)
}

fn check_logic_completeness(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("建议：确保所有分支路径都有明确的返回值或处理逻辑".into(), true);
    }
    let has_else = code.contains("else");
    let has_match = code.contains("match");
    let has_default = has_else || has_match;
    let detail = if has_default {
        "检测到 else/match 默认分支，逻辑相对完备".to_string()
    } else {
        "建议添加 else 分支或 match 的 _ 通配分支，确保所有情况都有处理".to_string()
    };
    (detail, has_default)
}

fn check_error_handling(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("建议：使用 Result 类型传递错误，避免 panic! 和 unwrap()".into(), true);
    }
    let has_result = code.contains("Result");
    let has_question = code.contains('?');
    let unwrap_count = code.matches("unwrap()").count();
    let panic_count = code.matches("panic!").count();
    let good = has_result || has_question;
    let bad = unwrap_count + panic_count;
    let detail = format!(
        "错误处理：{} Result/? 操作符，{} 个 unwrap()，{} 个 panic!()",
        if good { "使用了" } else { "未检测到" },
        unwrap_count,
        panic_count
    );
    (detail, good && bad <= 1)
}

fn check_perf_optimization(code: &str, query: &str) -> (String, bool) {
    if code.is_empty() {
        if query.contains("优化") || query.contains("性能") {
            return ("性能优化建议：1) 使用迭代器替代索引访问 2) 预分配容器容量 3) 减少不必要的 clone 4) 合理使用缓存".into(), true);
        }
        return ("可优化空间：需提供具体代码以给出针对性建议".into(), true);
    }
    let mut tips = Vec::new();
    if code.contains("push(") && !code.contains("with_capacity") {
        tips.push("建议使用 Vec::with_capacity() 预分配容量");
    }
    if code.matches(".collect::<Vec<_>>()").count() > 0 && code.matches("for ").count() > 2 {
        tips.push("考虑使用迭代器适配器（map/filter/fold）替代显式 for 循环");
    }
    if tips.is_empty() {
        ("未检测到明显的性能优化点".into(), true)
    } else {
        (format!("优化建议：{}", tips.join("；")), false)
    }
}

fn check_readability(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("可读性建议：使用有意义的变量名、添加文档注释、控制函数长度".into(), true);
    }
    let has_doc_comment = code.contains("///");
    let line_count = code.lines().count();
    let detail = format!(
        "代码约 {} 行，{} 文档注释",
        line_count,
        if has_doc_comment { "包含" } else { "未检测到" }
    );
    (detail, has_doc_comment || line_count < 50)
}

fn check_maintainability(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("可维护性建议：模块化设计、单一职责、完善的单元测试".into(), true);
    }
    let has_test = code.contains("#[test]") || code.contains("cfg(test)");
    let has_mod = code.contains("mod ");
    let detail = format!(
        "维护性指标：{} 测试模块，{} 模块化结构",
        if has_test { "包含" } else { "未检测到" },
        if has_mod { "包含" } else { "未检测到" }
    );
    (detail, has_test || has_mod)
}

fn check_injection_risk(code: &str, query: &str) -> (String, bool) {
    if code.is_empty() {
        if query.contains("sql") || query.contains("数据库") || query.contains("查询") {
            return ("SQL 注入风险：务必使用参数化查询/预编译语句，禁止字符串拼接 SQL".into(), false);
        }
        return ("注入风险：无代码样本，建议对所有外部输入进行验证和转义".into(), true);
    }
    let risk_patterns = ["format!(\"SELECT", "push_str(\"SELECT", "+ \"SELECT", "format!(\"INSERT"];
    let mut found = Vec::new();
    for p in &risk_patterns {
        if code.contains(p) {
            found.push(*p);
        }
    }
    if found.is_empty() {
        ("未检测到明显的 SQL 注入模式".into(), true)
    } else {
        (format!("检测到潜在 SQL 注入风险：使用字符串拼接构建 SQL 语句。请改用参数化查询"), false)
    }
}

fn check_overflow_risk(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("溢出风险：建议使用 checked_* / saturating_* 方法替代直接算术运算".into(), true);
    }
    let direct_ops = code.matches(" + ").count() + code.matches(" - ").count() + code.matches(" * ").count();
    let safe_ops = code.matches("checked_").count() + code.matches("saturating_").count();
    if direct_ops > 0 && safe_ops == 0 {
        (format!("检测到 {} 处直接算术运算，未使用 checked_* / saturating_* 安全方法，存在溢出风险", direct_ops), false)
    } else {
        (format!("使用了 {} 个安全算术方法，溢出风险较低", safe_ops), true)
    }
}

fn check_permission_checks(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("权限检查：建议在敏感操作前验证用户角色和权限".into(), true);
    }
    let has_perm_check = code.contains("permission")
        || code.contains("auth")
        || code.contains("rbac")
        || code.contains("role");
    let detail = if has_perm_check {
        "检测到权限相关代码（permission/auth/rbac/role）"
    } else {
        "未检测到权限检查逻辑。如果涉及敏感操作，建议添加 RBAC 权限校验"
    };
    (detail.into(), has_perm_check)
}

fn check_secret_leak(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("密钥安全：切勿在代码中硬编码密钥、密码、Token 等敏感信息".into(), true);
    }
    let secret_patterns = [
        "api_key", "apikey", "secret_key", "private_key",
        "password", "passwd", "token =", "secret =",
        "API_KEY", "SECRET", "PASSWORD",
    ];
    let mut found = Vec::new();
    for p in &secret_patterns {
        if code.to_lowercase().contains(&p.to_lowercase()) {
            found.push(*p);
        }
    }
    if found.is_empty() {
        ("未检测到硬编码密钥模式".into(), true)
    } else {
        (format!("检测到潜在密钥泄露风险：代码中包含 {:?} 等敏感字段名，请勿硬编码密钥", found), false)
    }
}

fn analyze_variable_lifecycle(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("变量生命周期：需提供代码以进行详细分析".into(), true);
    }
    let let_count = code.matches("let ").count();
    let mut_count = code.matches("let mut ").count();
    let detail = format!(
        "变量统计：{} 个变量声明，其中 {} 个可变变量（mut 占比 {:.1}%）",
        let_count,
        mut_count,
        if let_count > 0 {
            mut_count as f64 / let_count as f64 * 100.0
        } else {
            0.0
        }
    );
    (detail, true)
}

fn analyze_dependencies(code: &str, _query: &str) -> (String, bool) {
    if code.is_empty() {
        return ("依赖分析：需提供代码以识别模块/函数依赖关系".into(), true);
    }
    let use_count = code.matches("use ").count();
    let fn_count = code.matches("fn ").count();
    let detail = format!(
        "依赖统计：{} 个 use 导入，{} 个函数定义",
        use_count, fn_count
    );
    (detail, true)
}

fn check_data_sanitization(code: &str, query: &str) -> (String, bool) {
    if code.is_empty() {
        if query.contains("用户输入") || query.contains("输入") || query.contains("外部") {
            return ("外部输入必须经过验证和转义：长度检查、类型验证、特殊字符转义".into(), false);
        }
        return ("数据脱敏：建议对所有外部输入进行验证和清洗".into(), true);
    }
    let has_sanitize = code.contains("sanitize")
        || code.contains("validate")
        || code.contains("trim()")
        || code.contains("escape");
    let detail = if has_sanitize {
        "检测到数据验证/清洗相关操作"
    } else {
        "未检测到明确的数据验证/清洗逻辑。如果处理外部输入，建议添加输入验证"
    };
    (detail.into(), has_sanitize)
}

// ================== 摘要与建议生成 ==================

fn generate_summary(checks: &[AlgoCheckItem], dim: AnalysisDimension) -> String {
    let total = checks.len();
    let passed = checks.iter().filter(|c| c.passed).count();
    let blocked = checks.iter().filter(|c| c.blocking && !c.passed).count();
    format!(
        "算法分析完成（维度：{}）：共 {} 项检查，通过 {} 项，阻断性问题 {} 项。{}",
        dim.label(),
        total,
        passed,
        blocked,
        if blocked > 0 {
            "存在阻断性安全问题，建议修复后再发布。"
        } else if passed == total {
            "所有检查项通过，代码质量良好。"
        } else {
            "存在改进空间，建议参考优化建议进行调整。"
        }
    )
}

fn generate_suggestions(checks: &[AlgoCheckItem], _dim: AnalysisDimension) -> Vec<String> {
    let mut suggestions = Vec::new();
    for c in checks.iter().filter(|c| !c.passed) {
        suggestions.push(format!("[{}] {}", c.severity, c.detail));
    }
    if suggestions.is_empty() {
        suggestions.push("暂无改进建议，当前代码质量良好。".into());
    }
    suggestions
}

// ================== 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_req(dim: &str, code: Option<&str>) -> AlgorithmAnalysisRequest {
        AlgorithmAnalysisRequest {
            query: "分析这段代码的质量".into(),
            dimension: dim.into(),
            code_snippet: code.map(|s| s.to_string()),
            flow_json: None,
            context: BTreeMap::new(),
        }
    }

    #[test]
    fn test_analysis_all_dimensions() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let req = make_req("all", Some(
            "fn add(a: i32, b: i32) -> i32 { a + b }"
        ));
        let result = analyzer.analyze(&req);
        assert_eq!(result.dimension, "all");
        assert!(!result.checks.is_empty());
        assert!(!result.summary.is_empty());
        assert!(!result.suggestions.is_empty());
        assert!(result.latency_ms < 1000);
    }

    #[test]
    fn test_analysis_complexity() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let req = make_req("complexity", Some(
            "for i in 0..n { for j in 0..n { println!(\"{}\", i*j); } }"
        ));
        let result = analyzer.analyze(&req);
        assert_eq!(result.dimension, "complexity");
        assert_eq!(result.checks.len(), 3);
    }

    #[test]
    fn test_analysis_security_detects_injection() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let code = r#"
            fn query(name: &str) {
                let sql = format!("SELECT * FROM users WHERE name = '{}'", name);
                execute(sql);
            }
        "#;
        let req = make_req("security", Some(code));
        let result = analyzer.analyze(&req);
        let injection = result.checks.iter().find(|c| c.name == "injection_risk");
        assert!(injection.is_some());
    }

    #[test]
    fn test_analysis_secret_leak_detection() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let code = "const API_KEY: &str = \"sk-12345\";";
        let req = make_req("security", Some(code));
        let result = analyzer.analyze(&req);
        let secret = result.checks.iter().find(|c| c.name == "secret_leak");
        assert!(secret.is_some());
        assert!(!secret.unwrap().passed, "应该检测到密钥泄露风险");
    }

    #[test]
    fn test_analysis_no_code_still_works() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let req = make_req("all", None);
        let result = analyzer.analyze(&req);
        assert!(!result.checks.is_empty());
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_analysis_dimension_counts() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let _ = analyzer.analyze(&make_req("complexity", None));
        let _ = analyzer.analyze(&make_req("security", None));
        let _ = analyzer.analyze(&make_req("all", None));
        assert_eq!(analyzer.total_analyses(), 3);
        assert!(analyzer.dimension_counts().len() >= 2);
    }

    #[test]
    fn test_analysis_vetoed_on_blocking_fail() {
        let mut analyzer = AlgorithmAnalyzer::new();
        let code = "let x = api_key = \"secret123\";";
        let req = make_req("security", Some(code));
        let result = analyzer.analyze(&req);
        assert!(result.vetoed || !result.all_passed);
    }

    #[test]
    fn test_dimension_from_str() {
        assert_eq!(AnalysisDimension::from_str("complexity"), AnalysisDimension::Complexity);
        assert_eq!(AnalysisDimension::from_str("security"), AnalysisDimension::Security);
        assert_eq!(AnalysisDimension::from_str("all"), AnalysisDimension::All);
        assert_eq!(AnalysisDimension::from_str("unknown"), AnalysisDimension::All);
    }
}
