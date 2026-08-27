// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use async_trait::async_trait;
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileStat {
    pub inode: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub mtime: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatFs {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub block_size: u32,
}

#[derive(Debug, Clone, Default)]
struct InodeEntry {
    stat: FileStat,
    xattrs: BTreeMap<String, Vec<u8>>,
    children: BTreeMap<String, u64>,
    parent: u64,
    basename: String,
}

fn norm(p: &str) -> String {
    if p.is_empty() {
        return "/".into();
    }
    let mut s = p.replace('\\', "/");
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    s
}
fn split_parent(path: &str) -> (String, String) {
    let p = norm(path);
    if p == "/" {
        return ("/".into(), String::new());
    }
    let pos = p.rfind('/').unwrap_or(0);
    if pos == 0 {
        let name = p[1..].to_string();
        return ("/".into(), name);
    }
    let parent = p[..pos].to_string();
    let name = p[pos + 1..].to_string();
    (parent, name)
}

#[async_trait]
pub trait MetaStorageProvider: Send + Sync {
    async fn mkdir(
        &self,
        parent_path: &str,
        name: &str,
        mode: u32,
    ) -> Result<u64, Box<dyn Error + Send + Sync>>;
    async fn rmdir(&self, path: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn rename(
        &self,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn symlink(
        &self,
        target: &str,
        link_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn readlink(&self, path: &str) -> Result<String, Box<dyn Error + Send + Sync>>;
    async fn stat(&self, path: &str) -> Result<FileStat, Box<dyn Error + Send + Sync>>;
    async fn getxattr(
        &self,
        path: &str,
        name: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;
    async fn setxattr(
        &self,
        path: &str,
        name: &str,
        value: &[u8],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn listxattr(&self, path: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>>;
    async fn chmod(&self, path: &str, mode: u32) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn chown(
        &self,
        path: &str,
        uid: u32,
        gid: u32,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn statfs(&self) -> Result<StatFs, Box<dyn Error + Send + Sync>>;
}

pub struct MockMetaStorageProvider {
    inodes: parking_lot::Mutex<BTreeMap<u64, InodeEntry>>,
    paths: parking_lot::Mutex<BTreeMap<String, u64>>,
    next: parking_lot::Mutex<u64>,
}

impl Default for MockMetaStorageProvider {
    fn default() -> Self {
        let mut i = BTreeMap::new();
        let mut p = BTreeMap::new();
        let root = InodeEntry {
            stat: FileStat {
                inode: 1,
                mode: 0o755,
                is_dir: true,
                ..Default::default()
            },
            parent: 1,
            basename: String::new(),
            ..Default::default()
        };
        i.insert(1, root);
        p.insert("/".into(), 1);
        Self {
            inodes: parking_lot::Mutex::new(i),
            paths: parking_lot::Mutex::new(p),
            next: parking_lot::Mutex::new(2),
        }
    }
}

impl MockMetaStorageProvider {
    fn insert_at(
        &self,
        parent_path: &str,
        name: &str,
        ie: InodeEntry,
    ) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let pp = norm(parent_path);
        let mut inodes = self.inodes.lock();
        let mut paths = self.paths.lock();
        let mut n = self.next.lock();
        let parent_inode = *paths.get(&pp).ok_or("parent not found")?;
        let ino = *n;
        *n += 1;
        let ino2 = ino;
        let ie2 = InodeEntry {
            stat: FileStat {
                inode: ino2,
                mode: ie.stat.mode,
                ..ie.stat
            },
            parent: parent_inode,
            basename: name.into(),
            xattrs: ie.xattrs,
            children: ie.children,
        };
        inodes.insert(ino, ie2);
        let new_path = if pp == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", pp, name)
        };
        paths.insert(new_path, ino);
        inodes
            .get_mut(&parent_inode)
            .ok_or("parent gone")?
            .children
            .insert(name.into(), ino);
        Ok(ino)
    }
    fn remove_path(&self, path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        if p == "/" {
            return Err("cannot remove root".into());
        }
        let mut paths = self.paths.lock();
        let mut inodes = self.inodes.lock();
        let ino = *paths.get(&p).ok_or("path not found")?;
        let (pp, name) = split_parent(&p);
        if let Some(parent_ino) = paths.get(&pp).copied() {
            if let Some(p_ie) = inodes.get_mut(&parent_ino) {
                p_ie.children.remove(&name);
            }
        }
        inodes.remove(&ino);
        paths.remove(&p);
        Ok(())
    }
}

#[async_trait]
impl MetaStorageProvider for MockMetaStorageProvider {
    async fn mkdir(
        &self,
        parent_path: &str,
        name: &str,
        mode: u32,
    ) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let ie = InodeEntry {
            stat: FileStat {
                mode,
                is_dir: true,
                ..Default::default()
            },
            ..Default::default()
        };
        self.insert_at(parent_path, name, ie)
    }
    async fn rmdir(&self, path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.remove_path(path)
    }
    async fn rename(
        &self,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let op = norm(old_path);
        let np = norm(new_path);
        let ino = {
            let paths = self.paths.lock();
            *paths.get(&op).ok_or("old not found")?
        };
        // remove from old parent children
        {
            let (opp, on) = split_parent(&op);
            let mut inodes = self.inodes.lock();
            let paths = self.paths.lock();
            if let Some(p_ino) = paths.get(&opp).copied() {
                if let Some(pie) = inodes.get_mut(&p_ino) {
                    pie.children.remove(&on);
                }
            }
        }
        // add to new parent children
        {
            let (npp, nn) = split_parent(&np);
            let mut inodes = self.inodes.lock();
            let paths = self.paths.lock();
            if let Some(np_ino) = paths.get(&npp).copied() {
                if let Some(np_ie) = inodes.get_mut(&np_ino) {
                    np_ie.children.insert(nn.clone(), ino);
                }
            }
            // update inode basename and parent
            if let Some(ie) = inodes.get_mut(&ino) {
                ie.basename = nn;
                if let Some(np_ino) = paths.get(&npp).copied() {
                    ie.parent = np_ino;
                }
            }
        }
        let mut paths = self.paths.lock();
        paths.remove(&op);
        paths.insert(np, ino);
        Ok(())
    }
    async fn symlink(
        &self,
        target: &str,
        link_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (pp, name) = split_parent(link_path);
        let ie = InodeEntry {
            stat: FileStat {
                mode: 0o777,
                is_symlink: true,
                symlink_target: Some(target.into()),
                ..Default::default()
            },
            ..Default::default()
        };
        self.insert_at(&pp, &name, ie)?;
        Ok(())
    }
    async fn readlink(&self, path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let inodes = self.inodes.lock();
        let ino = paths.get(&p).ok_or("not found")?;
        let ie = inodes.get(ino).ok_or("inode missing")?;
        ie.stat
            .symlink_target
            .clone()
            .ok_or_else(|| "not a symlink".into())
    }
    async fn stat(&self, path: &str) -> Result<FileStat, Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let inodes = self.inodes.lock();
        let ino = paths.get(&p).ok_or("path not found")?;
        let ie = inodes.get(ino).ok_or("inode missing")?;
        Ok(ie.stat.clone())
    }
    async fn getxattr(
        &self,
        path: &str,
        name: &str,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let inodes = self.inodes.lock();
        let ino = paths.get(&p).ok_or("path not found")?;
        let ie = inodes.get(ino).ok_or("inode missing")?;
        ie.xattrs
            .get(name)
            .cloned()
            .ok_or_else(|| "xattr missing".into())
    }
    async fn setxattr(
        &self,
        path: &str,
        name: &str,
        value: &[u8],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let mut inodes = self.inodes.lock();
        let ino = *paths.get(&p).ok_or("path not found")?;
        let ie = inodes.get_mut(&ino).ok_or("inode missing")?;
        ie.xattrs.insert(name.into(), value.to_vec());
        Ok(())
    }
    async fn listxattr(&self, path: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let inodes = self.inodes.lock();
        let ino = paths.get(&p).ok_or("path not found")?;
        let ie = inodes.get(ino).ok_or("inode missing")?;
        Ok(ie.xattrs.keys().cloned().collect())
    }
    async fn chmod(&self, path: &str, mode: u32) -> Result<(), Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let mut inodes = self.inodes.lock();
        let ino = *paths.get(&p).ok_or("path not found")?;
        let ie = inodes.get_mut(&ino).ok_or("inode missing")?;
        ie.stat.mode = (ie.stat.mode & !0o7777) | (mode & 0o7777);
        Ok(())
    }
    async fn chown(
        &self,
        path: &str,
        uid: u32,
        gid: u32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let p = norm(path);
        let paths = self.paths.lock();
        let mut inodes = self.inodes.lock();
        let ino = *paths.get(&p).ok_or("path not found")?;
        let ie = inodes.get_mut(&ino).ok_or("inode missing")?;
        ie.stat.uid = uid;
        ie.stat.gid = gid;
        Ok(())
    }
    async fn statfs(&self) -> Result<StatFs, Box<dyn Error + Send + Sync>> {
        Ok(StatFs {
            total_blocks: 1_000_000,
            free_blocks: 500_000,
            total_inodes: 100_000,
            free_inodes: 99_000,
            block_size: 4096,
        })
    }
}
