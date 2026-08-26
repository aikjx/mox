//! 开发专家视角性能基线（P4 · benches 基线）
//!
//! 覆盖 PrimiFlow 融合归一化层四条最热路径：
//! 1. `fuse_all` —— 13 crate 能力 + 6 张数据表一次性融合成统一图；
//! 2. `PrimiPlatform::synthesize` —— 主链路编排 + 六维登记 + 重建图 + 全局闸门；
//! 3. `UnifiedGraph::full_gate` —— 守恒残差(R07) + 六维零孤儿(A4) + GR-STD 8 闸门；
//! 4. `SixDimRegistry` 累积与溯源 —— 注册 + 按 code 反查需求。
//!
//! 由 CI（`cargo bench --bench development_experts -- --save-baseline <sha>`）持续采集，
//! 任何 >10% 的性能回归在 `performance-regression` 阶段被捕获。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mox_flow_fusion_svc::sixdim::{SixDimBinding, SixDimRegistry};
use mox_flow_fusion_svc::unified::PrimitiveCoords;
use mox_flow_fusion_svc::{fuse_all, PrimiPlatform};

/// 构造一条完整的六维绑定（与 sixdim 测试 `sample` 同形，bench 自包含）
fn sample(i: u64) -> SixDimBinding {
    let k = (i as f64) * 0.01 + 0.3;
    let t = (i as f64) * 0.007 + 0.2;
    let c = (k * k + t * t).sqrt();
    SixDimBinding {
        req_id: format!("REQ-BENCH-{i}"),
        req_text: "bench requirement".into(),
        project_id: format!("P-{i}"),
        status: "Completed".into(),
        coords: PrimitiveCoords {
            kappa: k,
            tau: t,
            c,
            q: 1.0,
        },
        requirement: format!("REQ-BENCH-{i}"),
        feature: format!("FUN-BENCH-{i}"),
        business: format!("BIZ-BENCH-{i}"),
        algorithm: format!("ALG-BENCH-{i}"),
        task: format!("TSK-BENCH-{i}"),
        code: format!("COD-BENCH-{i}"),
        topo_nodes: 5,
        timestamp_ms: 0,
    }
}

fn bench_fuse_all(c: &mut Criterion) {
    c.bench_function("fuse_all", |b| {
        b.iter(|| {
            let _ = fuse_all();
        })
    });
}

fn bench_synthesize(c: &mut Criterion) {
    let mut p = PrimiPlatform::new();
    c.bench_function("synthesize", |b| {
        b.iter(|| {
            // 每次用唯一需求文本，确保注册表累积（贴近真实运行态）
            p.synthesize(black_box("抓取销售数据、清洗对账、生成图表报告"), 0.2);
        })
    });
}

fn bench_full_gate(c: &mut Criterion) {
    // 预置一个含 8 条累积绑定的平台，单独压测全局闸门本身
    let mut p = PrimiPlatform::new();
    for i in 0..8u64 {
        p.synthesize(&format!("需求{i}：抓取数据清洗并出图"), 0.2);
    }
    let g = &p.graph;
    c.bench_function("full_gate", |b| {
        b.iter(|| {
            let _ = g.full_gate();
        })
    });
}

fn bench_registry_register_and_query(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU64, Ordering};
    let mut reg = SixDimRegistry::new();
    let counter = AtomicU64::new(0);
    c.bench_function("registry_register_and_query", |b| {
        b.iter(|| {
            let i = counter.fetch_add(1, Ordering::Relaxed);
            let binding = sample(black_box(i));
            reg.register(binding);
            let _ = reg.by_code("COD-BENCH-1");
        })
    });
}

criterion_group!(
    benches,
    bench_fuse_all,
    bench_synthesize,
    bench_full_gate,
    bench_registry_register_and_query
);
criterion_main!(benches);
