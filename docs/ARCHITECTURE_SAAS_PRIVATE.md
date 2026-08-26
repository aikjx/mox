# 璇玑 MOX · SaaS / 私有化双模架构总览

> **版本**: v1.0 · 2026-08-26 · 架构文档锚点（最后更新：TTS Rust DSP 全链路打通 + Rust 核心 4 crate / 3 绑定落地）

本文件服务三种读者：**产品交付/部署团队**（如何区分 SaaS 与私有部署）、**后端与算法工程师**（服务端计算核心的 Rust 化目标与边界）、**前端/客户端团队**（客户端只承载 UI、壳与本地缓存）。

---

## 1. 分层总览与服务端 / 客户端边界

系统按 **客户端**、**服务端接入层**、**服务端计算层**、**数据/资产层** 四层组织。SaaS 与私有化部署通过 `platform_config.json` 的 `deployment.mode = "saas" | "private"` 切换，私有化模式关闭 TenantMiddleware、配额计量与云端鉴权插件。

### 1.1 客户端 Client-side（只承载 UI / 壳 / 本地缓存）

| 模块 | 职责 | 关键代码锚点 |
|---|---|---|
| 前端 SPA (Vue3 + Vite `:3021`) | 主 UI、对话、图谱可视化、TTS 三层回退、业务面板 | [MessageBubble.vue](file:///d:/a10/aikjx/gitcode/infotopograph/frontend-ui/src/components/MessageBubble.vue#L836-L937)（TTS 三层回退 + 豆包级拟人音参数） · [ChatView.vue](file:///d:/a10/aikjx/gitcode/infotopograph/frontend-ui/src/views/ChatView.vue) · [vite.config.js](file:///d:/a10/aikjx/gitcode/infotopograph/frontend-ui/vite.config.js)（Vite `/voice` 代理 → `:3001`） |
| 桌面壳 (xiaobai-desktop) | 桌面端打包、离线文件关联、单实例启动 | [xiaobai-desktop/lib.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-desktop/src/lib.rs) |
| App / H5 / 小程序容器 | 移动端入口、弱网下缓存队列、扫码登录 | `platform/sdk/{nodejs,python,rust}/`（`mox-sdk-cloud`、`mox-sdk-graph`） |

> **边界规则（强制）**：客户端不得做任何图算法、归一化求解、意图分类 / 联盟打分、语音 DSP 的 CPU 密集计算；只做 RPC / WebSocket 调用、解码播放、UI 状态机。

### 1.2 服务端 Server-side —— 接入层（入口 / 路由 / 治理）

| 模块 | 职责 | 关键代码锚点 |
|---|---|---|
| Rust 网关 `operator-server :3001` | HTTP 接入、voice_proxy、RBAC、AI 路由、市场侧车 | [voice_proxy.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/routes/voice_proxy.rs#L90-L103)（**TTS/ASR 路由级 600s 超时**，健康 60s，其他 30s） · [rbac_middleware.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/rbac_middleware.rs) · [ai_router.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/ai_router.rs) |
| SaaS 多租户中间件（仅 SaaS 启用） | 租户上下文注入、配额计量、计费事件、审计流水 | [xiaobai-core/identity.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-core/src/identity.rs) · [xiaobai-core/rbac.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-core/src/rbac.rs) |
| 私有化一键入口（仅 private 启用） | systemd / Windows Service 注册、单机自更新、离线授权 | `deploy/docs/ops-manual.md` · [platform_config.json](file:///d:/a10/aikjx/gitcode/infotopograph/platform_config.json)（`xiaobai_voice` 服务注册 · `auto_start=true`） |

### 1.3 服务端 Server-side —— 计算 / 业务层（性能核心，全部 Rust 化目标）

> 本层即 MOX 规格 `.trae/specs/20260825-mox-all-core-rust-max-algo/spec.md` 定义的「核心 4 + 绑定 3」。验收标准见 [tasks.md](file:///d:/a10/aikjx/gitcode/infotopograph/.trae/specs/20260825-mox-all-core-rust-max-algo/tasks.md) 的 17 项任务切片。

#### 核心 Rust crates（`platform/crates/*`）

| Crate | 功能范围 | 单元测试基线 | 代码锚点 |
|---|---|---|---|
| `mox-formulas-core` | 12 项图权威公式（CSR / PageRank Gauss-Seidel / Brandes 介数 / CNM 社区 / 同配系数 / k-core / 度数中心性等） | 19 项单测 | [csr.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-formulas-core/src/csr.rs) · [pagerank.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-formulas-core/src/pagerank.rs) · [centrality.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-formulas-core/src/centrality.rs) · [community.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-formulas-core/src/community.rs) |
| `mox-norm-core` | Ahash 去重 · 规则引擎（Rete 简化版）· 冲突融合 · 增量字段合并 | 5 项单测 | [dedup.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-norm-core/src/dedup.rs) · [rules.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-norm-core/src/rules.rs) · [merge.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-norm-core/src/merge.rs) |
| `mox-intent-core` | Aho-Corasick 多模式匹配 · 等级评分（等级+长度+全词加分）· SIMD 联盟打分（wide f32x8） | 7 项单测 | [classifier.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-intent-core/src/classifier.rs) · [alliance.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/mox-intent-core/src/alliance.rs) |
| `xiaobai-dsp` | 线性插值重采样 · SOLA 同步叠加变速 · BS.1770-4 响度归一 + 软限幅 · WAV 头编解码 | 10 项单测 · **10.59×** Python 性能 | [resample.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-dsp/src/resample.rs) · [sola.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-dsp/src/sola.rs) · [loudness.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-dsp/src/loudness.rs) · [wav.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/xiaobai-dsp/src/wav.rs) |

#### 跨语言绑定 crates（`platform/crates/bindings/*`）

| Crate | 绑定技术 | 宿主 | 代码锚点 |
|---|---|---|---|
| `mox-formulas-native` | napi-rs | Node.js `backend-node` | [mox-formulas-native/src/lib.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/bindings/mox-formulas-native/src/lib.rs) |
| `mox-norm-intent-native` | napi-rs | Node.js `backend-node` | [mox-norm-intent-native/src/lib.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/bindings/mox-norm-intent-native/src/lib.rs) |
| `xiaobai-dsp-py` | PyO3 0.22 | Python 3.12 `xiaobai_voice` | [xiaobai-dsp-py/src/lib.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/crates/bindings/xiaobai-dsp-py/src/lib.rs) |

> 绑定调用链：`Node/Python → native binding → Rust core crate`。Rust 核心 4 不直接依赖 Node/Python 运行时，可被 `mox-server` 等 Rust 服务直接静态链接。

### 1.4 数据 / 资产层

| 层 | 资产 | 说明 |
|---|---|---|
| LLM 权重 | CosyVoice2-0.5B 目录（iic/CosyVoice2-0.5B） | `projects/xiaobai_voice/models/tts-cosyvoice2-0.5b/`（3.6GB，Apache-2.0） |
| 规则 / 模型 | 默认配置 + 用户 patch | `projects/xiaobai_voice/xiaobai_voice/config/default_config.yaml`（`voice.tts.speed=1.03`） |
| 存储 | S3 / 本地磁盘 / SQLite / Postgres | SaaS 推荐云端对象存储 + Postgres；私有化本地磁盘 + SQLite 可行 |
| 知识资产 | 云盘知识库 / 图谱节点 · 边 / 算子定义 | `projects/mox-cloud-drive-*` · `platform/services/kg-hub` · `platform/services/mox-graph-storage` |

---

## 2. 业务处理流程 —— 以 AI 对话 + TTS 朗读为主轴

### 2.1 主流程（Critical Path）

```
用户文字输入
  ↓ (前端 :3021)
ChatView → submit
  ↓ POST /api/chat
Rust 网关 :3001 → AI Router
  ↓ (LLM / 专家联盟)
mox-expert / flow-ai / mox-ai-core
  ↓ 生成 AI 回复（流式 token / SSE）
前端 MessageBubble 渲染消息 + 触发「朗读」按钮
  ↓ 用户点击或自动播放
handleSpeakThreeLayer()
  ├─► L1: GET /voice/tts/stream?(Vite proxy → :3001 voice_proxy → :3717)
  │      响应 audio/wav; 22050Hz; headers: X-TTS-Engine=cosyvoice2, X-TTS-DSP-Impl=Rust
  │      → decodeAudioData → Audio.play()
  ├─► L2 (L1 失败): Web Speech Synthesis PREMIUM_ZH 白名单精选女声
  └─► L3 (L2 失败): 剪贴板复制兜底
```

### 2.2 TTS 子系统内部链路（xiaobai_voice :3717）

| 阶段 | 代码锚点 | 关键优化 |
|---|---|---|
| 路由层（FastAPI，GET/POST 兼容） | [main.py:_tts_stream_impl](file:///d:/a10/aikjx/gitcode/infotopograph/projects/xiaobai_voice/xiaobai_voice/service/main.py#L454-L532) | synthesize_full 预合成 → 设 `X-TTS-Engine` / `X-TTS-DSP-Impl` / `Content-Length` |
| 引擎调度（TTS __init__ + default_engine 兼容） | [tts/__init__.py](file:///d:/a10/aikjx/gitcode/infotopograph/projects/xiaobai_voice/xiaobai_voice/tts/__init__.py#L27-L50) | 支持 `cosyvoice / cosyvoice2 / fish_s2_pro / browser` 别名 |
| CosyVoice2 推理（zero-shot / instruct2 / sft） | [cosyvoice2.py](file:///d:/a10/aikjx/gitcode/infotopograph/projects/xiaobai_voice/xiaobai_voice/tts/cosyvoice2.py) · `_bootstrap_zero_shot_default` · `_do_infer_raw` | 零样本音色注册（3s 正弦提示音）· 缺失 `embedding` 字段兜底降级 zero-shot · `load_wav` monkey-patch（soundfile → 跳过 torchaudio） |
| Rust DSP 优先流水线 | 同上 `apply_dsp_pipeline` → `xiaobai_dsp_native.apply_dsp_pipeline` | 线性重采样（替代 nearest）· SOLA 1.03× 变速 · EBU R128 响度归一 · 软限幅；失败回退 Python 实现 |

### 2.3 接入层 voice_proxy 超时矩阵（网关级保护）

| 路由分类 | 超时 | 场景 |
|---|---|---|
| `/voice/tts/*`, `/voice/asr/*`, `/voice/models/download` | **600s** | CosyVoice2 首合成 ~210s，预留 3× 余量 |
| `/voice/health`, `/voice/models*`, `/voice/hotwords`, `/voice/license_tier`, `/voice/metrics` | 60s | 健康检查与元数据 |
| 其他 | 30s | 列表、鉴权、配置 |

代码锚点：[voice_proxy.rs:94-103](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/routes/voice_proxy.rs#L94-L103)

---

## 3. SaaS / 私有化双模切换

### 3.1 切换开关

```jsonc
// platform_config.json 顶层
{
  "deployment": {
    "mode": "saas",        // "saas" | "private"
    "tenant_isolation": true,   // private 模式置 false
    "cloud_license": true       // private 模式置 false，启用本地 license 文件
  }
}
```

### 3.2 双模矩阵

| 能力 / 模块 | SaaS 模式 (mode="saas") | 私有化模式 (mode="private") |
|---|---|---|
| 租户隔离（TenantMiddleware） | 强制开启 | 关闭 |
| 配额 / 计量 / 计费事件 | 开启 + 上报 | 本地日志（可选） |
| 云端对象存储（S3/OSS） | 推荐 | 本地磁盘 (`~/.mox/`，[FS-S3 切换 SOP](file:///d:/a10/aikjx/gitcode/infotopograph/deploy/docs/storage-cloud-switch-sop.md)) |
| 单点登录 / 扫码登录 | OIDC / 钉钉 / 企业微信 | 本地账号 + 离线 license 签名 |
| 服务进程模型 | 多副本 + K8s HPA（[Helm chart](file:///d:/a10/aikjx/gitcode/infotopograph/deploy/helm/mox/values.yaml)） | 单机单实例 · systemd / Windows Service |
| 语音服务 | xiaobai_voice 独立 Deployment（auto_start=true，port=3717） | 同上，单机监听 `127.0.0.1:3717` |
| Rust 计算核心 | 4 crate + 3 绑定 全量启用 | 同上 |
| 桌面离线同步 | 客户端本地缓存队列 + 增量同步 | 全量离线模式，服务在本机 |

### 3.3 交付模式（SaaS 与私有部署共享同一代码库）

- **唯一来源 (Single Source of Truth)**: 顶层 `Cargo.toml` workspace + `platform_config.json` + `deploy/helm` 一套。
- **私有化一键包**: `scripts/build_private_installer.*` 产出 `MoxSetup.exe` / `.deb`，内嵌 release 版 operator-server、xiaobai_voice（含 CosyVoice2 权重 tar）、license.lic 校验。
- **SaaS 发布**: 通过 `.github/workflows/enterprise-ci.yml` → 镜像 `registry.cn-xxx/mox/runtime:$COMMIT` → Helm upgrade。

---

## 4. 需求的业务处理流程 —— 以「全维融合 / 图治理 / 专家联盟」为例

| 需求阶段 | 服务端模块 | 客户端模块 | 关键产物 |
|---|---|---|---|
| 需求输入 | `mox-expert/server.rs` / `market.rs`（需求编译 DSL） | ChatView / 需求模板页 | 需求规格 `.md` + `graphql seed` |
| 图谱生成 | `mox-formulas-core`（结构度量 + 社区）· `kg-hub`（摄入 + 推理） | GraphView / Guantu 治理台 | 关图骨架、节点-边 Schema |
| 归一与判重 | `mox-norm-core`（Ahash 去重 + 规则求解 + 冲突融合） | 治理台 Diff 视图 | 归一报告、冲突清单 |
| 意图识别 / 路由 | `mox-intent-core`（Aho-Corasick）· AI Router | 用户任务面板 | 意图分类 + 联盟候选 |
| 联盟打分 / 派单 | `mox-intent-core/alliance.rs`（SIMD 加权）· 市场侧车 | 专家中心、任务详情页 | 联盟分 Top-N、任务单 |
| 交付 / 验证 | `mox-expert/verify/`（CEM + Harness）· 审计 S3 | 合规页、验收报告 | 验收归档 `.md` + S3 清单 |

> 所有计算密集阶段（图算法 / 归一 / 意图 / 联盟 / DSP）**必须走 Rust 核心 4 crate 或其绑定**；禁止在 Node.js / Python 胶水层单线程跑权威公式。

---

## 5. 验收标准（Rubric 精简版）

| 维度 | Pass 条件 | 证据 |
|---|---|---|
| TTS 主路 | 点击「朗读」→ 不降级到浏览器；响应头 `X-TTS-Engine=cosyvoice2, X-TTS-DSP-Impl=Rust`；WAV 22050Hz | `scripts/verify_tts_rust_fullstack.py` 全部通过（E-1 ~ E-6） |
| TTS 音质 | 10 人主观盲测 ≥ 豆包 7/10；客观 PESQ / MOS 达标 | 验收报告中 P.863 PESQ 段 |
| 图公式 Rust | 12 项公式 Node.js ↔ Rust 输出相对误差 ≤ 1e-6 | `mox-formulas-native` 绑定对拍脚本 |
| 归一化 5× 吞吐 | 10 万实体吞吐 ≥ 5× JS 基线；内存 ≤ 1/10 | R-4 基准报告 |
| 意图分类 QPS | Aho-Corasick 多模式匹配，正则链退化消除 | 压力报告 |
| Rust DSP 10× | `xiaobai-dsp` 端到端延迟 ≤ Python 1/10 | E-2b 中 `_last_dsp_impl == Rust` + 耗时对拍 |
| 双模切换 | `mode=saas|private` 切换后健康检查通过，无多租户中间件泄漏 | 运维脚本 + 冒烟 |

---

## 6. 相关文档索引

| 文档 | 位置 |
|---|---|
| MOX 核心 Rust 化规格 | [spec.md](file:///d:/a10/aikjx/gitcode/infotopograph/.trae/specs/20260825-mox-all-core-rust-max-algo/spec.md) · [tasks.md](file:///d:/a10/aikjx/gitcode/infotopograph/.trae/specs/20260825-mox-all-core-rust-max-algo/tasks.md) |
| TTS 全链路验收脚本 | [verify_tts_rust_fullstack.py](file:///d:/a10/aikjx/gitcode/infotopograph/scripts/verify_tts_rust_fullstack.py) |
| 运维与部署 | `deploy/docs/ops-manual.md` · `deploy/docs/storage-cloud-switch-sop.md` |
| 企业级交付归档 | [enterprise 目录](file:///d:/a10/aikjx/gitcode/infotopograph/docs/enterprise/00-INDEX.md) |
| 根 Cargo workspace | [Cargo.toml](file:///d:/a10/aikjx/gitcode/infotopograph/Cargo.toml) |
