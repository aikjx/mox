# operator-wasm · WASM 字节码算子沙箱

## §1 · 概述
璇玑算子系统（L4Services）的**第三方算子安全沙箱**：基于 wasmer + cranelift AOT，把未可信用户提供的 `.wasm` 算子字节码载入受限内存执行；严格控制 CPU 指令、内存配额（线性内存 128MB 封顶）与系统调用（仅开放 `env::op_input / env::op_output` 两组桥接函数）。

## §2 · CRATE_ID / ENGINE_NAME / AIS 层级
归属 **AIS Layer = L4Services**。

```rust
pub const CRATE_ID: &str = "5a1df407-b217-5340-a5ae-5f4535d1e6de";
pub const ENGINE_NAME: &str = "xuanji::operator_wasm";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
};
```

## §3 · 模块结构 src/* 说明
| 文件 | 职责 |
|------|------|
| `src/lib.rs` | 三常量 + WASM Operator 主实现；对外实现 `operator_core::Operator` trait（使其可在 operator-core Registry 注册） |
| （内部依赖 `wasmer` + `wasmer-compiler-cranelift` workspace 继承） | 外部依赖仅限 workspace 继承的 wasmer 家族，禁止引入 wabt/wasmparser 等其他重型 Wasm 生态解析器（与单一真源 wasmer 冲突） |

## §4 · 关键 Trait & Impl
- **`pub trait WasmHost`**：定义宿主侧环境函数；默认 impl 包含 `op_input(idx) -> *const u8` / `op_output(ptr, len)` / `op_cancel()` 桥接。
- **`pub struct WasmOperator`**：持有 wasmer::Store + Instance；`impl operator_core::Operator for WasmOperator`（把 WASM 算子对接到 operator-core Registry）。
- **`pub struct WasmModule`**：字节码编译缓存（cranelift 产出 Module artifact）；可序列化复用避免重复编译。
- **`pub struct Instance`**：单实例运行句柄（每次 run 新 Instance 隔离、线性内存 128MB 封顶）。

## §5 · 跑单测指引
```bash
cargo test -p operator-wasm
```
断言覆盖：`.wasm` 非法字节码被 reject（非 wasm magic）、线性内存 >128MB 触发 OOM 且无 UB、`op_input/op_output` 往返一致性、`op_cancel()` 指令计数超出上限主动终止、WasmOperator 作为 Operator 注册到 operator-core Registry 成功并 run_op 返回预期。

## §6 · 二次开发 / DIP 反转指引
- **新增宿主函数**：实现 `trait WasmHost` 的 `fn register_imports(store: &mut Store, imports: &Imports)` → 注入新的 import。不得改 `src/lib.rs` 硬编码 env:: 名字。
- **WASM 编译缓存策略**：实现 `trait ModuleCache` trait（可选）→ 替换默认 LRU。

## §7 · TDD RED→GREEN 工作流 + 精度护栏
**流程**：① RED：先写恶意 wasm（OOM / 死循环 / 尝试文件系统 syscall）→ 测试应被沙箱拒绝；② GREEN：对应拒绝逻辑实现。
**精度护栏**：WASM 桥接函数的 i64 参数一律按无符号 u64 解释再转，绝不直接解释为有符号 i64（避免被恶意负数溢出绕过长度检查）；内存配额 128MB = 134,217,728 字节，硬常量不可配置（通过编译时 const 断言）。

## §8 · 图谱绑定（三注册 key + self_sync 规则）
```
domain id      : domain-rust-operator-wasm
engine id      : module-rust-operator-wasm
code_graph unit: operator-wasm
```
self_sync：改 `src/lib.rs` 三常量 / trait → `self_sync_rust.js` 刷新三注册。
