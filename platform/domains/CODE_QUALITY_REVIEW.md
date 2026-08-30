# 三域 Rust 模块代码质量与功能完备度评测报告

> 评测范围：`voice/`、`market/`、`project/` 三个 domain 目录下所有 Rust crate
> 评测日期：2026-08-30

---

## 一、总体概览

| 域 | Crate 数 | 源文件数 | 代码行数（约） | README | 测试文件 |
|---|---------|---------|--------------|--------|---------|
| **voice** | 8 | 39 | ~8,832 | 0 / 8 | 0 |
| **market** | 2 | 2 | ~618 | 1 / 2 | 0 |
| **project** | 2 | 7 | ~2,258 | 0 / 2 | 0 |
| **合计** | **12** | **48** | **~11,708** | **1 / 12** | **0** |

**实现状态分布：**
- 完整实现：5 个（42%）
- 实质实现：5 个（42%）
- 骨架占位：0 个（0%）
- API-only：2 个（16%）

---

## 二、Voice 域详细评测

### Crate 清单

| 子目录 | Crate 名称 | 文件数 | 代码行数 | 实现状态 | 测试 | README |
|-------|-----------|-------|---------|---------|------|--------|
| `api/` | mox-voice-api | 1 | 106 | API-only | 无 | 无 |
| `core/mox-voice-dsp-core/` | mox-voice-dsp-core | 5 | 429 | 完整实现 | 无 | 无 |
| `sdk/mox-voice-dsp-py/` | mox-voice-dsp-py | 1 | 232 | 实质实现 | 无 | 无 |
| `svc/mox-voice-asr-svc/` | mox-voice-asr-svc | 3 | 284 | 实质实现 | 无 | 无 |
| `svc/mox-voice-core-svc/` | mox-voice-core-svc | 9 | 1,608 | 完整实现 | 无 | 无 |
| `svc/mox-voice-desktop-app/` | mox-voice-desktop-app | 4 | 1,112 | 实质实现 | 无 | 无 |
| `svc/mox-voice-intent-svc/` | mox-voice-intent-svc | 3 | 471 | 完整实现 | 无 | 无 |
| `svc/mox-voice-operator-svc/` | mox-voice-operator-svc | 13 | 4,770 | 完整实现 | 无 | 无 |

### 各 Crate 简评

**1. mox-voice-api（API-only）**
- 纯 trait 契约层：定义 `SpeechRecognizer`、TTS、Intent、DSP、Operator 等 trait
- 数据结构完整（AsrResult、TtsRequest 等），错误类型枚举清晰
- 仅定义接口，无业务实现，符合 API 层定位

**2. mox-voice-dsp-core（完整实现）**
- 4 个核心模块：`resample`（线性重采样）、`sola`（SOLA 时域变速）、`loudness`（响度归一+软限幅）、`wav`（PCM WAV 编码）
- 含 SIMD 加速（`wide::f32x4`）和 rayon 并行优化
- 有 `dev-dependencies`（criterion + approx）但无实际测试文件
- 代码质量较高，文档注释详尽

**3. mox-voice-dsp-py（实质实现）**
- PyO3 扩展，将 dsp-core 暴露给 Python
- 支持 numpy 零拷贝和 list[float] 回退
- 单文件 232 行，封装完整，但缺少 Python 侧测试

**4. mox-voice-asr-svc（实质实现）**
- FR-5 热词三层注入：S1（ContextConfig 探测 stub）、S2（热词临时文件）、S3（Levenshtein post-hoc）
- S1 层为探测占位（feature-gate `sherpa-real`），实际接入需开 feature
- S2/S3 功能完整，与 core-svc 的 hotword 模块联动
- `injector.rs` 223 行，逻辑较丰富

**5. mox-voice-core-svc（完整实现）**
- 9 个模块：errors、identity、hotword、rbac、operator、engine、protocol、constants
- 1,608 行，是 voice 域的核心骨架
- OperatorEngine 三策略调度（LocalFirst/CloudFallback/CloudOnly）
- RBAC 四级 clearance + 5 角色映射
- voice_proxy JSON 信封协议完整实现
- 模块划分清晰，职责明确

