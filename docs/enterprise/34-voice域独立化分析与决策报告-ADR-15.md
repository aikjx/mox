# Voice 域独立化分析与决策报告（VOICE DOMAIN INDEPENDENCE · ADR-15）

> **文档身份**：voice 域是否从模块化单体中拆分为独立产品/独立服务的决策分析报告。基于 voice 域的架构特征、依赖关系、发布周期、业务定位，给出明确的独立化决策与演进路径。
> **版本**：v1.0 ENT（2026-08-26 · 开发专家联盟）
> **权威链**：`18` TOP-MASTER > `02` 架构七视图 > `29` 跨域依赖规则 > **本文件(ADR-15)**
> **关联 ADR**：ADR-09(voice域特殊规则)、ADR-16(微服务演进)

---

## §1 Voice 域现状

### 1.1 Crate 清单

| Crate | 层 | 职责 | 依赖 |
|-------|----|------|------|
| `mox-voice-asr-core` | core | 语音识别核心（ASR 算法/模型适配） | platform-foundation |
| `mox-voice-tts-core` | core | 语音合成核心（TTS 算法/模型适配） | platform-foundation |
| `mox-voice-intent-core` | core | 语音意图识别（NLU） | platform-foundation, ai-core? |
| `mox-voice-dsp-core` | core | 数字信号处理（降噪/VAD/回声消除） | platform-foundation |
| `mox-voice-core-svc` | svc | 语音服务编排（ASR→NLU→TTS 流水线） | ai-expert-svc, flow-operator-core ⚠️ |
| `mox-voice-desktop-app` | api/app | 桌面应用（Tauri + 前端） | voice-core-svc, platform |
| `mox-voice-dsp-py` | sdk | Python DSP 绑定（语音预处理） | — |

### 1.2 架构特征

| 特征 | 描述 |
|------|------|
| **独立 UI 层** | `mox-voice-desktop-app` 是完整的桌面应用（Tauri + Web 前端），有独立的构建/打包/发布流程 |
| **独立发布周期** | 桌面应用需要按客户端版本发布（如 v1.2.3），与后端服务版本解耦 |
| **可独立运行** | 本地 ASR/TTS 模式下，桌面应用可脱离主系统独立运行（离线语音助手） |
| **特殊技术栈** | 包含 Python 绑定（`mox-voice-dsp-py`）、音频处理库、Tauri 桌面框架，与纯 Rust 后端差异大 |
| **准独立依赖** | 当前仅 `mox-voice-core-svc` 违规依赖 ai/flow 域（arch test 2 项 P0），其余 crate 仅依赖 platform |

### 1.3 业务定位

| 定位 | 描述 |
|------|------|
| **产品形态** | 独立桌面应用（语音助手）+ 后端语音服务（API） |
| **目标用户** | 需要语音交互的终端用户（桌面端）+ 需要语音 API 的开发者 |
| **收入模式** | 桌面应用可能独立收费 / 语音 API 按调用量计费 |
| **与主系统关系** | 语音是主系统的一个**输入/输出通道**，但桌面应用本身是独立产品 |

---

## §2 独立化决策分析

### 2.1 独立化驱动力（WHY SPLIT）

| 驱动力 | 权重 | 描述 |
|--------|:----:|------|
| **发布周期解耦** | 高 | 桌面应用需要频繁迭代（UI/体验），后端服务稳定优先，混在一起互相阻塞 |
| **技术栈差异** | 高 | Tauri + Python + 音频处理 vs 纯 Rust 后端，构建工具链完全不同 |
| **团队独立** | 中 | 语音团队可独立迭代，不依赖后端发布窗口 |
| **独立部署** | 中 | 桌面应用本地运行，语音服务可独立扩缩容 |
| **安全隔离** | 低 | 桌面应用需要访问麦克风/音频设备，与后端服务权限隔离 |

### 2.2 独立化阻力（WHY NOT SPLIT）

| 阻力 | 权重 | 描述 |
|--------|:----:|------|
| **共享 AI 能力** | 高 | 语音意图识别依赖 AI 域的专家系统/Agent，拆分后需通过 API 调用 |
| **共享图谱能力** | 中 | 语音查询可能需要图谱检索，拆分后需经 kg-sdk |
| **运维复杂度** | 中 | 独立部署意味着独立的 CI/CD、监控、告警 |
| **数据一致性** | 低 | 语音会话历史可能需要与主系统用户数据关联 |

### 2.3 三种方案对比

| 维度 | 方案A：保留在单体 | 方案B：独立服务(Monorepo) | 方案C：独立产品(独立Repo) |
|------|:-----------------:|:-------------------------:|:-------------------------:|
| 发布解耦 | ❌ 耦合 | ✅ 服务独立发布 | ✅ 完全独立 |
| 技术栈统一 | ✅ 统一 | ⚠️ 桌面应用特殊 | ❌ 完全分离 |
| AI/图谱共享 | ✅ 直接调用 | ⚠️ 经 SDK/API | ❌ 经远程 API |
| 运维复杂度 | ✅ 简单 | ⚠️ 中等 | ❌ 高 |
| 团队协作 | ✅ 紧密 | ⚠️ 需接口契约 | ❌ 完全解耦 |
| 桌面应用打包 | ❌ 与后端混 | ✅ 独立打包 | ✅ 独立打包 |
| 迁移成本 | ✅ 零 | ⚠️ 中等 | ❌ 高 |

### 2.4 决策结论

**推荐方案 B：独立服务 + Monorepo（准独立）**

