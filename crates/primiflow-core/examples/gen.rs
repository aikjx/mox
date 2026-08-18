//! 关联图谱 → 代码/数据骨架 生成器示例
//!
//! 运行：`cargo run -p primiflow --example gen`
//!
//! 流程：
//! 1. 从 `primiflow/SPEC.md` 种子构建关联图谱
//! 2. 强制「一一对应」校验（任一链路断裂即 panic）
//! 3. 生成 `src/gen/*`（代码骨架 + 数据 schema + DDL + Mermaid + 溯源矩阵）

use primiflow_core::assoc::primiflow_seed;
use primiflow_core::generate::emit_all;
use std::path::Path;

fn main() {
    let graph = primiflow_seed();

    // 1) 一一对应校验：不满足就停下，绝不生成半成品
    let errs = graph.validate_correspondence();
    assert!(
        errs.is_empty(),
        "关联图谱不满足一一对应不变量:\n{:?}",
        errs
    );
    println!(
        "[primiflow] 关联图谱校验通过：{} 节点 / {} 边，全链路一一对应。",
        graph.nodes.len(),
        graph.edges.len()
    );

    // 2) 生成落地产物到 src/gen
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = Path::new(manifest).join("src/gen");
    emit_all(&graph, &out).expect("生成失败");

    println!(
        "[primiflow] `src/gen/*` 已生成并挂载到 crate（lib.rs 中 `pub mod gen;` 已启用）。\n\
         \x20  如需重新生成：改 `assoc::primiflow_seed` 后再次 `cargo run -p primiflow --example gen`。"
    );
}
