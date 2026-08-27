// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KvEngine：璇玑自研 K-V 封装层。
//!
//! 默认实现：`parking_lot::RwLock<BTreeMap>` in-memory（测试/内存模式）。
//! 启用 `persist-rocksdb` feature 时切换到真实 RocksDB 5 列族 / shard。

use crate::error::{StorageError, StorageResult};
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn cf_name_vid_meta(shard: u16) -> String {
    format!("vid_meta_{shard}")
}
pub fn cf_name_out(shard: u16) -> String {
    format!("out_edges_{shard}")
}
pub fn cf_name_in(shard: u16) -> String {
    format!("in_edges_{shard}")
}
pub fn cf_name_vp(shard: u16) -> String {
    format!("vertex_props_{shard}")
}
pub fn cf_name_ep(shard: u16) -> String {
    format!("edge_props_{shard}")
}
pub fn cf_names_for(shard: u16) -> [String; 5] {
    [
        cf_name_vid_meta(shard),
        cf_name_out(shard),
        cf_name_in(shard),
        cf_name_vp(shard),
        cf_name_ep(shard),
    ]
}

#[cfg(feature = "persist-rocksdb")]
mod backend {
    use super::*;
    use rocksdb::{
        ColumnFamily, ColumnFamilyDescriptor, DBCompactionStyle, Options, WriteBatch, DB,
    };

    pub type InnerDB = Arc<DB>;
    pub type Batch = WriteBatch;

    pub fn open_db(
        path: &Path,
        shard_ids: &[u16],
    ) -> StorageResult<(InnerDB, Arc<HashSet<u16>>, Arc<PathBuf>)> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_compaction_style(DBCompactionStyle::Level);
        opts.increase_parallelism(2);
        opts.set_max_background_jobs(2);
        let existing = DB::list_cf(&opts, path).unwrap_or_default();
        let mut cf_set = HashSet::new();
        for c in existing {
            cf_set.insert(c);
        }
        for &s in shard_ids {
            for n in cf_names_for(s) {
                cf_set.insert(n);
            }
        }
        let cf_descs: Vec<_> = cf_set
            .iter()
            .map(|n| ColumnFamilyDescriptor::new(n.clone(), Options::default()))
            .collect();
        let db = DB::open_cf_descriptors(&opts, path, cf_descs)
            .map_err(|e| StorageError::Internal(format!("rocksdb open: {e}")))?;
        let mut shards_set = HashSet::new();
        for &s in shard_ids {
            shards_set.insert(s);
            for n in cf_names_for(s) {
                if db.cf_handle(&n).is_none() {
                    db.create_cf(n, &Options::default())
                        .map_err(|e| StorageError::Internal(format!("create cf {e}")))?;
                }
            }
        }
        Ok((
            Arc::new(db),
            Arc::new(shards_set),
            Arc::new(path.to_path_buf()),
        ))
    }
    pub fn add_shard(
        db: &InnerDB,
        shards: &Arc<HashSet<u16>>,
        shard: u16,
    ) -> StorageResult<Arc<HashSet<u16>>> {
        for n in cf_names_for(shard) {
            if db.cf_handle(&n).is_none() {
                db.create_cf(n, &Options::default())
                    .map_err(|e| StorageError::Internal(format!("add cf {e}")))?;
            }
        }
        let mut s = (**shards).clone();
        s.insert(shard);
        Ok(Arc::new(s))
    }
    pub fn cf<'a>(db: &'a InnerDB, name: &str) -> StorageResult<&'a ColumnFamily> {
        db.cf_handle(name)
            .ok_or_else(|| StorageError::Internal(format!("cf not found: {name}")))
    }
    pub fn put_cf(db: &InnerDB, name: &str, k: &[u8], v: &[u8]) -> StorageResult<()> {
        let c = cf(db, name)?;
        db.put_cf(c, k, v)
            .map_err(|e| StorageError::Internal(format!("put {e}")))
    }
    pub fn get_cf(db: &InnerDB, name: &str, k: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let c = cf(db, name)?;
        db.get_cf(c, k)
            .map_err(|e| StorageError::Internal(format!("get {e}")))
    }
    pub fn delete_cf(db: &InnerDB, name: &str, k: &[u8]) -> StorageResult<()> {
        let c = cf(db, name)?;
        db.delete_cf(c, k)
            .map_err(|e| StorageError::Internal(format!("del {e}")))
    }
    pub fn new_batch() -> Batch {
        WriteBatch::default()
    }
    pub fn batch_put(
        db: &InnerDB,
        b: &mut Batch,
        n: &str,
        k: &[u8],
        v: &[u8],
    ) -> StorageResult<()> {
        let c = cf(db, n)?;
        b.put_cf(c, k, v);
        Ok(())
    }
    pub fn batch_del(db: &InnerDB, b: &mut Batch, n: &str, k: &[u8]) -> StorageResult<()> {
        let c = cf(db, n)?;
        b.delete_cf(c, k);
        Ok(())
    }
    pub fn write_batch(db: &InnerDB, b: Batch) -> StorageResult<()> {
        db.write(b)
            .map_err(|e| StorageError::Internal(format!("wb {e}")))
    }
    pub fn seek_prefix(
        db: &InnerDB,
        n: &str,
        prefix: &[u8],
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let c = cf(db, n)?;
        let mut it = db.raw_iterator_cf(c);
        let mut out = Vec::new();
        it.seek(prefix);
        while it.valid() {
            if let Some(k) = it.key() {
                if !k.starts_with(prefix) {
                    break;
                }
                let v = it.value().unwrap_or_default().to_vec();
                out.push((k.to_vec(), v));
            } else {
                break;
            }
            it.next();
        }
        Ok(out)
    }
    pub fn scan_cf(
        db: &InnerDB,
        n: &str,
        limit: u32,
        offset: u64,
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let c = cf(db, n)?;
        let mut it = db.raw_iterator_cf(c);
        let mut out = Vec::with_capacity(limit as usize);
        let mut skipped: u64 = 0;
        it.seek_to_first();
        while it.valid() {
            if skipped < offset {
                skipped += 1;
                it.next();
                continue;
            }
            if out.len() >= limit as usize {
                break;
            }
            if let (Some(k), Some(v)) = (it.key(), it.value()) {
                out.push((k.to_vec(), v.to_vec()));
            }
            it.next();
        }
        Ok(out)
    }
}

