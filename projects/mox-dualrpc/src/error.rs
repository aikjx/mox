//! Unified error mapping between gRPC Status and JSON-RPC 2.0 error codes.

use serde::{Deserialize, Serialize};
use std::fmt;
use tonic::Code;

/// JSON-RPC 2.0 standard error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonRpcErrorCode {
    /// -32700: Parse error
    ParseError,
    /// -32600: Invalid Request
    InvalidRequest,
    /// -32601: Method not found
    MethodNotFound,
    /// -32602: Invalid params
    InvalidParams,
    /// -32603: Internal error
    InternalError,
    /// -32000 to -32099: Server error (reserved for implementation-defined)
    ServerError(i32),
    /// Custom error code
    Custom(i32),
}

impl JsonRpcErrorCode {
    pub fn code(&self) -> i32 {
        match self {
            Self::ParseError => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::InternalError => -32603,
            Self::ServerError(c) => *c,
            Self::Custom(c) => *c,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::ParseError => "Parse error",
            Self::InvalidRequest => "Invalid Request",
            Self::MethodNotFound => "Method not found",
            Self::InvalidParams => "Invalid params",
            Self::InternalError => "Internal error",
            Self::ServerError(_) => "Server error",
            Self::Custom(_) => "Custom error",
        }
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: JsonRpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.code(),
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn method_not_found(method: &str) -> Self {
        Self::new(
            JsonRpcErrorCode::MethodNotFound,
            format!("Method '{}' not found", method),
        )
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(JsonRpcErrorCode::InvalidParams, msg)
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::new(JsonRpcErrorCode::InternalError, msg)
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::new(JsonRpcErrorCode::ParseError, msg)
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

/// Unified dual-protocol error
#[derive(Debug, thiserror::Error)]
pub enum DualRpcError {
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] JsonRpcError),

    #[error("Transcode error: {0}")]
    Transcode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Other: {0}")]
    Other(String),
}

/// Trait for converting errors to gRPC Status
pub trait ToStatus {
    fn to_status(&self) -> tonic::Status;
}

impl ToStatus for DualRpcError {
    fn to_status(&self) -> tonic::Status {
        match self {
            Self::Grpc(status) => status.clone(),
            Self::JsonRpc(e) => jsonrpc_to_grpc(e),
            Self::Transcode(msg) => tonic::Status::internal(msg.clone()),
            Self::Io(e) => tonic::Status::internal(e.to_string()),
            Self::AddrParse(e) => tonic::Status::invalid_argument(e.to_string()),
            Self::Serialization(e) => tonic::Status::internal(e.to_string()),
            Self::Other(msg) => tonic::Status::internal(msg.clone()),
        }
    }
}

/// Convert gRPC Status to JSON-RPC error
pub fn grpc_to_jsonrpc(status: &tonic::Status) -> JsonRpcError {
    let (code, message) = match status.code() {
        Code::Ok => return JsonRpcError::internal_error("Unexpected OK status as error"),
        Code::Cancelled => (JsonRpcErrorCode::ServerError(-32001), "Request cancelled"),
        Code::Unknown => (JsonRpcErrorCode::InternalError, "Unknown error"),
        Code::InvalidArgument => (JsonRpcErrorCode::InvalidParams, "Invalid argument"),
        Code::DeadlineExceeded => (JsonRpcErrorCode::ServerError(-32002), "Deadline exceeded"),
        Code::NotFound => (JsonRpcErrorCode::MethodNotFound, "Not found"),
        Code::AlreadyExists => (JsonRpcErrorCode::ServerError(-32003), "Already exists"),
        Code::PermissionDenied => (JsonRpcErrorCode::ServerError(-32004), "Permission denied"),
        Code::ResourceExhausted => (JsonRpcErrorCode::ServerError(-32005), "Resource exhausted"),
        Code::FailedPrecondition => (JsonRpcErrorCode::ServerError(-32006), "Failed precondition"),
        Code::Aborted => (JsonRpcErrorCode::ServerError(-32007), "Aborted"),
        Code::OutOfRange => (JsonRpcErrorCode::InvalidParams, "Out of range"),
        Code::Unimplemented => (JsonRpcErrorCode::MethodNotFound, "Unimplemented"),
        Code::Internal => (JsonRpcErrorCode::InternalError, "Internal error"),
        Code::Unavailable => (JsonRpcErrorCode::ServerError(-32008), "Service unavailable"),
        Code::DataLoss => (JsonRpcErrorCode::InternalError, "Data loss"),
        Code::Unauthenticated => (JsonRpcErrorCode::ServerError(-32009), "Unauthenticated"),
    };

    let mut err = JsonRpcError::new(code, if status.message().is_empty() { message } else { status.message() });
    if !status.details().is_empty() {
        err = err.with_data(serde_json::json!({ "details": status.message() }));
    }
    err
}

/// Convert JSON-RPC error to gRPC Status
pub fn jsonrpc_to_grpc(err: &JsonRpcError) -> tonic::Status {
    match err.code {
        -32700 => tonic::Status::invalid_argument(err.message.clone()),
        -32600 => tonic::Status::invalid_argument(err.message.clone()),
        -32601 => tonic::Status::not_found(err.message.clone()),
        -32602 => tonic::Status::invalid_argument(err.message.clone()),
        -32603 => tonic::Status::internal(err.message.clone()),
        c if (-32099..=-32000).contains(&c) => tonic::Status::unavailable(err.message.clone()),
        _ => tonic::Status::internal(err.message.clone()),
    }
}
