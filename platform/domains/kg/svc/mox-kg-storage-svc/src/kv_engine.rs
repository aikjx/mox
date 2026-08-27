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
        BlockBasedOptions, Cache, ColumnFamily, ColumnFamilyDescriptor, DBCompactionStyle,
        DBCompressionType, Options, ReadOptions, SliceTransform,
        WriteBatch, WriteOptions, DB,
    };
    use std::collections::HashMap;
    use std::sync::OnceLock;

    pub type InnerDB = Arc<DB>;
    pub type Batch = WriteBatch;

    /// 全局 block cache（所有CF共享，避免重复缓存）
    /// 生产环境建议设置为物理内存的30%-50%
    static BLOCK_CACHE: OnceLock<Cache> = OnceLock::new();
    fn block_cache() -> &'static Cache {
        BLOCK_CACHE.get_or_init(|| {
            // 默认512MB，可通过环境变量 MOX_ROCKSDB_BLOCK_CACHE_MB 覆盖
            let mb = std::env::var("MOX_ROCKSDB_BLOCK_CACHE_MB")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(512);
            Cache::new_lru_cache(mb * 1024 * 1024)
        })
    }

    /// 全局 WriteOptions（复用，避免每次新建）
    static WRITE_OPTS: OnceLock<WriteOptions> = OnceLock::new();
    fn write_opts() -> &'static WriteOptions {
        WRITE_OPTS.get_or_init(|| {
            let mut opts = WriteOptions::default();
            // 不等待WAL fsync，性能优先（Raft层已保证持久性）
            opts.set_sync(false);
            // 禁用WAL对于纯KV场景可进一步提升性能，但Raft场景保留WAL
            opts.disable_wal(false);
            opts
        })
    }

    /// 生产级 ColumnFamily Options
    fn cf_options() -> Options {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compaction_style(DBCompactionStyle::Level);

        // ===== Compression =====
        // L0/L1不压缩（写入频繁），L2及以后用LZ4（读多写少场景最优）
        opts.set_compression_per_level(&[
            DBCompressionType::None,
            DBCompressionType::None,
            DBCompressionType::Lz4,
            DBCompressionType::Lz4,
            DBCompressionType::Lz4,
            DBCompressionType::Lz4,
            DBCompressionType::Zstd,
        ]);

        // ===== Write Buffer =====
        // 每个CF 64MB write buffer，最多4个，memtable满载后flush
        opts.set_write_buffer_size(64 * 1024 * 1024);
        opts.set_max_write_buffer_number(4);
        // L0文件数达到4个触发compaction
        opts.set_level_zero_file_num_compaction_trigger(4);
        opts.set_level_zero_slowdown_writes_trigger(20);
        opts.set_level_zero_stop_writes_trigger(36);

        // ===== Block Based Table =====
        let mut block_opts = BlockBasedOptions::default();
        // 4KB block size（KG元数据偏小，4KB平衡索引和数据）
        block_opts.set_block_size(4 * 1024);
        // 共享全局block cache
        block_opts.set_block_cache(block_cache());
        // 开启cache_index_and_filter_blocks（索引和过滤器也进cache，内存命中更快）
        block_opts.set_cache_index_and_filter_blocks(true);
        // Bloom filter：10位/key，误判率~1%，大幅减少磁盘IO
        block_opts.set_bloom_filter(10.0, false);
        // 启用索引分区（大CF场景降低内存占用）
        block_opts.set_index_type(rocksdb::BlockBasedIndexType::TwoLevelIndexSearch);
        opts.set_block_based_table_factory(&block_opts);

        // ===== Prefix Extractor =====
        // KG的key设计为固定前缀（shard_id + entity_type），启用prefix_extractor
        // 使seek_prefix可以利用prefix bloom filter，避免全表扫描
        // 注意：prefix长度需要根据实际key设计调整，这里设为8字节（u64前缀）
        opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(8));

        // ===== 并行Compaction =====
        // 根据CPU核心数自动设置，最少4线程
        let parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(4);
        opts.increase_parallelism(parallelism as i32);
        opts.set_max_background_jobs(parallelism as i32);
        // 注意：不使用set_background_threads（部分版本API不稳定）
        // increase_parallelism已足够设置compaction线程池

        // ===== 其他优化 =====
        opts.set_max_open_files(-1); // 不限制打开文件数（生产环境）
        opts.set_keep_log_file_num(10);
        opts.set_max_log_file_size(64 * 1024 * 1024);
        // 优化点查：启用memtable前缀bloom
        opts.set_memtable_prefix_bloom_ratio(0.1);
        // 优化大范围scan：预读
        opts.set_compaction_readahead_size(2 * 1024 * 1024);

        opts
    }

    /// DB级 Options（全局）
    fn db_options() -> Options {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_compaction_style(DBCompactionStyle::Level);
        // 全局并行度
        let parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(4);
        opts.increase_parallelism(parallelism as i32);
        opts.set_max_background_jobs(parallelism as i32);
        opts.set_max_open_files(-1);
        opts
    }

    pub fn open_db(
        path: &Path,
        shard_ids: &[u16],
    ) -> StorageResult<(InnerDB, Arc<HashSet<u16>>, Arc<PathBuf>)> {
        let opts = db_options();
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
            .map(|n| ColumnFamilyDescriptor::new(n.clone(), cf_options()))
            .collect();
        let db = DB::open_cf_descriptors(&opts, path, cf_descs)
            .map_err(|e| StorageError::Internal(format!("rocksdb open: {e}")))?;
        let mut shards_set = HashSet::new();
        for &s in shard_ids {
            shards_set.insert(s);
            for n in cf_names_for(s) {
                if db.cf_handle(&n).is_none() {
                    db.create_cf(n, &cf_options())
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
                db.create_cf(n, &cf_options())
                    .map_err(|e| StorageError::Internal(format!("add cf {e}")))?;
            }
        }
        let mut s = (**shards).clone();
        s.insert(shard);
        Ok(Arc::new(s))
    }

    /// CF handle 缓存：避免每次调用 cf_handle() 都在内部HashMap查找
    /// 注意：CF创建后handle不变，可安全缓存
    pub struct CfCache {
        cache: parking_lot::Mutex<HashMap<String, *const ColumnFamily>>,
    }

    // CF handle是DB内部指针，DB存活期间有效；Send+Sync由DB本身保证
    unsafe impl Send for CfCache {}
    unsafe impl Sync for CfCache {}

    impl CfCache {
        pub fn new() -> Self {
            Self {
                cache: parking_lot::Mutex::new(HashMap::new()),
            }
        }
        /// 获取CF handle，缓存命中则直接返回指针
        pub fn get<'a>(&self, db: &'a DB, name: &str) -> StorageResult<&'a ColumnFamily> {
            let mut cache = self.cache.lock();
            if let Some(ptr) = cache.get(name) {
                // Safety: CF handle在DB生命周期内有效，指针来自db.cf_handle
                return Ok(unsafe { &**ptr });
            }
            let cf = db
                .cf_handle(name)
                .ok_or_else(|| StorageError::Internal(format!("cf not found: {name}")))?;
            let ptr = cf as *const ColumnFamily;
            cache.insert(name.to_string(), ptr);
            Ok(cf)
        }
    }

    /// 全局CF handle缓存
    static CF_CACHE: OnceLock<CfCache> = OnceLock::new();
    fn cf_cache() -> &'static CfCache {
        CF_CACHE.get_or_init(CfCache::new)
    }

    pub fn cf<'a>(db: &'a InnerDB, name: &str) -> StorageResult<&'a ColumnFamily> {
        cf_cache().get(db, name)
    }

    pub fn put_cf(db: &InnerDB, name: &str, k: &[u8], v: &[u8]) -> StorageResult<()> {
        let c = cf(db, name)?;
        db.put_cf_opt(c, k, v, write_opts())
            .map_err(|e| StorageError::Internal(format!("put {e}")))
    }

    pub fn get_cf(db: &InnerDB, name: &str, k: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let c = cf(db, name)?;
        db.get_cf(c, k)
            .map_err(|e| StorageError::Internal(format!("get {e}")))
    }

    /// MultiGet批量查询：一次FFI调用获取多个key，大幅减少FFI开销
    /// 适用于批量点查场景（如批量获取顶点属性）
    pub fn multi_get_cf(
        db: &InnerDB,
        name: &str,
        keys: &[&[u8]],
    ) -> StorageResult<Vec<Option<Vec<u8>>>> {
        let c = cf(db, name)?;
        // rocksdb 0.25 的 multi_get_cf 返回 Vec<Result<Option<Vec<u8>>, Error>>
        let results = db.multi_get_cf(c, keys);
        results
            .into_iter()
            .map(|r| r.map_err(|e| StorageError::Internal(format!("multi_get {e}"))))
            .collect()
    }

    pub fn delete_cf(db: &InnerDB, name: &str, k: &[u8]) -> StorageResult<()> {
        let c = cf(db, name)?;
        db.delete_cf_opt(c, k, write_opts())
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
        db.write_opt(b, write_opts())
            .map_err(|e| StorageError::Internal(format!("wb {e}")))
    }

    /// 优化版 seek_prefix：使用 iterate_upper_bound 避免扫描超出prefix范围
    /// 配合 prefix_extractor + bloom filter，可大幅减少磁盘IO
    pub fn seek_prefix(
        db: &InnerDB,
        n: &str,
        prefix: &[u8],
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let c = cf(db, n)?;
        let mut read_opts = ReadOptions::default();
        // 设置iterate_upper_bound = prefix + 1（字典序上界）
        // 使RocksDB在到达上界后自动停止，避免扫描无关数据
        let mut upper = prefix.to_vec();
        let mut carry = true;
        let mut i = upper.len();
        while carry && i > 0 {
            i -= 1;
            if upper[i] < 0xff {
                upper[i] += 1;
                carry = false;
            } else {
                upper[i] = 0;
            }
        }
        if !carry {
            read_opts.set_iterate_upper_bound(upper);
        }
        // prefix_same_as_start：确保迭代器只在相同prefix内移动
        read_opts.set_prefix_same_as_start(true);

        let mut iter = db.raw_iterator_cf_opt(c, read_opts);
        let mut out = Vec::new();
        iter.seek(prefix);
        while iter.valid() {
            if let Some(k) = iter.key() {
                if !k.starts_with(prefix) {
                    break;
                }
                let v = iter.value().unwrap_or_default().to_vec();
                out.push((k.to_vec(), v));
            } else {
                break;
            }
            iter.next();
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
        let mut read_opts = ReadOptions::default();
        // 大范围scan预读256KB
        read_opts.set_readahead_size(256 * 1024);
        let mut iter = db.raw_iterator_cf_opt(c, read_opts);
        let mut out = Vec::with_capacity(limit as usize);
        let mut skipped: u64 = 0;
        iter.seek_to_first();
        while iter.valid() {
            if skipped < offset {
                skipped += 1;
                iter.next();
                continue;
            }
            if out.len() >= limit as usize {
                break;
            }
            if let (Some(k), Some(v)) = (iter.key(), iter.value()) {
                out.push((k.to_vec(), v.to_vec()));
            }
            iter.next();
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
    /// MultiGet批量查询（内存模式实现）
    pub fn multi_get_cf(
        db: &InnerDB,
        n: &str,
        keys: &[&[u8]],
    ) -> StorageResult<Vec<Option<Vec<u8>>>> {
        let cf = db.cf.read();
        let map = cf.get(n);
        let results: Vec<Option<Vec<u8>>> = keys
            .iter()
            .map(|k| map.and_then(|m| m.get(*k).cloned()))
            .collect();
        Ok(results)
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

    /// MultiGet批量查询：一次FFI调用获取多个key，大幅减少FFI开销
    /// 适用于批量点查场景（如批量获取顶点属性、边属性）
    /// 性能收益：N个key的批量查询 vs N次单查，FFI开销从O(N)降至O(1)
    pub fn multi_get_cf(
        &self,
        cf: &str,
        keys: &[&[u8]],
    ) -> StorageResult<Vec<Option<Vec<u8>>>> {
        backend::multi_get_cf(&self.db, cf, keys)
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

    /// 优雅关闭：flush所有memtable到磁盘，等待compaction完成
    /// 企业级特性：确保进程退出前数据不丢失
    pub fn graceful_shutdown(&self) -> StorageResult<()> {
        #[cfg(feature = "persist-rocksdb")]
        {
            use rocksdb::WaitForCompactOptions;
            tracing::info!("KvEngine graceful shutdown: flushing memtables");
            // flush所有CF的memtable
            self.db
                .flush()
                .map_err(|e| StorageError::Internal(format!("flush: {e}")))?;
            // 等待compaction完成（最多等待60秒）
            let mut opts = WaitForCompactOptions::default();
            opts.set_timeout(60_000); // 60秒超时
            tracing::info!("KvEngine graceful shutdown: waiting for compaction");
            self.db
                .wait_for_compact(&opts)
                .map_err(|e| StorageError::Internal(format!("wait_for_compact: {e}")))?;
            tracing::info!("KvEngine graceful shutdown: completed");
        }
        #[cfg(not(feature = "persist-rocksdb"))]
        {
            tracing::info!("KvEngine graceful shutdown: memory mode, nothing to flush");
        }
        Ok(())
    }

    /// 健康检查：验证DB可读写
    /// 企业级特性：用于K8s liveness/readiness探针
    pub fn health_check(&self) -> StorageResult<HealthStatus> {
        let test_key = b"__health_check__";
        let test_value = b"ok";
        // 写入测试
        self.put_cf(
            cf_name_vid_meta(0).as_str(),
            test_key,
            test_value,
        )?;
        // 读取验证
        let val = self.get_cf(cf_name_vid_meta(0).as_str(), test_key)?;
        let ok = val.as_deref() == Some(test_value);
        // 清理
        self.delete_cf(cf_name_vid_meta(0).as_str(), test_key)?;
        if ok {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unhealthy("read-write verification failed".to_string()))
        }
    }

    /// 获取分片数量
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }
}

/// 健康检查状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }
}
