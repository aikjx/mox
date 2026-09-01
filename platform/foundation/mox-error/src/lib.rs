// =============================================================================
// MOX 全局错误码系统
// =============================================================================
// 设计原则：
// 1. 每个错误有唯一的字符串错误码（如 "KG01001"），便于跨系统追踪
// 2. 错误分为不同层级（Info / Warning / Error / Critical）
// 3. 每个错误携带 trace_id，支持分布式追踪
// 4. 统一的 JSON 响应格式，前端可直接解析
// 5. 支持错误链（source），保留原始错误上下文
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// =============================================================================
// 错误等级
// =============================================================================

/// 错误严重等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorLevel {
    /// 信息级 - 不影响主流程，仅作提示
    Info,
    /// 警告级 - 可能影响部分功能，系统仍可运行
    Warning,
    /// 错误级 - 功能异常，需要处理
    Error,
    /// 严重级 - 系统级故障，需要立即介入
    Critical,
}

impl fmt::Display for ErrorLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorLevel::Info => write!(f, "INFO"),
            ErrorLevel::Warning => write!(f, "WARN"),
            ErrorLevel::Error => write!(f, "ERROR"),
            ErrorLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

// =============================================================================
// 错误域代码
// =============================================================================

/// 业务域代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorDomain {
    /// 知识图谱域
    Kg,
    /// AI 能力域
    Ai,
    /// 工作流域
    Flow,
    /// 算子引擎域
    Operator,
    /// 项目管理域
    Project,
    /// 资源中心域
    Resource,
    /// 用户/权限域
    User,
    /// 平台/系统域
    Platform,
    /// 数据处理域
    Data,
    /// 云存储域
    Cloud,
}

impl Default for ErrorDomain {
    fn default() -> Self {
        ErrorDomain::Platform
    }
}

impl ErrorDomain {
    pub fn code(&self) -> &'static str {
        match self {
            ErrorDomain::Kg => "KG",
            ErrorDomain::Ai => "AI",
            ErrorDomain::Flow => "FL",
            ErrorDomain::Operator => "OP",
            ErrorDomain::Project => "PJ",
            ErrorDomain::Resource => "RS",
            ErrorDomain::User => "US",
            ErrorDomain::Platform => "PL",
            ErrorDomain::Data => "DT",
            ErrorDomain::Cloud => "CL",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ErrorDomain::Kg => "知识图谱",
            ErrorDomain::Ai => "AI 能力",
            ErrorDomain::Flow => "工作流",
            ErrorDomain::Operator => "算子引擎",
            ErrorDomain::Project => "项目管理",
            ErrorDomain::Resource => "资源中心",
            ErrorDomain::User => "用户权限",
            ErrorDomain::Platform => "平台系统",
            ErrorDomain::Data => "数据处理",
            ErrorDomain::Cloud => "云存储",
        }
    }
}

// =============================================================================
// 统一错误结构体
// =============================================================================

/// MOX 平台统一错误类型
///
/// 错误码格式：{域代码}{模块代码:02d}{序号:03d}
/// 例如：KG01001 = 知识图谱域·存储模块·第1号错误
#[derive(Debug, Serialize, Deserialize)]
pub struct MoxError {
    /// 错误码（如 "KG01001"）
    pub code: String,
    /// 错误消息（面向用户的可读消息）
    pub message: String,
    /// 错误详情（面向开发者的详细信息）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 错误等级
    pub level: ErrorLevel,
    /// HTTP 状态码
    #[serde(skip)]
    pub http_status: u16,
    /// 追踪 ID
    pub trace_id: String,
    /// 时间戳
    pub timestamp: String,
    /// 业务域
    #[serde(skip)]
    pub domain: ErrorDomain,
    /// 错误来源（原始错误）
    #[serde(skip)]
    pub source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl MoxError {
    // =========================================================================
    // 构造器
    // =========================================================================

    /// 创建新错误
    pub fn new(
        domain: ErrorDomain,
        module: u8,
        seq: u16,
        message: impl Into<String>,
        level: ErrorLevel,
        http_status: u16,
    ) -> Self {
        let code = format!("{}{:02}{:03}", domain.code(), module, seq);
        Self {
            code,
            message: message.into(),
            detail: None,
            level,
            http_status,
            trace_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            domain,
            source: None,
        }
    }

    /// 添加详情信息
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// 添加来源错误
    pub fn with_source<E>(mut self, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.source = Some(Box::new(source));
        self
    }

    /// 设置追踪 ID
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    // =========================================================================
    // 通用错误快捷方法
    // =========================================================================

    /// 参数校验错误 (400)
    pub fn bad_request(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Error, 400)
    }

