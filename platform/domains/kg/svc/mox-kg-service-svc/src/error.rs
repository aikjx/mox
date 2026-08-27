// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Graph Service 统一错误枚举
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    #[error("SyntaxError: {0}")]
    SyntaxError(String),

    #[error("SemanticError: {0}")]
    SemanticError(String),

    #[error("NoSpaceError: {0}")]
    NoSpaceError(String),

    #[error("SchemaNotFound: {0}")]
    SchemaNotFound(String),

    #[error("OptimizerTimeout: plan exceed timeout budget")]
    OptimizerTimeout,

    #[error("StorageError: {0}")]
    StorageError(String),

    #[error("AlgoBridgeMismatch: {0}")]
    AlgoBridgeMismatch(String),

    #[error("RBACDenied: {0}")]
    RBACDenied(String),

    #[error("Internal: {0}")]
    Internal(String),
}

pub type GraphResult<T> = Result<T, GraphError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_err_syntax_display() {
        let e = GraphError::SyntaxError("unexpected token at 1:5".into());
        assert!(format!("{}", e).contains("SyntaxError"));
    }

    #[test]
    fn t_err_semantic_eq() {
        let a = GraphError::SemanticError("tag missing".into());
        let b = GraphError::SemanticError("tag missing".into());
        assert_eq!(a, b);
    }

    #[test]
    fn t_err_no_space() {
        let e = GraphError::NoSpaceError("demo".into());
        assert!(format!("{}", e).contains("demo"));
    }

    #[test]
    fn t_err_schema_not_found() {
        let e = GraphError::SchemaNotFound("Tag:Player".into());
        assert!(matches!(e, GraphError::SchemaNotFound(_)));
    }

    #[test]
    fn t_err_optimizer_timeout() {
        let e = GraphError::OptimizerTimeout;
        assert!(format!("{}", e).contains("timeout"));
    }

    #[test]
    fn t_err_storage() {
        let e = GraphError::StorageError("raft leader down".into());
        assert!(matches!(e, GraphError::StorageError(_)));
    }

    #[test]
    fn t_err_algo_bridge() {
        let e = GraphError::AlgoBridgeMismatch("PPR delta 1e-4 exceeded".into());
        assert!(matches!(e, GraphError::AlgoBridgeMismatch(_)));
    }

    #[test]
    fn t_err_rbac_denied() {
        let e = GraphError::RBACDenied("role=guest op=DROP_SPACE".into());
        assert!(matches!(e, GraphError::RBACDenied(_)));
    }

    #[test]
    fn t_err_internal() {
        let e = GraphError::Internal("oops".into());
        assert!(matches!(e, GraphError::Internal(_)));
    }

    #[test]
    fn t_result_alias_ok() {
        let r: GraphResult<i32> = Ok(42);
        assert_eq!(r.unwrap(), 42);
    }
}
