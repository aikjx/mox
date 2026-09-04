// =============================================================================
// 统一错误码体系
// =============================================================================
// 错误码格式：{域代码:2}{模块代码:02}{序号:03}
// 例如：AI01001 = AI域·对话模块·第1号错误
//
// 跨端对齐：Python 和 前端必须使用相同的错误码格式和语义。
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
// 错误域代码（SSOT - 所有域必须在此注册）
// =============================================================================

/// 业务域代码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorDomain {
    /// 平台系统域
    Platform,
    /// AI 能力域
    Ai,
    /// 专家联盟域
    Alliance,
    /// 知识图谱域
    Kg,
    /// 云盘/知识库域
    Cloud,
    /// 工作流域
    Flow,
    /// 数据处理域
    Data,
    /// 项目管理域
    Project,
    /// 资源中心域
    Resource,
    /// 用户/权限域
    User,
    /// 市场/模板域
    Market,
    /// 语音域
    Voice,
    /// 算子引擎域
    Operator,
}

impl ErrorDomain {
    /// 域代码（2位大写字母）
    pub fn code(&self) -> &'static str {
        match self {
            ErrorDomain::Platform => "PL",
            ErrorDomain::Ai => "AI",
            ErrorDomain::Alliance => "AL",
            ErrorDomain::Kg => "KG",
            ErrorDomain::Cloud => "CL",
            ErrorDomain::Flow => "FL",
            ErrorDomain::Data => "DT",
            ErrorDomain::Project => "PJ",
            ErrorDomain::Resource => "RS",
            ErrorDomain::User => "US",
            ErrorDomain::Market => "MK",
            ErrorDomain::Voice => "VC",
            ErrorDomain::Operator => "OP",
        }
    }

    /// 域中文名
    pub fn name(&self) -> &'static str {
        match self {
            ErrorDomain::Platform => "平台系统",
            ErrorDomain::Ai => "AI 能力",
            ErrorDomain::Alliance => "专家联盟",
            ErrorDomain::Kg => "知识图谱",
            ErrorDomain::Cloud => "云盘知识库",
            ErrorDomain::Flow => "工作流",
            ErrorDomain::Data => "数据处理",
            ErrorDomain::Project => "项目管理",
            ErrorDomain::Resource => "资源中心",
            ErrorDomain::User => "用户权限",
            ErrorDomain::Market => "市场模板",
            ErrorDomain::Voice => "语音",
            ErrorDomain::Operator => "算子引擎",
        }
    }

    /// 从代码字符串解析域
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_uppercase().as_str() {
            "PL" => Some(ErrorDomain::Platform),
            "AI" => Some(ErrorDomain::Ai),
            "AL" => Some(ErrorDomain::Alliance),
            "KG" => Some(ErrorDomain::Kg),
            "CL" => Some(ErrorDomain::Cloud),
            "FL" => Some(ErrorDomain::Flow),
            "DT" => Some(ErrorDomain::Data),
            "PJ" => Some(ErrorDomain::Project),
            "RS" => Some(ErrorDomain::Resource),
            "US" => Some(ErrorDomain::User),
            "MK" => Some(ErrorDomain::Market),
            "VC" => Some(ErrorDomain::Voice),
            "OP" => Some(ErrorDomain::Operator),
            _ => None,
        }
    }
}

impl Default for ErrorDomain {
    fn default() -> Self {
        ErrorDomain::Platform
    }
}

impl fmt::Display for ErrorDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

// =============================================================================
// 错误码（值对象）
// =============================================================================

/// 错误码值对象
///
/// 格式：{域代码}{模块代码:02}{序号:03}
/// 例如：AI01001
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorCode {
    /// 域代码
    pub domain: ErrorDomain,
    /// 模块代码（0-99）
    pub module: u8,
    /// 序号（0-999）
    pub seq: u16,
}

impl ErrorCode {
    /// 创建错误码
    pub fn new(domain: ErrorDomain, module: u8, seq: u16) -> Self {
        Self { domain, module, seq }
    }