    /// 未认证错误 (401)
    pub fn unauthorized(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Warning, 401)
    }

    /// 无权限错误 (403)
    pub fn forbidden(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Warning, 403)
    }

    /// 资源不存在 (404)
    pub fn not_found(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Warning, 404)
    }

    /// 资源冲突 (409)
    pub fn conflict(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Warning, 409)
    }

    /// 业务规则校验失败 (422)
    pub fn unprocessable(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Error, 422)
    }

    /// 限流错误 (429)
    pub fn too_many_requests(domain: ErrorDomain, module: u8, seq: u16) -> Self {
        Self::new(domain, module, seq, "请求过于频繁，请稍后再试", ErrorLevel::Warning, 429)
    }

    /// 内部服务器错误 (500)
    pub fn internal(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Error, 500)
    }

    /// 服务不可用 (503)
    pub fn unavailable(domain: ErrorDomain, module: u8, seq: u16, message: impl Into<String>) -> Self {
        Self::new(domain, module, seq, message, ErrorLevel::Critical, 503)
    }

    // =========================================================================
    // 平台级通用错误（域=Platform）
    // =========================================================================

    /// 未知错误（兜底）
    pub fn unknown() -> Self {
        Self::internal(ErrorDomain::Platform, 00, 999, "系统内部错误，请稍后重试")
    }

    /// 服务暂不可用
    pub fn service_unavailable() -> Self {
        Self::unavailable(ErrorDomain::Platform, 00, 001, "服务暂不可用，请稍后重试")
    }

    /// 请求超时
    pub fn timeout() -> Self {
        Self::new(
            ErrorDomain::Platform,
            00,
            002,
            "请求超时，请稍后重试",
            ErrorLevel::Warning,
            504,
        )
    }
}

// =============================================================================
// Trait 实现
// =============================================================================

impl fmt::Display for MoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} - {}", self.code, self.level, self.message)
    }
}

impl std::error::Error for MoxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

// =============================================================================
// Axum 集成：转换为 HTTP 响应
// =============================================================================

#[cfg(feature = "axum-integration")]
impl axum::response::IntoResponse for MoxError {
    fn into_response(self) -> axum::response::Response {
        let status = axum::http::StatusCode::from_u16(self.http_status)
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        let body = serde_json::json!({
            "success": false,
            "error": {
                "code": self.code,
                "message": self.message,
                "detail": self.detail,
                "level": self.level,
                "trace_id": self.trace_id,
                "timestamp": self.timestamp,
            }
        });

        // 记录错误日志
        match self.level {
            ErrorLevel::Info => tracing::info!(error_code = %self.code, trace_id = %self.trace_id, "{}", self.message),
            ErrorLevel::Warning => tracing::warn!(error_code = %self.code, trace_id = %self.trace_id, "{}", self.message),
            ErrorLevel::Error => tracing::error!(error_code = %self.code, trace_id = %self.trace_id, "{}", self.message),
            ErrorLevel::Critical => tracing::error!(error_code = %self.code, trace_id = %self.trace_id, "CRITICAL ERROR: {}", self.message),
        }

        (status, axum::Json(body)).into_response()
    }
}

// =============================================================================
// 便捷类型别名
// =============================================================================

