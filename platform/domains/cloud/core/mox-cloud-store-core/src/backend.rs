// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 可插拔后端工厂。
//!
//! [`create_backend`] 依据 [`StoreConfig`] 装配存储后端，返回统一的
//! [`StoreBackend`]（对象存储 + KV + 流式写三路物理口）。
//!
//! - 阶段1：`Fs`（内容寻址去重真实落盘）。
//! - 阶段2（feature `s3`）：`S3`/`Minio`/`Oss`（自研 SigV4 客户端 + 回源）。

use crate::fs_backend::FsObjectStore;
use mox_base_store_core::{KvStore, ObjectStore, ObjectStreamWriter, StoreError, StoreResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// 后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendKind {
    /// 本地文件系统（内容寻址 + 引用计数 GC）
    Fs,
    /// 通用 S3 兼容（MinIO/COS/OBS/OSS）
    S3,
    /// MinIO
    Minio,
    /// 阿里云 OSS
    Oss,
}

impl BackendKind {
    /// 从配置字符串解析（`fs|s3|minio|oss`），大小写不敏感
    pub fn from_str_ci(s: &str) -> StoreResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fs" | "file" => Ok(Self::Fs),
            "s3" => Ok(Self::S3),
            "minio" => Ok(Self::Minio),
            "oss" => Ok(Self::Oss),
            other => Err(StoreError::Other(format!("未知后端类型: {other}"))),
        }
    }
}

/// S3 客户端配置（阶段2）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3ClientConfig {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    #[serde(default = "default_true")]
    pub force_path_style: bool,
}

/// 存储后端装配配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    pub kind: BackendKind,
    /// 本地数据目录（FS 后端；S3 后端用于缓存/索引）
    pub data_dir: PathBuf,
    #[serde(default = "default_true")]
    pub verify_checksum: bool,
    #[serde(default)]
    pub s3: Option<S3ClientConfig>,
}

/// 统一后端门面：三路物理口
#[derive(Clone)]
pub struct StoreBackend {
    pub kind: BackendKind,
    pub object: Arc<dyn ObjectStore>,
    pub kv: Arc<dyn KvStore>,
    pub stream: Arc<dyn ObjectStreamWriter>,
    /// 本地数据目录（FS 后端为数据根；S3 后端为缓存/KV/索引根）
    pub data_dir: PathBuf,
}

fn default_true() -> bool {
    true
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            kind: BackendKind::Fs,
            data_dir: PathBuf::from("./data/store"),
            verify_checksum: true,
            s3: None,
        }
    }
}

/// 依据配置装配后端（每阶段可编译可测试）
pub fn create_backend(cfg: &StoreConfig) -> StoreResult<StoreBackend> {
    match cfg.kind {
        BackendKind::Fs => {
            let store = Arc::new(FsObjectStore::with_options(&cfg.data_dir, cfg.verify_checksum)?);
            Ok(StoreBackend {
                kind: BackendKind::Fs,
                object: store.clone(),
                kv: store.clone(),
                stream: store,
                data_dir: cfg.data_dir.clone(),
            })
        }
        #[cfg(feature = "s3")]
        BackendKind::S3 | BackendKind::Minio | BackendKind::Oss => {
            let s3_cfg = cfg
                .s3
                .as_ref()
                .ok_or_else(|| StoreError::Other("S3 后端缺少 s3 配置".into()))?;
            crate::s3_backend::build_s3_backend(&cfg.data_dir, s3_cfg, cfg.kind)
        }
        #[cfg(not(feature = "s3"))]
        BackendKind::S3 | BackendKind::Minio | BackendKind::Oss => Err(StoreError::Other(format!(
            "后端 {:?} 需启用 feature `s3`（阶段2 计划）",
            cfg.kind
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_parsing() {
        assert_eq!(BackendKind::from_str_ci("fs").unwrap(), BackendKind::Fs);
        assert_eq!(BackendKind::from_str_ci("S3").unwrap(), BackendKind::S3);
        assert_eq!(BackendKind::from_str_ci(" MinIO ").unwrap(), BackendKind::Minio);
        assert_eq!(BackendKind::from_str_ci("oss").unwrap(), BackendKind::Oss);
        assert!(BackendKind::from_str_ci("gcs").is_err());
    }

    #[tokio::test]
    async fn create_fs_backend_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let be = create_backend(&cfg).unwrap();
        assert_eq!(be.kind, BackendKind::Fs);
        let obj = be
            .object
            .put("kb/notes.md", "text/markdown", bytes::Bytes::from_static(b"# note"))
            .await
            .unwrap();
        assert!(obj.sha256.is_some());
        let got = be.object.get("kb/notes.md").await.unwrap();
        assert_eq!(&got[..], b"# note");

        be.kv
            .put("bucket:docs", bytes::Bytes::from_static(b"{}"))
            .await
            .unwrap();
        assert_eq!(&be.kv.get("bucket:docs").await.unwrap().unwrap()[..], b"{}");
    }
}