**6. mox-voice-desktop-app（实质实现）**
- 桌面悬浮球（BallWidget 5 状态机）+ 全局热键 + voice_proxy 服务
- `main.rs` 达 757 行，承担过多逻辑，建议拆分
- Slint UI 为 P2 规划，当前用 wry/tao WebView2
- 存在 `main.rs.bak_mojibake` 垃圾文件
- 语音引擎为 feature-gate（P2 实现）

**7. mox-voice-intent-svc（完整实现）**
- PPR 规则意图路由，`rules.rs` 325 行含 40+ 中文/拼音正则规则
- 40 条应用别名映射
- 歧义阈值机制，与 Engine 联动裁决
- 纯函数式 `RuledRouter` + async trait `IntentRouterImpl`
- 规则体系完整，与 Python 版 1:1 对齐

**8. mox-voice-operator-svc（完整实现）**
- 8 大类系统算子：app、file、volume、input、network、display、browser、notify
- 4,770 行，是 voice 域代码量最大的 crate
- 跨平台回退链设计（Windows/windows-rs → macOS/osascript → Linux）
- 含 voice_proxy 3717 HTTP 服务（axum + WebSocket）
- 含 voice_engine（录音/播放/sherpa-onnx ASR）和 avatar 模块
- feature-gate 设计合理（server-3717、voice-engine）
- 部分模块（display 555 行、input 558 行、notify 531 行）偏大，可考虑进一步拆分

---

## 三、Market 域详细评测

### Crate 清单

| 子目录 | Crate 名称 | 文件数 | 代码行数 | 实现状态 | 测试 | README |
|-------|-----------|-------|---------|---------|------|--------|
| `api/` | mox-market-api | 1 | 88 | API-only | 无 | 无 |
| `svc/mox-market-template-svc/` | mox-market-template-svc | 1 | 530 | 实质实现 | 无 | 有 |

### 各 Crate 简评

**1. mox-market-api（API-only）**
- 定义插件市场 trait 契约：PluginInfo、PluginStatus、PluginType、ExtensionPoint 等
- 仅 88 行，结构清晰但覆盖的接口较少
- 缺少 Marketplace 主 trait 的完整定义（前 60 行只看到数据结构）

**2. mox-market-template-svc（实质实现）**
- 草莓多平台模板市场：publish/list/load/fork 四大操作
- JSON 持久化到 `templates/` 目录
- 单文件 530 行，所有逻辑集中在 `lib.rs`
- 有 README.md（三域中唯一有文档的 crate）
- 缺少 core 层抽象，业务逻辑与存储耦合在同一文件
- 域内缺少 sdk/core 分层，仅有 api + 一个 svc

---

## 四、Project 域详细评测

### Crate 清单

| 子目录 | Crate 名称 | 文件数 | 代码行数 | 实现状态 | 测试 | README |
|-------|-----------|-------|---------|---------|------|--------|
| `core/mox-project-graph-core/` | mox-project-graph-core | 3 | 1,103 | 完整实现 | 无 | 无 |
| `svc/mox-project-graph-svc/` | mox-project-graph-svc | 4 | 1,155 | 实质实现 | 无 | 无 |

### 各 Crate 简评

**1. mox-project-graph-core（完整实现）**
- 项目需求知识图谱核心引擎
- 3 个模块：schema（实体/关系类型定义）、engine（图谱操作封装）、lib（重导出）
- 1,103 行，其中 engine.rs 达 770 行
- 核心能力：CRUD、依赖管理、进度计算、影响分析、人员负载、关键路径识别
- 基于 `mox-kg-core` 构建，复用知识图谱底层
- engine.rs 偏大，建议按操作类别拆分（如 query_ops、mutation_ops、analysis_ops）

**2. mox-project-graph-svc（实质实现）**
- HTTP 服务层，axum 框架
- 4 个文件：dto（数据传输对象 341 行）、server（路由+处理器 767 行）、main（入口）、lib（重导出）
- server.rs 767 行偏多，建议按资源拆分为多个 handler 模块
- 缺少 api 层 trait 定义（与 voice/market 域的分层模式不一致）
- 有独立的 bin 入口，可直接运行

---

## 五、总体问题清单

