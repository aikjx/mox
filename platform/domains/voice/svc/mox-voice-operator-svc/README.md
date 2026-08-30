# mox-voice-operator-svc · 语音系统算子服务

FR-13 8 大类系统算子 Rust 权威实现，与 Python `operator/` 下各模块 1:1 对齐。每个动作都实现 1~N 层跨平台回退链，阻塞系统调用封装在 `execute()` 内由 Engine 通过 `spawn_blocking` 隔离。

## 功能特性

- **8 大类系统算子**：应用、文件、音量、输入、网络、显示、浏览器、通知
- **跨平台回退链**：Windows/windows-rs/CoreAudio → macOS/osascript → Linux/pactl/xdg-open
- **一行注册全量算子**：`register_all_defaults(&engine)` 快速构建默认算子矩阵
- **阻塞调用隔离**：所有 Win32/Shell 阻塞调用封装独立，配合 `spawn_blocking` 使用
- **语音引擎集成**（feature `voice-engine`）：录音器、语音引擎、数字人 Avatar 系统
- **3717 端口服务**（feature `server-3717`）：完整 HTTP 服务、路由构建与阻塞运行

## 架构定位

本 crate 属于 MOX 平台 **voice 领域服务层**，位于：

```
platform/domains/voice/
├── api/                    ← trait 契约层
├── core/                   ← 核心领域逻辑
└── svc/
    └── mox-voice-operator-svc/  ← 本 crate（系统算子服务）
```

- 向上：被 voice 引擎 / BallWidget / voice_proxy 调用，执行实际系统操作
- 向下：依赖 `mox-voice-core-svc` 的 engine 模块（`OperatorEngine`）
- 定位：语音领域的"执行层"，将意图路由输出的动作指令转化为实际系统行为

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-voice-operator-svc = { path = "../svc/mox-voice-operator-svc" }
```

可选 features：
- `voice-engine` — 启用语音引擎、录音器与 Avatar 系统
- `server-3717` — 启用 3717 端口 HTTP 服务

### 基本用法

```rust
use mox_voice_operator_svc::register_all_defaults;
use mox_voice_core_svc::engine::OperatorEngine;

// 创建算子引擎并注册全部 8 大类算子
let engine = OperatorEngine::new();
register_all_defaults(&engine);

// 执行操作（由 Engine 通过 spawn_blocking 调用阻塞操作）
// engine.execute("open_app", params).await?;
```

使用语音引擎（需 feature `voice-engine`）：

```rust
use mox_voice_operator_svc::{VoiceEngine, Recorder};

let mut voice_engine = VoiceEngine::new();
voice_engine.start_listening()?;
```

启动 HTTP 服务（需 feature `server-3717`）：

```rust
use mox_voice_operator_svc::{serve, VoiceServiceConfig};

let config = VoiceServiceConfig::default();
serve(config).await?;
```

## 核心模块 / 类型

### 算子模块（8 大类）
- `app::AppOperator` — 应用操作算子（打开、关闭、切换等）
- `file::FileOperator` — 文件操作算子（打开、复制、删除等）
- `volume::VolumeOperator` — 音量控制算子（调节、静音、获取音量等）
- `input::InputOperator` — 输入模拟算子（键盘、鼠标输入）
- `network::NetworkOperator` — 网络操作算子（Wi-Fi、代理等）
- `display::DisplayOperator` — 显示操作算子（亮度、分辨率等）
- `browser::BrowserOperator` — 浏览器操作算子（打开网址、搜索等）
- `notify::NotifyOperator` — 通知操作算子（发送系统通知）

### 辅助模块
- `helpers` — 平台检测等通用工具
- `helpers::platform_tag() -> &'static str` — 获取当前平台标签

### 语音引擎（feature = "voice-engine"）
- `voice_engine::VoiceEngine` — 语音引擎主结构体
- `voice_engine::Recorder` — 音频录音器
- `avatar::Avatar` — 数字人 Avatar
- `avatar::AvatarMeta` — Avatar 元数据
- `avatar::AvatarRegistry` — Avatar 注册表
- `avatar::PersonaConf` — 人设配置
- `avatar::VisualConf` — 视觉配置
- `avatar::VoiceConf` — 语音配置

### 3717 服务（feature = "server-3717"）
- `server_3717::XiaobaiVoiceService` — 小白语音服务
- `server_3717::VoiceServiceConfig` — 服务配置
- `server_3717::serve` — 启动服务（async）
- `server_3717::build_router` — 构建 Axum 路由
- `server_3717::run_service_blocking` — 阻塞式运行服务

### 注册函数
- `register_all_defaults(engine: &OperatorEngine)` — 注册全部 8 大类默认算子

## License

Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟

Licensed under the MIT License.

- GitHub 主仓: <https://github.com/aikjx/mox.git>
- GitCode 镜像: <https://gitcode.com/aikjx/mox>
