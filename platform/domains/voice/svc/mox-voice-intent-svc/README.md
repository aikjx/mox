# mox-voice-intent-svc · 语音意图路由服务

小白语音 PPR（Pattern-Parse-Route）意图路由 Rust 实现，与 Python `intent/router.py` 功能 1:1 对齐。基于正则规则匹配中文语音指令，输出动作、类别、分数与参数的四元组路由结果。

## 功能特性

- **40+ 中文/拼音正则规则**：覆盖系统操作、音量控制、应用打开、文件操作等常见语音指令
- **40 条应用别名映射**：如"打开微信" → `open_app(app_name="wechat.exe")`
- **多候选排序与歧义检测**：按 score 降序排序，top1-top2 ≤ 阈值时触发联盟裁决
- **规则参数抽取**：自动提取数字音量、中文路径百分比、按键名、文件名等参数
- **纯函数式规则引擎**：`RuledRouter` 无状态，易于测试与扩展
- **async trait 适配**：`IntentRouterImpl` 实现引擎 trait，可直接注册到 voice engine

## 架构定位

本 crate 属于 MOX 平台 **voice 领域服务层**，位于：

```
platform/domains/voice/
├── api/                    ← trait 契约层
├── core/                   ← 核心领域逻辑
└── svc/
    └── mox-voice-intent-svc/  ← 本 crate（意图路由服务）
```

- 向上：被 voice 引擎调用，将用户语音文本解析为结构化动作指令
- 向下：依赖 `mox-voice-core-svc` 的 engine 模块（`IntentRouter` trait、`RoutedAction` 等）
- 定位：语音领域的"语义理解"层，桥接 ASR 识别结果与系统算子执行

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-voice-intent-svc = { path = "../svc/mox-voice-intent-svc" }
```

### 基本用法

```rust
use mox_voice_intent_svc::{DefaultRouter, RuledRouter};
use mox_voice_core_svc::engine::IntentRouter;

// 使用默认路由（内置全部规则 + 应用别名）
let router = DefaultRouter::new();

// 异步 dispatch（引擎接口）
let actions = router.dispatch("把音量调到 50%").await?;
for action in &actions {
    println!("动作: {}, 类别: {}, 分数: {}", action.name, action.category, action.score);
}

// 纯函数式用法
let ruled = RuledRouter::default();
let results = ruled.dispatch("打开微信");
```

## 核心模块 / 类型

### `rules` 模块
- `build_rule_set() -> Vec<Rule>` — 构建完整规则集
- `APP_ALIAS_EXACT_LIST` — 应用别名精确匹配列表
- `COMMON_KEY_NAMES` — 常见按键名映射

### `router` 模块
- `RuledRouter` — 纯函数式规则路由引擎，同步 dispatch
- `IntentRouterImpl` — 异步 trait 实现，适配引擎接口

### 默认实现
- `DefaultRouter` — 对外默认路由实现，内置 RULE_REGEXES + APP_ALIAS
- `DefaultRouter::new()` — 创建默认路由实例

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>