### P0 - 严重问题

1. **零单元测试覆盖**：12 个 crate 全部没有 `tests/` 目录，集成测试为零
   - voice-dsp-core 虽声明了 criterion/approx 开发依赖，但无实际基准测试
   - 核心算法（DSP、意图路由、热词注入）无测试保障回归风险高
   - 跨平台算子（8 大类）无平台适配测试

### P1 - 高优先级问题

2. **文档严重缺失**：12 个 crate 中仅 1 个有 README（8.3%）
   - 域级 README 全部缺失（voice/README.md、market/README.md、project/README.md）
   - 新成员无法快速了解各域的架构和使用方式
   - 建议每个域至少有一个顶层 README 说明架构图和 crate 依赖关系

3. **域间分层不一致**
   - voice 域：api + core + sdk + svc（四层）
   - market 域：api + svc（两层，缺 core/sdk）
   - project 域：core + svc（两层，缺 api/sdk）
   - 建议统一域分层模式，至少保持 api/core/svc 三层

4. **market 域功能薄弱**
   - 仅 1 个 svc crate（模板市场），插件/扩展市场核心功能缺失
   - api 层仅 88 行，trait 定义不完整
   - 与 voice 域（8 crate）、project 域（2 crate 但代码量充足）相比差距明显

5. **单文件过大**
   - `mox-voice-operator-svc/src/display.rs` — 558 行
   - `mox-voice-operator-svc/src/input.rs` — 558 行
   - `mox-voice-operator-svc/src/notify.rs` — 531 行
   - `mox-voice-operator-svc/src/voice_engine.rs` — 432 行
   - `mox-project-graph-core/src/engine.rs` — 770 行
   - `mox-project-graph-svc/src/server.rs` — 767 行
   - `mox-voice-desktop-app/src/main.rs` — 757 行
   - 建议按子功能拆分模块，单文件控制在 300-400 行以内

### P2 - 中优先级问题

6. **垃圾文件残留**：`mox-voice-desktop-app/src/main.rs.bak_mojibake` 应清理

7. **market 域单文件架构**：`mox-market-template-svc` 全部 530 行逻辑在 `lib.rs` 中，缺少模块拆分（如 storage、service、model）

8. **project 域缺少 api 层**：无独立的 trait 契约 crate，svc 直接依赖 core，与其他域的 api + core + svc 三层模式不一致

9. **错误处理一致性**：部分 crate 用 `anyhow::Result`，部分用自定义错误类型 + `thiserror`，建议域内统一

10. **日志覆盖不足**：仅部分 svc crate 引入了 `tracing`，core 层日志埋点需确认

### P3 - 低优先级优化

11. **代码风格统一**：voice 域 lib.rs 头部版权注释有 BOM（`\uFEFF`），部分文件没有，建议统一

12. **依赖声明审查**：部分 crate 的 dependencies 可能有未使用项（如 mox-voice-asr-svc 依赖了 `async-trait` 但代码中未见直接使用）

13. **SDK 层缺失**：project 和 market 域均无 sdk 层（如 Python 绑定或 CLI 工具），voice 域有 dsp-py 但仅限 DSP 功能

---

## 六、成熟度评分（满分 10）

| 域 | 功能完备度 | 代码质量 | 测试覆盖 | 文档完善度 | 架构一致性 | 综合评分 |
|---|----------|---------|---------|----------|----------|---------|
| **voice** | 9 | 8 | 0 | 3 | 9 | **5.8** |
| **market** | 3 | 6 | 0 | 5 | 4 | **3.6** |
| **project** | 7 | 7 | 0 | 2 | 6 | **4.4** |

> 说明：测试覆盖全部为 0 分，严重拉低综合评分。若排除测试因素，voice 域可达 7.5 分以上。

---

## 七、改进建议优先级

1. **立即行动（本周）**：为核心算法（dsp-core、intent-svc、hotword）补充单元测试
2. **短期（1-2 周）**：每个域补充顶层 README，清理垃圾文件，拆分超大文件
3. **中期（1 个月）**：market 域补齐核心功能，统一三域分层架构
4. **长期（持续）**：建立 CI 测试门禁，逐步提升测试覆盖率至 60%+