#[cfg(not(feature = "persist-rocksdb"))]
mod backend {
    use super::*;

    pub struct MemDB {
        pub cf: RwLock<BTreeMap<String, BTreeMap<Vec<u8>, Vec<u8>>>>,
    }
    pub type InnerDB = Arc<MemDB>;

    #[derive(Debug, Clone, Default)]
    pub struct Batch(pub Vec<BatchOp>);

    #[derive(Debug, Clone)]
    pub enum BatchOp {
        Put { cf: String, k: Vec<u8>, v: Vec<u8> },
        Del { cf: String, k: Vec<u8> },
    }

    pub fn open_db(
        path: &Path,
        shard_ids: &[u16],
    ) -> StorageResult<(InnerDB, Arc<HashSet<u16>>, Arc<PathBuf>)> {
        let mut cf_map = BTreeMap::new();
        for &s in shard_ids {
            for n in cf_names_for(s) {
                cf_map.insert(n, BTreeMap::new());
            }
        }
        std::fs::create_dir_all(path).map_err(|e| StorageError::Internal(format!("mkdir {e}")))?;
        let db = Arc::new(MemDB {
            cf: RwLock::new(cf_map),
        });
        let mut shards = HashSet::new();
        for &s in shard_ids {
            shards.insert(s);
        }
        Ok((db, Arc::new(shards), Arc::new(path.to_path_buf())))
    }