/// MOX 统一结果类型
pub type MoxResult<T> = Result<T, MoxError>;

// =============================================================================
// 各域错误码定义宏
// =============================================================================

/// 为特定域定义错误码常量的宏
///
/// # 示例
/// ```rust,ignore
/// use mox_error::define_domain_errors;
/// define_domain_errors!(KgError, Kg,
///     NODE_NOT_FOUND: (1, 1, "节点不存在", 404, Warning),
///     DUPLICATE_NODE: (1, 2, "节点已存在", 409, Warning),
/// );
/// ```
// 错误码采用"域(module) + 序号(seq)"两段式设计（如 01-001），
// 前导零是故意的格式（非八进制），此处统一豁免 zero_prefixed_literal。
#[allow(clippy::zero_prefixed_literal)]
#[macro_export]
macro_rules! define_domain_errors {
    ($struct_name:ident, $domain:ident,
        $($name:ident: ($module:expr, $seq:expr, $msg:expr, $http:expr, $level:ident)),*
        $(,)?
    ) => {
        pub struct $struct_name;

        // 错误码方法名采用大写常量式命名（如 `NODE_NOT_FOUND`）是错误码库的惯例，
        // 显式 allow(non_snake_case) 表明这是有意的 API 设计，而非命名错误。
        #[allow(non_snake_case)]
        impl $struct_name {
            $(
                pub fn $name() -> $crate::MoxError {
                    $crate::MoxError::new(
                        $crate::ErrorDomain::$domain,
                        $module,
                        $seq,
                        $msg,
                        $crate::ErrorLevel::$level,
                        $http,
                    )
                }
            )*
        }
    };
}

// =============================================================================
// 知识图谱域错误码
// =============================================================================

/// KG 域错误码
pub mod kg {

    define_domain_errors!(KgErrors, Kg,
        // 存储模块 (01)
        NODE_NOT_FOUND:       (01, 001, "节点不存在", 404, Warning),
        EDGE_NOT_FOUND:       (01, 002, "关系不存在", 404, Warning),
        NODE_ALREADY_EXISTS:  (01, 003, "节点已存在", 409, Warning),
        EDGE_ALREADY_EXISTS:  (01, 004, "关系已存在", 409, Warning),
        STORAGE_ERROR:        (01, 099, "存储操作失败", 500, Error),

        // 算法模块 (02)
        ALGORITHM_ERROR:      (02, 001, "图算法执行失败", 500, Error),
        GRAPH_EMPTY:          (02, 002, "图谱为空，无法执行算法", 422, Warning),
        INVALID_PARAMS:       (02, 003, "算法参数无效", 400, Warning),

        // 元数据模块 (03)
        META_NOT_FOUND:       (03, 001, "元数据不存在", 404, Warning),
        SCHEMA_CONFLICT:      (03, 002, "Schema 冲突", 409, Warning),
    );
}

// =============================================================================
// AI 能力域错误码
// =============================================================================

/// AI 域错误码
pub mod ai {

    define_domain_errors!(AiErrors, Ai,
        // 对话模块 (01)
        CONVERSATION_NOT_FOUND:  (01, 001, "会话不存在", 404, Warning),
        EMPTY_MESSAGE:           (01, 002, "消息内容不能为空", 400, Warning),
        CONTEXT_TOO_LONG:        (01, 003, "上下文长度超限", 422, Warning),

        // LLM 模块 (02)
        PROVIDER_NOT_FOUND:      (02, 001, "模型提供商不存在", 404, Warning),
        MODEL_NOT_FOUND:         (02, 002, "模型不存在", 404, Warning),
        LLM_REQUEST_FAILED:      (02, 099, "大模型请求失败", 500, Error),
        RATE_LIMITED:            (02, 003, "模型调用频率超限", 429, Warning),

        // Agent 模块 (03)
        AGENT_NOT_FOUND:         (03, 001, "Agent 不存在", 404, Warning),
        TOOL_CALL_FAILED:        (03, 002, "工具调用失败", 500, Error),
        AGENT_TIMEOUT:           (03, 003, "Agent 执行超时", 504, Error),
    );
}

