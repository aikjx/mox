// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RustFS ecstore 存储后端接入点骨架。
//!
//! 参考 RustFS ecstore 架构（Apache 2.0, `ais/RustFS/crates/ecstore/`），
//! 本文件定义 [`RustFsEcstoreBackend`] 作为 [`StorageBackend`] trait 的接入点骨架。
//!
//! **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
//!
//! 当前所有方法均返回 [`StorageError::Unsupported`]，明确标注待对接状态。
//! 后续阶段将实现：
//! - 通过 Unix Socket / gRPC 与 RustFS ecstore 进程通信
//! - erasure coding 分片写入（data + parity chunks）
//! - 跨节点冗余读取与修复
//! - chunk 元数据（EC profile、replica 位置）管理

use async_trait::async_trait;
use mox_cloud_domain_traits::{
    BackendCapabilities, BackendType, ChunkId, ChunkInfo, ChunkListPage, ConsistencyModel,
    StorageBackend, StorageError,
};

/// RustFS ecstore 后端错误信息常量。
const UNSUPPORTED_MSG: &str =
    "RustFS ecstore backend: 接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段";

/// RustFS ecstore 存储后端（骨架实现）。
///
/// 通过 `endpoint`（RustFS ecstore 进程地址）和 `pool_name`（EC 存储池名称）
/// 构造。当前阶段所有数据面操作均返回 `Unsupported`，仅用于验证 trait 接入和
/// feature flag 编译路径。
///
/// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
pub struct RustFsEcstoreBackend {
    endpoint: String,
    pool_name: String,
}

impl RustFsEcstoreBackend {
    /// 构造 RustFS ecstore 后端骨架。
    ///
    /// # 参数
    /// - `endpoint`: RustFS ecstore 进程监听地址（如 `unix:///var/run/rustfs/ecstore.sock`）
    /// - `pool_name`: EC 存储池名称（对应 RustFS ecstore 的 pool 概念）
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    pub fn new(endpoint: String, pool_name: String) -> Self {
        Self {
            endpoint,
            pool_name,
        }
    }

    /// 检查后端是否可用。
    ///
    /// 当前阶段始终返回 `false`——实际 RustFS 进程对接待后续阶段。
    /// 后续将实现：探测 endpoint 连通性 + ecstore 进程健康检查。
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    pub fn is_available(&self) -> bool {
        false
    }

    /// 获取配置的 endpoint（诊断用）。
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// 获取配置的 pool_name（诊断用）。
    pub fn pool_name(&self) -> &str {
        &self.pool_name
    }
}

#[async_trait]
impl StorageBackend for RustFsEcstoreBackend {
    /// 写入数据块到 RustFS ecstore。
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    /// 后续将实现：EC 编码分片 → 写入多节点 → 返回 chunk 元信息（含 EC profile）。
    async fn put_chunk(
        &self,
        _chunk_id: &ChunkId,
        _data: &[u8],
    ) -> Result<ChunkInfo, StorageError> {
        Err(StorageError::Unsupported)
    }

    /// 从 RustFS ecstore 读取数据块。
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    /// 后续将实现：从多节点读取 data+parity shards → EC 解码重建 → 返回完整数据。
    async fn get_chunk(&self, _chunk_id: &ChunkId) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::Unsupported)
    }

    /// 从 RustFS ecstore 删除数据块。
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    /// 后续将实现：通知 ecstore 回收所有关联 shards（data + parity）。
    async fn delete_chunk(&self, _chunk_id: &ChunkId) -> Result<bool, StorageError> {
        Err(StorageError::Unsupported)
    }

    /// 检查数据块是否存在于 RustFS ecstore。
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    async fn chunk_exists(&self, _chunk_id: &ChunkId) -> Result<bool, StorageError> {
        Err(StorageError::Unsupported)
    }

    /// 按前缀分页列出 RustFS ecstore 中的数据块。
    ///
    /// **接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段。**
    async fn list_chunks(
        &self,
        _prefix: &str,
        _marker: Option<&str>,
        _limit: u32,
    ) -> Result<ChunkListPage, StorageError> {
        Err(StorageError::Unsupported)
    }

    fn backend_type(&self) -> BackendType {
        BackendType::RustFsEcstore
    }

    fn capabilities(&self) -> BackendCapabilities {
        // 目标能力：EC 后端最终应支持范围读、原子写、强一致（quorum 读修复）
        BackendCapabilities {
            supports_range_read: true,
            supports_atomic_write: true,
            supports_conditional_put: false,
            consistency_model: ConsistencyModel::Strong,
            max_chunk_size: 128 * 1024 * 1024,
            preferred_chunk_size: 4 * 1024 * 1024,
        }
    }

    fn name(&self) -> &'static str {
        "rustfs-ecstore-backend"
    }
}

impl std::fmt::Debug for RustFsEcstoreBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RustFsEcstoreBackend")
            .field("endpoint", &self.endpoint)
            .field("pool_name", &self.pool_name)
            .field("available", &self.is_available())
            .field("status", &"skeleton: FFI/process integration pending")
            .finish()
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_and_accessors() {
        let backend = RustFsEcstoreBackend::new(
            "unix:///var/run/rustfs/ecstore.sock".into(),
            "ec-pool-01".into(),
        );
        assert_eq!(backend.endpoint(), "unix:///var/run/rustfs/ecstore.sock");
        assert_eq!(backend.pool_name(), "ec-pool-01");
        assert!(!backend.is_available(), "骨架阶段应始终不可用");
    }

    #[tokio::test]
    async fn test_all_methods_return_unsupported() {
        let backend = RustFsEcstoreBackend::new("ep".into(), "pool".into());
        let id = ChunkId::new("test-chunk");

        assert!(matches!(
            backend.put_chunk(&id, b"data").await,
            Err(StorageError::Unsupported)
        ));
        assert!(matches!(
            backend.get_chunk(&id).await,
            Err(StorageError::Unsupported)
        ));
        assert!(matches!(
            backend.delete_chunk(&id).await,
            Err(StorageError::Unsupported)
        ));
        assert!(matches!(
            backend.chunk_exists(&id).await,
            Err(StorageError::Unsupported)
        ));
        assert!(matches!(
            backend.list_chunks("prefix", None, 10).await,
            Err(StorageError::Unsupported)
        ));
    }

    #[test]
    fn test_backend_metadata() {
        let backend = RustFsEcstoreBackend::new("ep".into(), "pool".into());
        assert_eq!(backend.backend_type(), BackendType::RustFsEcstore);
        assert_eq!(backend.name(), "rustfs-ecstore-backend");
        let caps = backend.capabilities();
        assert_eq!(caps.consistency_model, ConsistencyModel::Strong);
        assert!(caps.supports_range_read);
    }

    #[test]
    fn test_debug_output_contains_skeleton_note() {
        let backend = RustFsEcstoreBackend::new("ep".into(), "pool".into());
        let dbg = format!("{:?}", backend);
        assert!(dbg.contains("skeleton"));
        assert!(dbg.contains("pending"));
    }
}