    /// 格式化为字符串
    pub fn as_str(&self) -> String {
        format!("{}{:02}{:03}", self.domain.code(), self.module, self.seq)
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() != 7 {
            return None;
        }
        let domain = ErrorDomain::from_code(&s[0..2])?;
        let module: u8 = s[2..4].parse().ok()?;
        let seq: u16 = s[4..7].parse().ok()?;
        Some(Self { domain, module, seq })
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// 统一错误结构体
// =============================================================================

/// MOX 平台统一错误类型
///
/// 跨端对齐：Python 和 前端必须使用相同的 JSON 结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoxError {
    /// 错误码（如 "AI01001"）
    pub code: String,
    /// 错误消息（面向用户的可读消息，中文）
    pub message: String,
    /// 错误详情（面向开发者的详细信息，生产环境可省略）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 错误等级
    pub level: ErrorLevel,
    /// HTTP 状态码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// 追踪 ID（全链路透传）
    pub trace_id: String,
    /// 时间戳（ISO-8601）
    pub timestamp: String,
    /// 业务域
    #[serde(skip)]
    pub domain: ErrorDomain,
}

impl MoxError {
    /// 创建新错误
    pub fn new(
        domain: ErrorDomain,
        module: u8,
        seq: u16,
        message: impl Into<String>,
        level: ErrorLevel,
        http_status: u16,
    ) -> Self {
        let code = ErrorCode::new(domain, module, seq);
        Self {
            code: code.as_str(),
            message: message.into(),
            detail: None,
            level,
            http_status: Some(http_status),
            trace_id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            domain,
        }
    }

    /// 添加详情信息
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// 设置追踪 ID
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    // ── 快捷构造器 ──────────────────────────────────────────────────────

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

    /// 未知错误（兜底）
    pub fn unknown() -> Self {
        Self::internal(ErrorDomain::Platform, 0, 999, "系统内部错误，请稍后重试")
    }
}

impl fmt::Display for MoxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} - {}", self.code, self.level, self.message)
    }
}

impl std::error::Error for MoxError {}

/// MOX 统一结果类型
pub type MoxResult<T> = Result<T, MoxError>;

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_format() {
        let code = ErrorCode::new(ErrorDomain::Ai, 1, 1);
        assert_eq!(code.as_str(), "AI01001");
    }

    #[test]
    fn error_code_parse_roundtrip() {
        let code = ErrorCode::new(ErrorDomain::Alliance, 5, 123);
        let s = code.as_str();
        let parsed = ErrorCode::parse(&s).unwrap();
        assert_eq!(code, parsed);
    }

    #[test]
    fn all_domains_have_unique_codes() {
        let domains = [
            ErrorDomain::Platform,
            ErrorDomain::Ai,
            ErrorDomain::Alliance,
            ErrorDomain::Kg,
            ErrorDomain::Cloud,
            ErrorDomain::Flow,
            ErrorDomain::Data,
            ErrorDomain::Project,
            ErrorDomain::Resource,
            ErrorDomain::User,
            ErrorDomain::Market,
            ErrorDomain::Voice,
            ErrorDomain::Operator,
        ];
        let codes: Vec<&str> = domains.iter().map(|d| d.code()).collect();
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i], codes[j], "域代码重复: {}", codes[i]);
            }
        }
    }

    #[test]
    fn mox_error_display() {
        let err = MoxError::not_found(ErrorDomain::Ai, 1, 1, "会话不存在");
        let display = format!("{}", err);
        assert!(display.contains("AI01001"));
        assert!(display.contains("会话不存在"));
    }

    #[test]
    fn mox_error_serialization() {
        let err = MoxError::bad_request(ErrorDomain::Platform, 1, 1, "参数错误")
            .with_detail("详细信息");
        let json = serde_json::to_string(&err).unwrap();
        let parsed: MoxError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, "PL01001");
        assert_eq!(parsed.message, "参数错误");
        assert_eq!(parsed.detail, Some("详细信息".to_string()));
    }
}