    pub fn add_shard(
        db: &InnerDB,
        shards: &Arc<HashSet<u16>>,
        shard: u16,
    ) -> StorageResult<Arc<HashSet<u16>>> {
        let mut cf = db.cf.write();
        for n in cf_names_for(shard) {
            cf.entry(n).or_default();
        }
        let mut s = (**shards).clone();
        s.insert(shard);
        Ok(Arc::new(s))
    }
    pub fn put_cf(db: &InnerDB, n: &str, k: &[u8], v: &[u8]) -> StorageResult<()> {
        let mut cf = db.cf.write();
        cf.entry(n.to_string())
            .or_default()
            .insert(k.to_vec(), v.to_vec());
        Ok(())
    }
    pub fn get_cf(db: &InnerDB, n: &str, k: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let cf = db.cf.read();
        Ok(cf.get(n).and_then(|m| m.get(k).cloned()))
    }
    pub fn delete_cf(db: &InnerDB, n: &str, k: &[u8]) -> StorageResult<()> {
        let mut cf = db.cf.write();
        if let Some(m) = cf.get_mut(n) {
            m.remove(k);
        }
        Ok(())
    }
    pub fn new_batch() -> Batch {
        Batch::default()
    }
    pub fn batch_put(
        _db: &InnerDB,
        b: &mut Batch,
        n: &str,
        k: &[u8],
        v: &[u8],
    ) -> StorageResult<()> {
        b.0.push(BatchOp::Put {
            cf: n.to_string(),
            k: k.to_vec(),
            v: v.to_vec(),
        });
        Ok(())
    }
    pub fn batch_del(_db: &InnerDB, b: &mut Batch, n: &str, k: &[u8]) -> StorageResult<()> {
        b.0.push(BatchOp::Del {
            cf: n.to_string(),
            k: k.to_vec(),
        });
        Ok(())
    }
    pub fn write_batch(db: &InnerDB, b: Batch) -> StorageResult<()> {
        let mut cf = db.cf.write();
        for op in b.0 {
            match op {
                BatchOp::Put { cf: n, k, v } => {
                    cf.entry(n).or_default().insert(k, v);
                }
                BatchOp::Del { cf: n, k } => {
                    if let Some(m) = cf.get_mut(&n) {
                        m.remove(&k);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn seek_prefix(
        db: &InnerDB,
        n: &str,
        prefix: &[u8],
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let cf = db.cf.read();
        let Some(map) = cf.get(n) else {
            return Ok(Vec::new());
        };
        let start: Vec<u8> = prefix.to_vec();
        // range upper = prefix incremented by least significant byte (lexicographic +1).
        let mut end = start.clone();
        let mut carry = true;
        let mut i = end.len();
        while carry && i > 0 {
            i -= 1;
            if end[i] < 0xff {
                end[i] += 1;
                carry = false;
            } else {
                end[i] = 0;
            }
        }
        // If carry remains (all 0xff), end is empty → range is unbounded.
        let mut out = Vec::new();
        if carry {
            for (k, v) in map.range(start..) {
                out.push((k.clone(), v.clone()));
            }
        } else {
            for (k, v) in map.range(start..end) {
                out.push((k.clone(), v.clone()));
            }
        }
        Ok(out)
    }
    pub fn scan_cf(
        db: &InnerDB,
        n: &str,
        limit: u32,
        offset: u64,
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let cf = db.cf.read();
        let Some(map) = cf.get(n) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::with_capacity(limit as usize);
        for (i, (k, v)) in map.iter().enumerate() {
            if (i as u64) < offset {
                continue;
            }
            if out.len() >= limit as usize {
                break;
            }
            out.push((k.clone(), v.clone()));
        }
        Ok(out)
    }
}

#[derive(Clone)]
pub struct KvEngine {
    pub path: Arc<PathBuf>,
    pub db: backend::InnerDB,
    pub shards: Arc<HashSet<u16>>,
}

impl KvEngine {
    pub fn new_mem() -> StorageResult<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let r = rand::random::<u32>();
        let base = std::env::temp_dir().join(format!("mox-kv-mem-{ns}-{r}"));
        std::fs::create_dir_all(&base).map_err(|e| StorageError::Internal(format!("mkdir {e}")))?;
        Self::open(&base, &[])
    }

    pub fn open<P: AsRef<Path>>(path: P, shard_ids: &[u16]) -> StorageResult<Self> {
        let (db, shards, pb) = backend::open_db(path.as_ref(), shard_ids)?;
        Ok(Self {
            db,
            shards,
            path: pb,
        })
    }
    pub fn add_shard(&mut self, shard: u16) -> StorageResult<()> {
        let new = backend::add_shard(&self.db, &self.shards, shard)?;
        self.shards = new;
        Ok(())
    }
    pub fn contains_shard(&self, shard: u16) -> bool {
        self.shards.contains(&shard)
    }
    pub fn put_cf(&self, cf: &str, k: &[u8], v: &[u8]) -> StorageResult<()> {
        backend::put_cf(&self.db, cf, k, v)
    }
    pub fn get_cf(&self, cf: &str, k: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        backend::get_cf(&self.db, cf, k)
    }
    pub fn delete_cf(&self, cf: &str, k: &[u8]) -> StorageResult<()> {
        backend::delete_cf(&self.db, cf, k)
    }
    pub fn write_batch(&self, b: backend::Batch) -> StorageResult<()> {
        backend::write_batch(&self.db, b)
    }
    pub fn new_batch() -> backend::Batch {
        backend::new_batch()
    }
    pub fn batch_put_cf(
        &self,
        b: &mut backend::Batch,
        cf: &str,
        k: &[u8],
        v: &[u8],
    ) -> StorageResult<()> {
        backend::batch_put(&self.db, b, cf, k, v)
    }
    pub fn batch_del_cf(&self, b: &mut backend::Batch, cf: &str, k: &[u8]) -> StorageResult<()> {
        backend::batch_del(&self.db, b, cf, k)
    }
    pub fn seek_prefix(&self, cf: &str, prefix: &[u8]) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        backend::seek_prefix(&self.db, cf, prefix)
    }
    pub fn scan_cf(
        &self,
        cf: &str,
        limit: u32,
        offset: u64,
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        backend::scan_cf(&self.db, cf, limit, offset)
    }
}
