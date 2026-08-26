//! POSIX API：Filer 结构体，包装 MetaStorageProvider 暴露标准 POSIX 文件操作。
//!
//! 覆盖 pjd-fstest 的 10 个高层操作：
//! stat / chmod / link / symlink / mkdir / rmdir / open_close / read / write / rename / unlink

use std::sync::Arc;

use crate::error::{FilerError, FilerResult};
use crate::meta_trait::{Attr, AttrPatch, DirEntry, MetaStorageProvider};

#[derive(Clone)]
pub struct Filer {
    pub provider: Arc<dyn MetaStorageProvider>,
}

impl Filer {
    pub fn new(provider: Arc<dyn MetaStorageProvider>) -> Self {
        Self { provider }
    }

    // ============= path helpers =============
    async fn resolve(&self, path: &str) -> FilerResult<u64> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return Ok(1);
        }
        let mut parent = 1u64;
        for seg in path.split('/') {
            if seg.is_empty() {
                continue;
            }
            parent = self.provider.inode_lookup(parent, seg).await?;
        }
        Ok(parent)
    }

    async fn split_parent_name(&self, path: &str) -> FilerResult<(u64, String)> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return Err(FilerError::AttrInvalid);
        }
        let parts: Vec<&str> = path.split('/').collect();
        let name = parts.last().unwrap().to_string();
        let parent_path = parts[..parts.len() - 1].join("/");
        let parent = if parent_path.is_empty() {
            1
        } else {
            self.resolve(&format!("/{}", parent_path)).await?
        };
        Ok((parent, name))
    }

    // ============= POSIX ops =============

    pub async fn mkdir(&self, path: &str, mode: u32) -> FilerResult<u64> {
        let (parent, name) = self.split_parent_name(path).await?;
        self.provider.inode_mkdir(parent, &name, mode).await
    }

    pub async fn create(&self, path: &str, mode: u32) -> FilerResult<u64> {
        let (parent, name) = self.split_parent_name(path).await?;
        self.provider.inode_create(parent, &name, mode).await
    }

    pub async fn open_close(&self, path: &str) -> FilerResult<Attr> {
        let ino = self.resolve(path).await?;
        self.provider.inode_read_attr(ino).await
    }

    pub async fn stat(&self, path: &str) -> FilerResult<Attr> {
        self.open_close(path).await
    }

    pub async fn lstat(&self, path: &str) -> FilerResult<Attr> {
        self.stat(path).await
    }

    pub async fn read(&self, path: &str, offset: u64, buf: &mut [u8]) -> FilerResult<usize> {
        let ino = self.resolve(path).await?;
        let a = self.provider.inode_read_attr(ino).await?;
        let start = (offset as usize).min(a.data.len());
        let end = (start + buf.len()).min(a.data.len());
        let slice = &a.data[start..end];
        buf[..slice.len()].copy_from_slice(slice);
        Ok(slice.len())
    }

    pub async fn read_all(&self, path: &str) -> FilerResult<Vec<u8>> {
        let ino = self.resolve(path).await?;
        let a = self.provider.inode_read_attr(ino).await?;
        Ok(a.data)
    }

    pub async fn write(&self, path: &str, offset: u64, data: &[u8]) -> FilerResult<usize> {
        let ino = match self.resolve(path).await {
            Ok(i) => i,
            Err(_) => self.create(path, 0o644).await?,
        };
        let mut a = self.provider.inode_read_attr(ino).await?;
        let need = offset as usize + data.len();
        if a.data.len() < need {
            a.data.resize(need, 0);
        }
        a.data[offset as usize..offset as usize + data.len()].copy_from_slice(data);
        self.provider
            .inode_write_attr(
                ino,
                AttrPatch {
                    data: Some(&a.data),
                    ..Default::default()
                },
            )
            .await?;
        Ok(data.len())
    }

    pub async fn unlink(&self, path: &str) -> FilerResult<()> {
        let (parent, name) = self.split_parent_name(path).await?;
        self.provider.inode_unlink(parent, &name).await
    }

    pub async fn rmdir(&self, path: &str) -> FilerResult<()> {
        let ino = self.resolve(path).await?;
        let entries = self.provider.inode_list_dir(ino).await?;
        if !entries.is_empty() {
            return Err(FilerError::NotEmpty);
        }
        let (parent, name) = self.split_parent_name(path).await?;
        self.provider.inode_unlink(parent, &name).await
    }

    pub async fn rename(&self, from: &str, to: &str) -> FilerResult<()> {
        let (op, on) = self.split_parent_name(from).await?;
        let (np, nn) = self.split_parent_name(to).await?;
        self.provider.inode_rename(op, &on, np, &nn).await
    }

    pub async fn link(&self, src: &str, dst: &str) -> FilerResult<()> {
        let ino = self.resolve(src).await?;
        let (np, nn) = self.split_parent_name(dst).await?;
        self.provider.inode_link(ino, np, &nn).await
    }

    pub async fn symlink(&self, target: &str, linkpath: &str) -> FilerResult<u64> {
        let (np, nn) = self.split_parent_name(linkpath).await?;
        self.provider.inode_symlink(np, &nn, target).await
    }

    pub async fn readdir(&self, path: &str) -> FilerResult<Vec<DirEntry>> {
        let ino = self.resolve(path).await?;
        self.provider.inode_list_dir(ino).await
    }

    pub async fn chmod(&self, path: &str, mode: u32) -> FilerResult<()> {
        let ino = self.resolve(path).await?;
        self.provider
            .inode_write_attr(
                ino,
                AttrPatch {
                    mode: Some(mode),
                    ..Default::default()
                },
            )
            .await
    }
}
