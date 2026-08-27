// rust-rocksdb FFI benchmark
// cargo bench -p mox-kg-storage-svc --features persist-rocksdb

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rocksdb::{Options, DB};
use tempfile::TempDir;

fn setup_db() -> (DB, TempDir) {
    let temp = TempDir::new().unwrap();
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_write_buffer_size(64 * 1024 * 1024);
    opts.set_max_write_buffer_number(4);
    let db = DB::open(&opts, temp.path()).unwrap();
    let mut batch = rocksdb::WriteBatch::default();
    for i in 0..10000u32 {
        let key = format!("key_{:08}", i).into_bytes();
        let value = format!("value_{:08}_data_padding_1234567890", i).into_bytes();
        batch.put(&key, &value);
    }
    db.write(batch).unwrap();
    db.flush().unwrap();
    db.compact_range::<&[u8], &[u8]>(None, None);
    (db, temp)
}

fn bench_single_get(c: &mut Criterion) {
    let (db, _temp) = setup_db();
    for i in 0..1000u32 {
        let key = format!("key_{:08}", i).into_bytes();
        let _ = db.get(&key).unwrap();
    }
    let mut group = c.benchmark_group("single_get");
    group.throughput(Throughput::Elements(1));
    group.bench_function("cached_hit", |b| {
        let mut i = 0u32;
        b.iter(|| {
            let key = format!("key_{:08}", i % 1000).into_bytes();
            let _ = db.get(&key).unwrap();
            i += 1;
        });
    });
    group.bench_function("cached_miss", |b| {
        let mut i = 0u32;
        b.iter(|| {
            let key = format!("miss_{:08}", i).into_bytes();
            let _ = db.get(&key).unwrap();
            i += 1;
        });
    });
    group.finish();
}

fn bench_batch_put(c: &mut Criterion) {
    let (db, _temp) = setup_db();
    let mut group = c.benchmark_group("batch_put");
    for &batch_size in &[10usize, 100, 1000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_function(format!("batch_{}", batch_size), |b| {
            let mut counter = 0u32;
            b.iter(|| {
                let mut batch = rocksdb::WriteBatch::default();
                for _ in 0..batch_size {
                    let key = format!("wkey_{:010}", counter).into_bytes();
                    let value = format!("wvalue_{:010}_data", counter).into_bytes();
                    batch.put(&key, &value);
                    counter += 1;
                }
                db.write(batch).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_multi_get(c: &mut Criterion) {
    let (db, _temp) = setup_db();
    for i in 0..1000u32 {
        let key = format!("key_{:08}", i).into_bytes();
        let _ = db.get(&key).unwrap();
    }
    let mut group = c.benchmark_group("multi_get");
    for &count in &[10usize, 100, 500] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(format!("multi_get_{}", count), |b| {
            let keys: Vec<Vec<u8>> = (0..count)
                .map(|i| format!("key_{:08}", i % 1000).into_bytes())
                .collect();
            let key_refs: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
            b.iter(|| {
                let _ = db.multi_get(&key_refs);
            });
        });
    }
    group.bench_function("loop_get_100", |b| {
        let keys: Vec<Vec<u8>> = (0..100)
            .map(|i| format!("key_{:08}", i % 1000).into_bytes())
            .collect();
        b.iter(|| {
            for k in &keys {
                let _ = db.get(k).unwrap();
            }
        });
    });
    group.finish();
}

fn bench_seek_prefix(c: &mut Criterion) {
    let (db, _temp) = setup_db();
    let mut group = c.benchmark_group("seek_prefix");
    group.bench_function("prefix_10k", |b| {
        b.iter(|| {
            let prefix = b"key_0000";
            let mut iter = db.raw_iterator();
            iter.seek(prefix);
            let mut count = 0;
            while iter.valid() {
                if let Some(k) = iter.key() {
                    if !k.starts_with(prefix) { break; }
                    count += 1;
                }
                iter.next();
            }
            assert_eq!(count, 10000);
        });
    });
    group.finish();
}

fn bench_scan(c: &mut Criterion) {
    let (db, _temp) = setup_db();
    let mut group = c.benchmark_group("scan");
    for &limit in &[100usize, 1000, 10000] {
        group.throughput(Throughput::Elements(limit as u64));
        group.bench_function(format!("scan_{}", limit), |b| {
            b.iter(|| {
                let mut iter = db.raw_iterator();
                iter.seek_to_first();
                let mut count = 0;
                while iter.valid() && count < limit {
                    let _ = iter.key();
                    let _ = iter.value();
                    count += 1;
                    iter.next();
                }
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_single_get, bench_batch_put, bench_multi_get, bench_seek_prefix, bench_scan);
criterion_main!(benches);