**理由**：
1. voice 域的核心价值（ASR/TTS/NLU）是**后端服务能力**，应保留在 Monorepo 中共享 AI/图谱能力
2. `mox-voice-desktop-app` 作为**独立客户端**，有独立的构建/发布流程，但代码仍在 Monorepo 中（便于共享类型定义）
3. 通过依赖规则确保 voice 域**准独立**（不被其他域依赖，也不依赖其他域的 svc/core）
4. 未来如需完全独立产品，可平滑迁移到独立 Repo（因为依赖已隔离）

**不推荐方案 C 的原因**：
- voice 域的意图识别深度依赖 AI 域的专家系统，远程 API 调用增加延迟和复杂度
- 独立 Repo 的运维成本过高，当前团队规模不支持
- Monorepo 中的代码共享（类型、SDK、工具）效率更高

---

## §3 准独立架构设计

### 3.1 依赖隔离

**voice 域允许的依赖**：
- ✅ `mox-foundation-*`（基础工具）
- ✅ `mox-platform-*`（平台底座：iam、meta、system）
- ✅ `mox-ai-sdk`（经 SDK 调用 AI 能力，Phase 3 创建）
- ✅ `mox-kg-sdk`（经 SDK 调用图谱能力，Phase 3 创建）
- ❌ 任何域的 `svc`/`core`（除 platform）

**voice 域不被任何其他域依赖**（voice 是终端能力，不提供服务给其他域）

### 3.2 当前违规修复

| 违规依赖 | 修复方案 | 优先级 |
|----------|----------|:------:|
| `mox-voice-core-svc` → `mox-ai-expert-svc` | 创建 `mox-ai-sdk`，voice 经 sdk 调用专家校验 | P1 |
| `mox-voice-core-svc` → `mox-flow-operator-core` | 评估是否真的需要 flow 算子；如需要，经 `mox-flow-sdk` | P1 |

### 3.3 桌面应用独立化

`mox-voice-desktop-app` 的独立化措施：

| 措施 | 描述 |
|------|------|
| **独立构建脚本** | `platform/domains/voice/app/mox-voice-desktop-app/` 下有独立的 `build.rs`、`tauri.conf.json`、前端 package.json |
| **独立 CI Job** | GitHub Actions 中独立的 `voice-desktop-build` job，不依赖后端构建 |
| **独立版本号** | 桌面应用版本号独立于后端（如 `v1.2.3`），通过 `tauri.conf.json` 管理 |
| **独立发布渠道** | 桌面应用通过 GitHub Releases / 应用商店发布，不随后端 Docker 镜像发布 |
| **类型共享** | 桌面应用与后端共享的类型定义放在 `mox-voice-sdk` 中，通过依赖引用 |

### 3.4 语音服务 API

voice 域后端服务通过标准 REST API 暴露（Phase 3 创建 `mox-voice-api`）：

| 端点 | 描述 |
|------|------|
| POST /api/v1/voice/asr | 语音识别（上传音频→文本） |
| POST /api/v1/voice/tts | 语音合成（文本→音频） |
| POST /api/v1/voice/intent | 语音意图识别（音频→结构化意图） |
| WS /api/v1/voice/stream | 流式语音交互（ASR→NLU→TTS 实时流水线） |
| GET /api/v1/voice/models | 可用语音模型列表 |

---

## §4 演进路线图

| 阶段 | 时间 | 交付物 | 验收 |
|------|------|--------|------|
| 3.1 | 第5周 | 修复 voice 域违规依赖（经 ai-sdk/flow-sdk） | arch_test voice 相关 P0=0 |
| 3.2 | 第5-6周 | 创建 `mox-voice-sdk`（DTO + 客户端 trait） | 桌面应用经 sdk 调用后端 |
| 3.3 | 第6周 | 桌面应用独立 CI/CD + 独立版本号 | 桌面应用可独立构建发布 |
| 3.4 | 第6-7周 | 创建 `mox-voice-api`（REST + WebSocket） | 语音服务 API 可独立访问 |
| 3.5 | 第7-8周 | 语音服务可独立部署（Docker 镜像） | voice 服务可单独扩缩容 |
| 后续 | — | 评估是否迁移到独立 Repo（方案C） | 当 voice 团队 >5 人或需要独立融资时 |

---

## §5 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| AI SDK 未就绪导致 voice 无法调用 AI | voice 功能受限 | Phase 3 优先创建 ai-sdk，voice 可临时用 HTTP 调用 ai-api |
| 桌面应用与后端类型不一致 | 接口联调困难 | 类型统一定义在 mox-voice-sdk，两端共享 |
| 流式语音 WebSocket 性能 | 实时性要求高 | 独立部署 voice 服务，就近接入，WebSocket 连接池 |
| 未来完全独立时迁移成本 | 代码拆分困难 | 现在就保持依赖隔离，未来拆分只需移动目录 |

---

## §6 决策记录（ADR）

**决策 ID**：ADR-15
**日期**：2026-08-26
**决策**：voice 域采用**准独立架构**（方案B）—— 保留在 Monorepo 中，但通过依赖规则确保不被其他域依赖、也不依赖其他域 svc/core；桌面应用独立构建/发布；语音服务可独立部署。
**理由**：平衡了发布解耦需求与 AI/图谱能力共享需求，迁移成本可控，未来可平滑升级为完全独立产品。
**关联**：ADR-09 §4（voice 域特殊规则）、ADR-16（微服务演进）

---

## 变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0 | 2026-08-26 | 首版：现状分析+三方案对比+准独立决策+依赖隔离+演进路线+风险 | 开发专家联盟 |
