//! operator-core 性能基线（关图骨架定义 §6 P4：benches 基线）。
//!
//! `Cargo.toml` 早已声明 `[[bench]] operator_benches`，但基准文件缺失，
//! 导致整个 workspace 无法解析 manifest。本文件补齐该声明，
//! 并对状态向量与守恒校验这两条热路径建立可回归的性能基线。

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use operator_core::state::StateVector;

/// 状态向量构造：维度扫描，确认分配开销随维度线性
fn bench_state_alloc(c: &mut Criterion) {
    let mut g = c.benchmark_group("state_alloc");
    for dim in [64usize, 1024, 8192] {
        g.throughput(Throughput::Elements(dim as u64));
        g.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, &d| {
            b.iter(|| StateVector::zeros(black_box(d)));
        });
    }
    g.finish();
}

/// 向量代数热路径：norm / dot / scale / add
fn bench_state_algebra(c: &mut Criterion) {
    let dim = 1024usize;
    let a = StateVector::from_vec((0..dim).map(|i| i as f64 / dim as f64).collect());
    let b_vec = StateVector::from_vec((0..dim).map(|i| (dim - i) as f64 / dim as f64).collect());

    let mut g = c.benchmark_group("state_algebra");
    g.throughput(Throughput::Elements(dim as u64));

    g.bench_function("norm_l2", |bch| bch.iter(|| black_box(&a).norm()));
    g.bench_function("norm_l1", |bch| bch.iter(|| black_box(&a).norm_l1()));
    g.bench_function("dot", |bch| {
        bch.iter(|| {
            black_box(&a)
                .dot(black_box(&b_vec))
                .expect("同维点积不会失败")
        })
    });
    g.bench_function("scale", |bch| {
        bch.iter(|| black_box(&a).scale(black_box(1.5)))
    });
    g.bench_function("add", |bch| {
        bch.iter(|| {
            black_box(&a)
                .add(black_box(&b_vec))
                .expect("同维相加不会失败")
        })
    });
    g.finish();
}

/// 归一化：概率归一是守恒律检查前的必经步骤
fn bench_normalize(c: &mut Criterion) {
    let dim = 1024usize;
    let base: Vec<f64> = (0..dim).map(|i| (i as f64).mul_add(0.5, 1.0)).collect();

    let mut g = c.benchmark_group("normalize");
    g.throughput(Throughput::Elements(dim as u64));
    g.bench_function("l2", |bch| {
        bch.iter_batched(
            || StateVector::from_vec(base.clone()),
            |mut v| {
                v.normalize();
                v
            },
            criterion::BatchSize::SmallInput,
        )
    });
    g.bench_function("probability", |bch| {
        bch.iter_batched(
            || StateVector::from_vec(base.clone()),
            |mut v| {
                v.normalize_probability();
                v
            },
            criterion::BatchSize::SmallInput,
        )
    });
    g.finish();
}

/// 残差计算：守恒律闸门的核心度量（对齐 PT-Primi A3 残差判定）
fn bench_residual(c: &mut Criterion) {
    let dim = 1024usize;
    let a = StateVector::from_vec(vec![1.0; dim]);
    let b = StateVector::from_vec(vec![1.0 + 1e-12; dim]);

    let mut g = c.benchmark_group("conservation");
    g.throughput(Throughput::Elements(dim as u64));
    g.bench_function("residual", |bch| {
        bch.iter(|| {
            black_box(&a)
                .residual(black_box(&b))
                .expect("同维残差不会失败")
        })
    });
    g.finish();
}

/// 算子 ID 生成：注册链路的高频调用
fn bench_operator_id(c: &mut Criterion) {
    c.bench_function("generate_operator_id", |b| {
        b.iter(operator_core::generate_operator_id)
    });
}

criterion_group!(
    benches,
    bench_state_alloc,
    bench_state_algebra,
    bench_normalize,
    bench_residual,
    bench_operator_id
);
criterion_main!(benches);