// =============================================================================
// 用户/权限域错误码
// =============================================================================

/// 用户权限域错误码
pub mod user {

    define_domain_errors!(UserErrors, User,
        // 认证模块 (01)
        TOKEN_MISSING:         (01, 001, "缺少认证 Token", 401, Warning),
        TOKEN_INVALID:         (01, 002, "Token 无效或已过期", 401, Warning),
        LOGIN_FAILED:          (01, 003, "用户名或密码错误", 401, Warning),
        ACCOUNT_DISABLED:      (01, 004, "账号已被禁用", 403, Warning),

        // 权限模块 (02)
        PERMISSION_DENIED:     (02, 001, "无权限执行此操作", 403, Warning),
        ROLE_NOT_FOUND:        (02, 002, "角色不存在", 404, Warning),
    );
}

// =============================================================================
// 平台级错误码
// =============================================================================

/// 平台域错误码
pub mod platform {

    define_domain_errors!(PlatformErrors, Platform,
        // 通用模块 (00)
        UNKNOWN_ERROR:         (00, 999, "系统内部错误", 500, Error),
        SERVICE_UNAVAILABLE:   (00, 001, "服务暂不可用", 503, Critical),
        TIMEOUT:               (00, 002, "请求超时", 504, Warning),
        RATE_LIMITED:          (00, 003, "请求过于频繁", 429, Warning),

        // 配置模块 (01)
        CONFIG_ERROR:          (01, 001, "配置错误", 500, Error),

        // 验证模块 (02)
        VALIDATION_ERROR:      (02, 001, "参数验证失败", 400, Warning),
    );
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_format() {
        let err = MoxError::not_found(ErrorDomain::Kg, 01, 001, "节点不存在");
        assert_eq!(err.code, "KG01001");
        assert_eq!(err.http_status, 404);
        assert_eq!(err.level, ErrorLevel::Warning);
    }

    #[test]
    fn test_kg_error_macro() {
        let err = kg::KgErrors::NODE_NOT_FOUND();
        assert_eq!(err.code, "KG01001");
        assert_eq!(err.http_status, 404);
    }

    #[test]
    fn test_ai_error_macro() {
        let err = ai::AiErrors::LLM_REQUEST_FAILED();
        assert_eq!(err.code, "AI02099");
        assert_eq!(err.http_status, 500);
    }

    #[test]
    fn test_error_display() {
        let err = MoxError::bad_request(ErrorDomain::Ai, 01, 001, "参数错误");
        let display = format!("{}", err);
        assert!(display.contains("AI01001"));
        assert!(display.contains("ERROR"));
    }

    #[test]
    fn test_with_detail() {
        let err = MoxError::internal(ErrorDomain::Platform, 00, 001, "内部错误")
            .with_detail("详细堆栈信息");
        assert_eq!(err.detail.unwrap(), "详细堆栈信息");
    }

    #[test]
    fn test_all_domains_have_unique_prefix() {
        let domains = [
            (ErrorDomain::Kg, "KG"),
            (ErrorDomain::Ai, "AI"),
            (ErrorDomain::Flow, "FL"),
            (ErrorDomain::Operator, "OP"),
            (ErrorDomain::Project, "PJ"),
            (ErrorDomain::Resource, "RS"),
            (ErrorDomain::User, "US"),
            (ErrorDomain::Platform, "PL"),
            (ErrorDomain::Data, "DT"),
            (ErrorDomain::Cloud, "CL"),
        ];
        let prefixes: Vec<&str> = domains.iter().map(|(d, _)| d.code()).collect();
        // 确保没有重复的前缀
        for i in 0..prefixes.len() {
            for j in (i + 1)..prefixes.len() {
                assert_ne!(prefixes[i], prefixes[j], "域代码重复: {}", prefixes[i]);
            }
        }
    }
}
