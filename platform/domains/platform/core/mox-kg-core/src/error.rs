// mox-kg-core 错误类型

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KgError {
    #[error("存储错误: {0}")]
    StorageError(String),

    #[error("顶点不存在: {0}")]
    VertexNotFound(String),

    #[error("边不存在: {0}")]
    EdgeNotFound(String),

    #[error("顶点已存在: {0}")]
    VertexAlreadyExists(String),

    #[error("边已存在: {0}")]
    EdgeAlreadyExists(String),

    #[error("DSL解析错误: {0}")]
    DslParseError(String),

    #[error("查询错误: {0}")]
    QueryError(String),

    #[error("参数错误: {0}")]
    InvalidParam(String),

    #[error("序列化错误: {0}")]
    SerializeError(String),

    #[error("反序列化错误: {0}")]
    DeserializeError(String),

    #[error("遍历深度超限: max_depth={0}")]
    TraverseDepthExceeded(usize),

    #[error("内部错误: {0}")]
    InternalError(String),
}

pub type KgResult<T> = Result<T, KgError>;
