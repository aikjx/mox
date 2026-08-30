# mox-voice-asr-svc · 语音识别热词注入服务

FR-5 热词三层注入 Rust 实现，与 Python `asr/hotwords.py` 功能 1:1 对齐。提供热词验证排序、模型层注入、热词文件重建以及后处理模糊替换的完整三层注入链路。

## 功能特性

- **热词验证与排序**：Levenshtein 去重、max_score 饱和、max_length 检查、黑名单 PII 警告
- **S1 模型层注入**：通过 feature-gate 接入 `sherpa-rs` ContextConfig，支持运行时 FFI 探测降级
- **S2 热词文件重建**：生成 UTF-8 `word\tscore\n` 格式热词文件，供外部引擎重建识别器
- **S3 后处理模糊替换**：对识别结果进行模糊匹配替换，输出命中热词列表
- **分层状态报告**：`InjectionReport` 详细记录每一层注入状态与降级原因
- **灵活的 feature 控制**：默认不依赖 sherpa-onnx-sys，避免 CI 编译过重

## 架构定位

本 crate 属于 MOX 平台 **voice 领域服务层**，位于：

```
platform/domains/voice/
├── api/                    ← trait 契约层
├── core/                   ← 核心领域逻辑
└── svc/
    └── mox-voice-asr-svc/  ← 本 crate（ASR 热词注入服务）
```

- 向上：被 voice 引擎 / operator 服务调用，为 ASR 识别提供热词增强能力
- 向下：依赖 `mox-voice-core-svc` 的 hotword 模块（验证、模糊匹配）
- 横向：可选接入 `sherpa-rs` 进行模型层热词注入（通过 feature `sherpa-real` 启用）

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-voice-asr-svc = { path = "../svc/mox-voice-asr-svc" }
```

如需启用真实 sherpa-rs 模型层注入：

```toml
[dependencies]
mox-voice-asr-svc = { path = "../svc/mox-voice-asr-svc", features = ["sherpa-real"] }
```

### 基本用法

```rust
use mox_voice_asr_svc::{HotwordInjector, default_injector};

// 创建默认注入器（无热词）
let mut injector = default_injector();

// 设置热词列表
injector.set_hotwords(vec![
    ("张三".into(), 3.0),
    ("李四".into(), 2.5),
]);

// 执行三层注入
let report = injector.inject_all()?;
println!("S1 模型层状态: {:?}", report.s1_status);
println!("S2 文件层状态: {:?}", report.s2_status);
println!("S3 后处理层状态: {:?}", report.s3_status);

// 获取热词文件路径（供引擎重建识别器）
let hotword_file = injector.hotword_file_path();

// 对识别结果进行后处理模糊替换
let (corrected_text, hits) = injector.apply_post_hoc("张3说的话")?;
```

## 核心模块 / 类型

### `injector` 模块
- `HotwordInjector` — 热词注入器主结构体，管理三层注入流程
- `InjectionReport` — 注入结果报告，记录每层状态与详情
- `HotwordLayerStatus` — 单层注入状态枚举（Active / Degraded / Skipped / Error）

### `ffi_probe` 模块
- `sherpa_rs_context_config_available() -> bool` — 运行时探测 sherpa-rs 是否可链接

### 辅助函数
- `default_injector() -> HotwordInjector` — 一行创建默认注入器

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>
