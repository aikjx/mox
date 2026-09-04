# 小白语音服务 + 璇玑 MOX 架构mox 模块化系统架构落地 · 任务切片
## AC 概述：rule(15) x FR + rule(15) x NFR + rubric(8) = Grade S（加权>=90）

## 四阶段里程碑
| 阶段 | 时间窗 | 交付 | Task | 退出条件 |
|------|--------|------|------|----------|
| M1 基础完善 | 09-10月 | 骨架+P1-P4根治+ASR/TTS+死锁 | 1-7+12核心 | 缺陷TR100%；CER<=5%；死锁1000轮零 |
| M2 能力增强 | 11-12月 | 七层+PPR+鉴权+专家+三策略 | 8-11+16+12收尾 | 七层零断点；8闸门全绿 |
| M3 SaaS发布 | 01-02月 | 多租户+会员+OTA+可观测 | 13-15 | 云平台上线；金丝雀4阶段 |
| M4 规模化信创 | 03-04月 | E2E>=829+GradeS+信创 | 17-18 | 全绿；Rubric>=90；信创4/4 |

## 依赖图
1. 骨架 Task1
2. 并行组A：Task2 ASR + Task3 TTS + Task12死锁子项
3. 并行组B：Task4 sensitivity + Task5 Reconcile + Task6 Suggestion*Constraint + Task7 constants
4. 并行组C：Task8七层 + Task9 PPR + Task10鉴权 + Task11专家联盟
5. Task12 客户端+PyInstaller
6. Task13 云平台多租户+会员
7. Task14 OTA+CDN+差分
8. Task15 P99仪表+取证
9. Task16 三策略
10. Task17 E2E>=829+Harness700
11. Task18 Rubric+Grade S

---

## Task 1: Workspace 骨架 + 指标注册中心 + 七层类型共享

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-4~FR-11 前置（类型定义共享）；NFR-5 指标注册；M1 里程碑基座 |
| Dependencies | - |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- `crates/mox-expert/Cargo.toml`：新增 `sensitivity`、`constants` 模块声明；features = ["default", "wasm", "enterprise"]
- `crates/mox-expert/src/lib.rs`：pub mod sensitivity; pub mod constants; re-export Dimension / Constraint / ExpertOpinion
- `projects/xiaobai_voice/pyproject.toml`：dependencies + extras [asr,tts,desktop,dev,cloud]
- `projects/xiaobai_voice/xiaobai_voice/__init__.py`：版本号 + 子包导入桩
- `projects/xiaobai_voice/xiaobai_voice/config/default_config.yaml`：asr/tts/desktop/service 默认配置
- `projects/xiaobai_voice/xiaobai_voice/config/models.yaml`：4 条模型（Paraformer/Sherpa/Fish/CosyVoice）+ SHA256
- `crates/mox-observability/src/registry_ext.rs`：注册 9 项新指标（asr_cer, tts_mos, ppr_accuracy, mox_gate_latency, session_count, p99_play_latency, ota_success_rate, tenant_quota_usage, deadlock_detector_count）
- `platform/services/mox-saas-tenant/Cargo.toml` + stub lib.rs：多租户骨架
- `platform/services/mox-ota-differential/Cargo.toml` + stub lib.rs：OTA 骨架
- 工作区根 `Cargo.toml`：members 追加 5 新 crate（mox-sensitivity-unused / mox-constants-policy / mox-saas-tenant / mox-ota-differential / mox-ppr-router）

### Task-local Test Requirements (TR >= 8)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T1-TR1 | rule | 9 个新 crate `cargo check` 零 error；Python 项目 `pip install -e .` 成功 | 9/9 + pip exit 0 | cargo-check.log + pip.log |
| T1-TR2 | rule | Metric Registry 注册后 `prometheus.gather()` 包含 `asr_cer_bucket` 等 9 指标名全匹配 | metric name count = 9 | registry_unit_test.rs |
| T1-TR3 | rule | Python `python -m xiaobai_voice selftest` 打印 MISSING_MODEL 提示 + 返回码 0（无模型不崩溃） | stderr contains "MISSING_MODEL" AND exit=0 | selftest.log |
| T1-TR4 | rule | 配置文件 YAML 语法合法；`models.yaml` 4 条模型含 id/url/sha256/size_bytes 4 字段齐全 | 4/4 models with 4/4 fields each | config_loader_test.py |
| T1-TR5 | rule | Rust `Dimension` 枚举 7 变体（Permission/Security/Resource/Data/Business/Observability/Algorithm）全导入可构造 | 7/7 variants constructible | ir_derive_test.rs |
| T1-TR6 | rule | `Constraint` 8 变体（MustGuard/MustSerialize/MustIsolate/MustOrder/MustAudit/ResourceCap/VetoGuard/CapabilityGuard）serde roundtrip 一致 | 8/8 roundtrip = | constraint_serde_test.rs |
| T1-TR7 | rubric | 目录布局工程质量：语音/璇玑/云平台三大板块解耦；跨板块仅通过 traits/FFI 通信 | review score >= 90 | code-review-t1.md |
| T1-TR8 | rule | 工作区 `cargo test --lib` 既有 649 UT 零回归（本任务不改逻辑，仅验证骨架无破坏） | pass 649/649 fail 0 | workspace-test-summary.log |

### Completion Evidence
_（待实施时填写：cargo check 日志、pip install 日志、9 指标 prometheus 快照、649 UT 汇总报告）_

---

## Task 2: ASR 三层回退（Paraformer→Sherpa→SenseVoice）+ VAD + 热词 + INT8

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-1；NFR-2 流式延迟；NFR-9 热词注入；NFR-11 VAD 断句；M1 里程碑语音层 |
| Dependencies | Task 1 (xiaobai_voice skeleton) |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- `projects/xiaobai_voice/xiaobai_voice/asr/base.py`：ABC `ASRBackend`（recognize_stream / recognize_full / set_hotwords / prewarm / close / health）
- `projects/xiaobai_voice/xiaobai_voice/asr/paraformer.py`：Paraformer-zh int8 加载；流式 partial 回调；热词注入 `hotwords.txt`
- `projects/xiaobai_voice/xiaobai_voice/asr/sherpa_onnx.py`：sherpa-onnx paraformer + silero-vad threshold=0.5；min_silence_ms=800 可配
- `projects/xiaobai_voice/xiaobai_voice/asr/sensevoice_fallback.py`：SenseVoice 多语种第三层；ImportError 友好降级
- `projects/xiaobai_voice/xiaobai_voice/asr/__init__.py`：`build_asr_backend(config) -> FallbackChain`；三层 try/except 自动回退
- `projects/xiaobai_voice/xiaobai_voice/asr/vad.py`：SileroVADWrapper；端点检测 `on_speech_start` / `on_silence_end` 回调
- `projects/xiaobai_voice/xiaobai_voice/tests/test_asr.py`：12+ 单测；fixture 合成 16k PCM WAV
- `projects/xiaobai_voice/xiaobai_voice/config/hotwords.json`：默认热词「璇玑」「小白」「玄铁」「RISC-V」

### Task-local Test Requirements (TR >= 12)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T2-TR1 | rule | 短句 CER 测试：合成 10 条中文短句（每句 4-8 字）WAV 16k；Paraformer 识别 CER <= 5% | 10/10 CER avg <= 5% | asr_cer_short_report.json |
| T2-TR2 | rule | 流式延迟：WebSocket 上送 1 s PCM；首 partial 结果 latency P99 <= 300 ms | P99 <= 300 ms（20 次平均） | asr_stream_latency.csv |
| T2-TR3 | rule | INT8 加载：sherpa-onnx-paraformer-zh-int8.onnx 加载耗时 <= 1200 ms；峰值内存 <= 250 MB | time <= 1200 ms AND mem <= 250 MB | int8_load_bench.log |
| T2-TR4 | rule | VAD 断句：5 段音频（含 2 处 1 s 静音间隙）→ 正确切分 7 句；切分准确率 >= 95% | 7/7 boundaries correct | vad_segment_test.log |
| T2-TR5 | rule | 回退机制 100%：模拟 Paraformer DLL 失败 → Sherpa 接管；模拟 Sherpa OOM → SenseVoice 接管；三层全崩返回 FallbackError 不崩溃 | 3 场景 exit 正常 / raise FallbackError | asr_fallback_3scenarios.log |
| T2-TR6 | rule | 热词注入：5 条含「璇玑」短句，未加热词识别率 70% → 加热词后 >= 75%（提升 >= 5%） | delta >= +5% | hotword_improvement.json |
| T2-TR7 | rule | 热键冲突检测：Alt+X 录音与系统 Alt+X（如 Office 剪贴板）冲突时 toast 提示「热键冲突请在设置修改」 | toast shown | hotkey_conflict_toast.png |
| T2-TR8 | rule | DLL 失败提示：DLL_LOAD_FAIL 分类返回 `AsrError::DllLoad(dll_name)`；UI 显示「xxx.dll 缺失，请重装语音包」而非 500 | message match 100% | dll_fail_ui.html |
| T2-TR9 | rule | prewarm()：启动预热 30 ms 短句；预热后首句延迟降低 >= 40%（对比冷启动首句） | delta_latency <= -40% | prewarm_benchmark.json |
| T2-TR10 | rule | 长句识别（>= 30 字）：5 条长句平均 CER <= 6%；无超时（timeout=30 s） | 5/5 success AND avg CER <= 6% | asr_long_cer.json |
| T2-TR11 | rule | 热词持久化：POST /voice/hotwords {"璇玑": 20} → 重启服务后热词仍生效；权重 20 生效 | after-restart hotword 生效 | hotwords_persist_roundtrip.log |
| T2-TR12 | rule | MissingModel 分级：缺 Paraformer → Warning 走 Sherpa；缺前两层 → Error 提示下载中心；日志分级对应 warning/error | log level match 2/2 | missing_model_log_levels.log |

### Completion Evidence
_（待实施时填写：CER 报告、流式延迟 CSV、INT8 内存截图、三层回退全链路日志、热词提升对比数据、死锁零复发断言）_

---

## Task 3: TTS 三层回退（Fish→CosyVoice→Browser）+ 零样本克隆 + 情绪 + SOLA

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-2；NFR-2 TTS 首字节；NFR-10 零样本克隆；NFR-12 SOLA 不变调；NFR-13 响度；M1 里程碑语音层 |
| Dependencies | Task 1 (xiaobai_voice skeleton) |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- `projects/xiaobai_voice/xiaobai_voice/tts/base.py`：ABC `TTSBackend`（synthesize_stream / synthesize_full / get_speakers / set_emotion / clone_voice / close）
- `projects/xiaobai_voice/xiaobai_voice/tts/fish_s2_pro.py`：delayed import（仅 tier=research 才 __import__）；零样本克隆 reference_audio；情绪 5 标签
- `projects/xiaobai_voice/xiaobai_voice/tts/cosyvoice2.py`：Apache2 默认；chunk=250 ms 流式；INT8/FP16 自动探测；speaker_id 探测
- `projects/xiaobai_voice/xiaobai_voice/tts/browser_fallback.py`：空后端；返回空流 + 响应头 `X-TTS-Fallback: browser`
- `projects/xiaobai_voice/xiaobai_voice/tts/__init__.py`：`build_tts_backend(config, tier)`；三层回退链 + license_tier 路由
- `projects/xiaobai_voice/xiaobai_voice/tts/audio_proc.py`：SOLA 算法（语速 0.8×~1.4× 不变调）；EBU R128 响度 -18 dBFS；软限幅 True Peak <= -1 dBTP
- `projects/xiaobai_voice/xiaobai_voice/tts/playback.py`：`_PlaySession` 会话化对象（死锁修复核心）；play() / stop() / pause() 状态机
- `projects/xiaobai_voice/xiaobai_voice/tests/test_tts.py`：14+ 单测；含 MOS 代理（STOI 客观指标替代）

### Task-local Test Requirements (TR >= 14)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T3-TR1 | rule | MOS >= 4.0：20 条句主观评测（或 STOI >= 0.92 客观替代）；Fish/CosyVoice 各 10 条全过 | STOI avg >= 0.92 | tts_mos_objective.json |
| T3-TR2 | rule | 首 token CPU <= 3.5 s：CPU 机器 synthesize_stream 首字节 latency P99 <= 3500 ms（20 次） | P99 <= 3500 ms | tts_first_token_cpu.csv |
| T3-TR3 | rule | 零样本克隆：上传 5 s 参考 WAV → speaker 持久化；合成 5 句相似度 MOS >= 3.8（或 cosine sim >= 0.82） | sim >= 0.82 | zero_shot_clone_report.json |
| T3-TR4 | rule | 情绪 5 标签：neutral / happy / sad / serious / angry；CosyVoice 全部可设；unknown 情绪 → neutral（不报错） | 5/5 set ok + 1 fallback ok | emotion_5tags_roundtrip.log |
| T3-TR5 | rule | 流式边生成边播放：MediaSource + SourceBuffer；updateend 事件 2 s 内首触发；播放不卡顿（缓冲 < 500 ms） |首 trigger <= 2 s AND buffering < 500 ms 90% | stream_playback_metrics.csv |
| T3-TR6 | rule | 三层回退 100%：Fish license=apache2 禁用 → CosyVoice；CosyVoice OOM → Browser；UI toast 提示「已降级浏览器 TTS」 | 3 场景 toast match 3/3 | tts_fallback_3tiers.log |
| T3-TR7 | rule | SOLA 语速不变调：1.0× / 0.8× / 1.2× / 1.4× 四档；基频 F0 偏差 <= ±5%；MOS 不降级 >= 3.9 | delta F0 <= ±5% 4/4 | sola_pitch_bias.csv |
| T3-TR8 | rule | 响度 -18 dBFS：20 条合成音频集成响度 LUFS 落在 [-19, -17] 区间（容差 ±1）；True Peak <= -1 dBTP 100% | 20/20 within range AND 20/20 TP ok | loudness_r128_report.json |
| T3-TR9 | rule | 软限幅：+6 dBFS 正弦峰值输入 → 输出无削波（THD+N <= 1%）；无数字 over（sample 无 0x7FFF/0x8000） | THD+N <= 1% AND zero clip | soft_limiter_thd.log |
| T3-TR10 | rule | speaker_id 探测：CosyVoice 模型加载时枚举 20+ speaker_id；下拉列表 UI 正确显示；选 speaker=xiaobai_default 合成无报错 | 20+ enumerated AND 1/1 play ok | speaker_id_detect.log |
| T3-TR11 | rule | license_tier=apache2 AST 扫描：grep -r "fish_speech" `xiaobai_voice/` 计数 = 0；import fish_speech raise ImportError 被 fallback 捕获 | count = 0 AND catch ok | apache2_compliance_ast.log |
| T3-TR12 | rule | 克隆音色持久化：clone_voice 存 voice_clips/<sha1>.wav；重启服务后 sha1 查询仍可合成；路径跨平台兼容 | after-restart synthesize ok 3/3 | clone_persist_roundtrip.log |
| T3-TR13 | rule | 长文本 1000 字流式：分片 chunk 无缝；总 duration 误差 <= ±5%；无内存泄漏（前后 RSS delta <= 50 MB） | delta dur <= ±5% AND leak <= 50 MB | longtext_stream_1000.log |
| T3-TR14 | rule | 音频播放器优雅关闭：play 中途 stop() → audio device release <= 200 ms；1000 轮循环无死锁（Task 12 死锁修复交叉验证） | 1000/1000 no deadlock AND release <= 200 ms | playback_stress_1000.log |

### Completion Evidence
_（待实施时填写：STOI/MOS 报告、首 token 延迟 CSV、克隆相似度数据、R128 响度报告、1000 轮死锁零复发断言、Apache2 AST 合规扫描）_

---

## Task 4: sensitivity.rs 归一（P1 缺陷 · PII 判据三处分叉 → 唯一权威）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-4（P1 根治）；NFR-6 安全合规；M1 里程碑缺陷修复；OUT-8 冲突溯源 |
| Dependencies | Task 1（mox-expert crate 骨架） |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- `crates/mox-expert/src/sensitivity.rs`：单一权威 SSOT 模块，导出：
  - `pub const SENSITIVE_PREFIXES: &[&str]`：`["db:citizen_", "pii:", "id_card:", "phone:", "bank_card:"]`（统一带下划线前缀，根治 `contains("citizen")` 误杀 `var:citizen_safe`）
  - `pub fn is_sensitive(resource: &str) -> bool`：starts_with 精确前缀匹配
  - `pub fn is_production(resource: &str) -> bool`：生产前缀 `["db:prod/", "db:prod_", "env:prod:"]`
  - `pub fn is_desensitized(resource: &str) -> bool`：后缀 `_safe` / `_desensitized` / `_masked` / `_anon` → 返回 true
  - `pub fn is_sensitive_leak(resource: &str) -> bool`：`is_sensitive(r) && !is_desensitized(r)`
  - `pub fn classify_prefix(resource: &str) -> SensitivityCategory`：NotSensitive / SensitivePii / SensitiveId / SensitivePhone / SensitiveBank / ProductionWritesensitive / AlreadyDesensitized
- 修改 `crates/mox-expert/src/permission.rs:22` + `:53`：删除重复 `sensitive_prefixes`/`sensitive_prefixes_w` 两处数组，改为调用 `sensitivity::is_sensitive_leak` / `sensitivity::is_production`
- 修改 `crates/mox-expert/src/security.rs:52`：删除 `contains("pii") || contains("citizen")` 粗暴匹配，改为调用 `sensitivity::is_sensitive_leak(resource)`
- `crates/mox-expert/src/lib.rs`：`pub mod sensitivity;` + re-export `SensitivityCategory`
- `crates/mox-expert/src/tests/sensitivity_tests.rs`：10+ 场景覆盖单测

### Task-local Test Requirements (TR >= 10)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T4-TR1 | rule | 同资源三位置 100% 一致：20 条资源（含 10 敏感 / 5 脱敏 / 5 非敏感）→ permission.rs / security.rs / sensitivity.rs 三处调用结果 bool 全等 | 20/20 three-way equal | three_way_consistency_20cases.log |
| T4-TR2 | rule | 已脱敏数据不阻断假阳性：`var:citizen_safe` / `pii:phone_masked` / `db:citizen_anon` 共 5 条 → permission AND security 均 NOT Blocking | 5/5 not blocked AND not vetoed | desensitized_not_blocked.log |
| T4-TR3 | rule | 20 场景前缀分类正确：按 classify_prefix 20 条 → 枚举 variant 与期望（SensitivePii×4/SensitiveId×3/SensitivePhone×3/SensitiveBank×3/ProductionWrite×3/AlreadyDesensitized×2/NotSensitive×2）完全匹配 | 20/20 variant = expected | classify_prefix_20cases.log |
| T4-TR4 | rule | 未命中 → NotSensitive 正确：`db:sales/order` / `var:user_name` / `file:report_2026.pdf` 等 8 条非敏感 → `is_sensitive_leak`=false AND `classify`=NotSensitive | 8/8 false + NotSensitive | notsensitive_8cases.log |
| T4-TR5 | rule | 脱敏标记流转不重复判敏：图节点带 tag `desensitized=true` → `govern()` 二次判敏短路；审计链无重复 "SensitiveCheck" record 2+ | repeat_count = 0 / 10 flows | desensitized_tag_shortcircuit.log |
| T4-TR6 | rule | P1 经典复现：`var:citizen_safe`（旧 security.rs `contains("citizen")` 会误判）→ 新代码三处调用全返回 NOT sensitive | 3/3 false AND regression test pass | p1_regression_var_citizen_safe.log |
| T4-TR7 | rule | 生产写 veto：`db:prod/citizen/info` 写操作 → `permission.expert` `push_veto`；`algo.vetoed=true`（不可自动修复） | vetoed=true 3/3 scenarios | prod_write_veto.log |
| T4-TR8 | rule | regulated 租户 PII 外发：tenant.regulated=true + `is_sensitive_leak(r)` AND node.out == http → security.expert Blocking + remediation "插入脱敏 Guard 节点" | Blocking + remediation filled 5/5 | regulated_pii_http_block.log |
| T4-TR9 | rule | 反向 5 条真实敏感未漏判：`db:citizen_profile` / `pii:id_card_no` / `phone:138xxxx` / `bank_card:6222xxxx` / `db:prod/pii/user` → 全 5 `is_sensitive_leak=true` | 5/5 true | real_sensitive_not_missed.log |
| T4-TR10 | rule | 模块零外部依赖（仅 std）：sensitivity.rs cargo tree 无第三方；wasm32 target 编译通过（offline policy 可嵌入 WASM） | no deps AND wasm32 compile ok | sensitivity_cargo_tree_wasm.log |

### Completion Evidence
_（待实施时填写：三位置一致性 20 条报告、P1 假阳性复现→修复回归日志、20 分类枚举快照、生产写 veto 审计链段）_

---

## Task 5: Reconcile 冲突裁决升级（P2 缺陷 · conflicts 永久空 Vec → 同优先级冲突升级 Blocking）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-5（P2 根治）；OUT-8 所有阻断必须 conflicts 溯源；M1 里程碑缺陷修复 |
| Dependencies | Task 1（mox-expert 骨架）+ Task 4（sensitivity.rs，冲突判据依赖维度） |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- 修改 `crates/mox-expert/src/reconcile.rs:43`：`let conflicts: Vec<ReconcileConflict> = Vec::new();` → `let mut conflicts: Vec<ReconcileConflict> = Vec::new();` 可变
- `crates/mox-expert/src/reconcile.rs` 新增：
  - `struct ConstraintKind` 枚举：`GuardKind / SerialKind / IsolateKind / OrderKind / AuditKind / ResourceKind / VetoKind / CapabilityKind`
  - `impl Constraint { pub fn kind(&self) -> ConstraintKind; pub fn nodes(&self) -> &[NodeId]; }`
  - `struct ReconcileConflict { same_priority: bool, dimension_pair: (Dimension, Dimension), kind_pair: (ConstraintKind, ConstraintKind), node_ids: Vec<NodeId>, escalated: bool, reason_code: ReasonCode, }`
  - `enum ReasonCode`：`SAME_PRIORITY_SAME_KIND_OVERLAP / SEMANTIC_OPPOSITE_SERIAL_PARALLEL / ...`（可解释原因码）
  - 冲突检测循环：按节点归并 Constraint → 比较每对 → 同优先级 + 同 Kind + 节点交集非空 → push ReconcileConflict{escalated:true}
  - 返回 `ReconciledPlan { plan, conflicts, escalated_blocking: bool }`；`escalated_blocking=true` 时 pipeline 并入 `algo.vetoed`
- 修改 `crates/mox-expert/src/pipeline.rs`：`if plan.conflicts.iter().any(|c| c.escalated) { algo.vetoed = true; algo.summary.push_str("同级冲突无法仲裁，请人工审批"); }`
- `crates/mox-expert/tests/reconcile_conflict_tests.rs`：8+ 场景单测

### Task-local Test Requirements (TR >= 8)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T5-TR1 | rule | Permission=7 vs Security=7 冲突升级 Blocking：构造双约束（同节点 node_A Permission.MustGuard vs Security.MustIsolate 同 Kind=Guard）→ conflicts Vec 非空含 escalated=true | conflicts.len() >= 1 AND escalated=true | same_priority_7vs7_escalate.log |
| T5-TR2 | rule | conflicts 非空断言：10 条冲突图（覆盖 7 维度×3 种 Kind）→ `plan.conflicts.len() >= 1` 10/10；零冲突图 5 条 → conflicts.len() == 0 5/5 | 10/10 nonempty AND 5/5 empty = | conflicts_nonempty_assert.log |
| T5-TR3 | rule | 低优先级被高优先级正确覆盖：Algorithm(priority=2) MustSerialize vs Resource(priority=6) ResourceCap → 最终 ReconciledPlan 保留 Resource 约束；Algorithm 约束被 discard；无 escalated（不同级不升级） | retain_resource_only AND discard_algo AND escalated=false | high_priority_overwrites_low.log |
| T5-TR4 | rule | 3 场景裁决确定无随机性：同输入图连续跑 reconcile 30 次 → plan.conflicts 完全一致（序列化字节相等）；无 sort_unstable 导致的抖动 | 30/30 plan.serialize() = | deterministic_30runs.log |
| T5-TR5 | rule | 互补约束（MustGuard + MustIsolate）不同 Kind → 记录 semantic 溯源但不 escalated；不阻断正常出码 | escalated=false AND semantic record present | complementary_not_escalated.log |
| T5-TR6 | rule | reason_code 可解释：冲突出码附带 reason_code=SAME_PRIORITY_SAME_KIND_OVERLAP；UI 闸门页显示冲突双方维度、节点 ID、建议修复动作（"请调整 node_A 仅保留一位专家约束"） | UI reason_code render 5/5 scenes | reason_code_ui_screenshot.html |
| T5-TR7 | rule | pipeline.vetoed 传导：escalated=true → `algo.vetoed=true` → govern() GateResult.approved=false（即使其他维度无风险） | approved=false AND vetoed=true 3/3 | escalated_to_gate_veto.log |
| T5-TR8 | rule | 旧回归用例 `missing_desensitize_blocked_by_gate` 仍通过（T4 修复 + T5 冲突检测不误杀互补 MustGuard+MustIsolate）→ pass 1/1 | regression pass | p2_p1_joint_regression.log |

### Completion Evidence
_（待实施时填写：7v7 升级截图、30 次确定性 SHA256 对比、reason_code UI 渲染快照、pipeline→govern veto 传导链路日志）_

---

## Task 6: Suggestion × Constraint 语义交叉校验（P3 缺陷 · 静默冲突 → 可解释溯源）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-6（P3 根治）；OUT-8 冲突溯源；M1 里程碑缺陷修复 |
| Dependencies | Task 1（mox-expert 骨架）+ Task 5（conflicts Vec 可承载 semantic） |
| Blocked By | Task 5（需 conflicts 结构新增 semantic 字段） |
| Unblock Condition | Task 5 `ReconcileConflict` 含 `semantic_type: Option<SemanticConflictType>` 字段 |

### 核心产物清单
- 新增枚举 `crates/mox-expert/src/reconcile.rs`：
  - `enum SuggestionKind { Parallelize, Cache, Merge, Offload, Batch, Retry }`
  - `impl Suggestion { pub fn kind(&self) -> SuggestionKind; pub fn nodes(&self) -> &[NodeId]; }`
  - `enum SemanticConflictType { SerializeVsParallelize, GuardVsOffload, ResourceCapVsBatch }`
- 修改 reconcile.rs：
  - 收集所有专家 `ExpertOpinion.suggestions` 进入 `pending_suggestions: Vec<Suggestion>`
  - 交叉循环：对每个 Suggestion（尤其 Parallelize）遍历所有已采纳 Constraint；若 MustSerialize.nodes() 与 Parallelize.nodes() 交集非空 →
    - 标记 Suggestion 为 "not adopted"（不写入 ReconciledPlan.adopted_suggestions）
    - push ReconcileConflict { semantic_type: Some(SerializeVsParallelize), escalated: false, reason_code: ReasonCode::SEMANTIC_OPPOSITE_SERIAL_PARALLEL }
  - 非冲突 Suggestion（Cache/Merge/Offload 等）正常采纳进 `ReconciledPlan.adopted_suggestions: Vec<Suggestion>`
- 修改 `crates/mox-expert/src/govern.rs`：GovernanceReport 含 `adopted_suggestions` 字段（UI 可展示采纳了哪些软建议）
- `crates/mox-expert/tests/suggestion_constraint_cross.rs`：6+ 场景单测

### Task-local Test Requirements (TR >= 6)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T6-TR1 | rule | Parallelize vs MustSerialize 语义冲突溯源：algorithm 专家出 Suggestion::Parallelize(nodes=[A,B,C]) + data 专家出 Constraint::MustSerialize(nodes=[B,C]) → 冲突 Vec 含 SEMANTIC_OPPOSITE_SERIAL_PARALLEL；Parallelize 未被 adopted（adopted_suggestions.len() = 其他建议数 - 1） | conflict present AND not adopted 1/1 | serialize_vs_parallelize_trace.log |
| T6-TR2 | rule | 非冲突正常通过：Parallelize([X,Y,Z]) + MustSerialize([A,B]) 节点完全不相交 → Parallelize 被 adopted；conflicts Vec 不含 SEMANTIC_OPPOSITE 类型 | adopted=true AND no conflict | non_conflict_pass.log |
| T6-TR3 | rule | 冲突输出可解释原因码：UI 渲染 GovernanceReport → suggestion "Parallelize(node B,C)" 旁显示红色气泡「与 data 专家 MustSerialize 串行约束冲突；建议人工确认是否允许并行」；reason_code = 0x0201（可映射帮助文档 URL） | UI render 气泡 + reason_code=0x0201 | semantic_conflict_ui.html |
| T6-TR4 | rule | Cache/Merge/Offload 与任何 Constraint 无冲突 → 均被 adopted；adopted_suggestions 计数与输入一致（3/3） | 3/3 adopted AND 0 semantic conflict | other_suggestions_adopted.log |
| T6-TR5 | rule | GuardVsOffload：MustGuard(A) + Suggestion::Offload(A)（offload 可能绕过 Guard 节点）→ semantic 冲突标记；Offload 不被 adopted | conflict present AND offload not adopted | guard_vs_offload.log |
| T6-TR6 | rule | adopted_suggestions 审计链写入：每次采纳 Suggestion 生成 AuditChain::SuggestionAdopted(suggestion_id, adopted_by, dimension_of_conflicting_constraint_if_any)；链式完整性校验通过（100 连续块） | 100/100 chain integrity AND adopted records count = | adopted_suggestions_audit_chain.log |

### Completion Evidence
_（待实施时填写：Parallelize×MustSerialize 冲突截图、原因码 0x0201 UI 渲染、审计链 adopted_suggestions 段、非冲突通过日志）_

---

## Task 7: constants.rs 归一（P4 缺陷 · 常量散落 10+ 文件 → 唯一权威 SSOT）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-7（P4 根治）；M1 里程碑缺陷修复；KB §6.1 constants/ 目录与代码双向绑定 |
| Dependencies | Task 1（mox-expert 骨架）+ Task 4（sensitivity.rs，前缀常量迁入或共享 policy） |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- 新建 `crates/mox-expert/src/constants.rs`：SSOT 集中导出：
  - `pub const SENSITIVE_PREFIXES: &[&str]`（T4 sensitivity.rs 改为引用此处）
  - `pub const PRODUCTION_PREFIXES: &[&str]`
  - `pub const DESENSITIZED_SUFFIXES: &[&str]`（`_safe/_desensitized/_masked/_anon`）
  - `pub const BLOCKING_SCORE_DEDUCTION: f64 = 0.5;`
  - `pub const WARNING_SCORE_DEDUCTION: f64 = 0.2;`
  - `pub const DIMENSION_PRIORITY: [(Dimension, u16); 7]`：`[(Permission, 100), (Security, 100), (Resource, 90), (Data, 80), (Business, 70), (Observability, 60), (Algorithm, 50)]`（7→100 保持 Permission/Security 同级，7-2 优先级差 10）
  - `pub const DEFAULT_QUOTA_CONCURRENCY: u32 = 8;`
  - `pub const DEFAULT_QUOTA_TOKEN_RATE: f64 = 1.0;`
  - `pub const DEFAULT_QUOTA_TIMEOUT_MS: u64 = 5000;`
  - `pub const FUZZY_WORDS: &[&str]`：`["尽量", "差不多", "尽可能", "大概", "或许"]`
  - `pub const GUARD_NODE_DEFAULT_DURATION_MS: u64 = 5;`
  - `pub const ROLE_NAMES: &[&str]`：`["admin", "editor", "viewer", "auditor", "operator"]`（角色字符串枚举化前置步骤）
- 可选新建 `crates/mox-expert/policy.toml`：镜像 constants.rs（便于运维无代码修改）；`constants.rs` 含 `include_str!` + 运行时校验两者一致（不一致 compile_error!）
- 修改 10+ 调用处：
  - `ir.rs::Dimension::priority()` → 委托 `constants::DIMENSION_PRIORITY`
  - `expert.rs:113,115` 扣分魔法数字 → 引用 BLOCKING_SCORE_DEDUCTION / WARNING_SCORE_DEDUCTION
  - `context.rs:87` 默认配额 → 引用 DEFAULT_QUOTA_*
  - `programming.rs:86-90` 模糊词 → 引用 FUZZY_WORDS
  - `reconcile.rs:72` duration_ms → 引用 GUARD_NODE_DEFAULT_DURATION_MS
  - `permission.rs`/`security.rs` 敏感前缀 → 通过 sensitivity.rs 间接触发 constants.rs
- `crates/mox-expert/tests/constants_ssot.rs`：5+ 单测（断言常量值 + grep 验证无漂移副本）

### Task-local Test Requirements (TR >= 5)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T7-TR1 | rule | 敏感前缀 ×3 统一源：permission.rs / security.rs / 其他处 不再硬编码数组；`rg 'SENSITIVE_PREFIXES|sensitive_prefixes\s*=' crates/mox-expert/src --count` 只在 constants.rs 和 sensitivity.rs 出现（调用）；其他 0 处 | file set = {constants.rs, sensitivity.rs} 只有 2 文件定义/引用定义 | grep_no_drift_definitions.log |
| T7-TR2 | rule | 扣分权重 0.5 / 0.2：BLOCKING_SCORE_DEDUCTION = 0.5 AND WARNING_SCORE_DEDUCTION = 0.2；`rg 'push_risk.*\* *0\.|deduction.*= *0\.[25]'` 不出现魔法数字（专家层）；全改为引用常量 | 0 magic occurrences AND 2 const value = | grep_no_drift_weights.log |
| T7-TR3 | rule | 维度优先级 7-2：DIMENSION_PRIORITY 含 Permission=100, Security=100, Resource=90, Data=80, Business=70, Observability=60, Algorithm=50（差 10）；`ir.rs::priority()` 单测 7 维度分别返回对应值 | 7/7 values AND fn delegation | dim_priority_7dims.log |
| T7-TR4 | rule | 默认配额 8/1.0/5000ms：`DEFAULT_QUOTA_CONCURRENCY/TIMEOUT/RATE` 三常量；context.rs::Quota::default() 三处均引用；grep "= 8;" / "= 5000;" 配额相关只在 constants.rs 定义 | grep matches count = 1 per value 仅 constants 处 | grep_no_drift_quota.log |
| T7-TR5 | rule | 模糊词表 5 词均来自唯一源 constants.rs / policy.toml：programming.rs 5 模糊词对比 FUZZY_WORDS =；policy.toml [fuzzy] words 数组也 =（一致） | two-sources equal AND programming.rs no hardcode list | fuzzy_ssot_roundtrip.log |

### Completion Evidence
_（待实施时填写：rg 零魔法数字报告、dim_priority 单测 7 维度截图、policy.toml↔constants.rs 一致性校验脚本输出、FUZZY_WORDS 双向相等断言）_

---

## Task 8: 七层骨架搭通（ir → expert → reconcile → verify → govern → programming → harness）零断点

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-8；四条不变式（节点唯一/专家只读/裁决不求解/否决单向）；M2 里程碑架构层 |
| Dependencies | Task 1（骨架）+ Task 4-7（P1-P4 缺陷全部修复完毕） |
| Blocked By | Task 5（Reconcile conflicts 非空）|
| Unblock Condition | Task 5 T5-TR1 & T5-TR3 通过（裁决器升级机制可用） |

### 核心产物清单
- `crates/mox-expert/src/pipeline.rs`：`Pipeline` struct，七层链式调用接口：
  - `Pipeline::new(flow_graph)` → `pipeline.normalize()`（ir 着色）→ `pipeline.dispatch_experts()`（7 专家只读并行）→ `pipeline.reconcile()`（约束翻译）→ `pipeline.verify()`（不变式校验 vetoed）→ `pipeline.govern()`（闸门 approved）→ `programming.five_guards_check()`（G-A~G-E）→ `harness.execute()`（运行时）
  - 每层输出 `LayerOutput<T>` 含 `passed: bool` + `artifacts` + `next_layer_input`
- `crates/mox-expert/src/ir.rs`：`FlowGraph.auto_dimension()` 补全缺失 DimensionTag（节点着色零遗漏）
- `crates/mox-expert/src/expert.rs`：`Expert::read_only_contract()` 断言（&self 不可变；无 &mut self 方法）
- `crates/mox-expert/src/verify.rs`：`verify()` 实现最高权限；`algo.vetoed=true` 时治理层不可覆盖（不变式 4）
- `crates/mox-expert/src/programming.rs`：五道护栏 G-A~G-E 接入 pipeline（`programming_gate()` 返回 `GateResult`）
- `crates/mox-expert/src/harness.rs`：`Harness::run_plan(ReconciledPlan)` 执行；含 PreGate/PostGate 瀑布钩子
- `crates/mox-expert/tests/seven_layer_e2e.rs`：8+ 链路单测

### Task-local Test Requirements (TR >= 8)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T8-TR1 | rule | 七层零断点：构造合法 gov-pii 图 → Pipeline 七层全 passed=true；每层 LayerOutput.artifacts 非空；next_layer_input 可序列化传递 | 7/7 layers passed=true AND artifacts len>0 each | seven_layer_full_pass.log |
| T8-TR2 | rule | 节点唯一不变式：`FlowGraph` 加 2 次同 id 节点 → merge 而非 duplicate；node_count 仍 = 1；4 种维度标签挂在同一节点 | dedup 100% (5/5 duplicates) | graph_node_uniqueness.log |
| T8-TR3 | rule | 专家只读不变式：7 专家 `ExpertOpinion::analyze(&self, graph: &FlowGraph) -> ExpertOpinion` 签名全 &self；编译期无 &mut self 方法；dispatch 并行（rayon 10 线程）结果一致（10 次 =） | 7/7 signatures + 10/10 parallel deterministic | expert_readonly_contract.log |
| T8-TR4 | rule | 裁决不求解不变式：`reconcile()` 内部不调用任何 `flow_ai::optimize` 相关 symbol；grep reconcile.rs "flow_ai" count = 0；唯一求解调用在 harness 层 | grep count=0 AND harness call present 1/1 | reconcile_nosolve_invariant.log |
| T8-TR5 | rule | 否决单向不变式：`algo.vetoed=true` → `govern()` 即使其他条件满足仍返回 approved=false；覆写尝试（`force_override=true`）编译期不可用（private field） | approved=false AND force_override compile_error | veto_oneway_invariant.log |
| T8-TR6 | rule | G-A~G-E 五道护栏：草稿 AiDraft → 执行阻断 100%；动作未映射节点 → G-B 阻断；三证缺一（!vetoed但!approved）→ G-C 阻断；未署名 → G-D 阻断；失败回退最近 Checkpoint → G-E 回退成功 | 5/5 guard scenarios blocked_or_rollback | programming_five_guards.log |
| T8-TR7 | rule | LoopStart 未登记：图含 LoopStart 但 registry 无 LoopGuard → verify() 设 vetoed=true；Block 理由 "无界循环默认否决（保守优先）" | vetoed=true 3/3 loop graphs | loop_unregistered_veto.log |
| T8-TR8 | rule | 失败层短路：Layer_02 dispatch_experts 返回 Permission Veto → Layer_03 reconcile 仍执行（记录冲突）→ Layer_04 verify 设 vetoed → Layer_05-07 跳过（passed=false，短路耗时 ≤ 1 ms） | short-circuit AND time <= 1 ms | pipeline_shortcircuit_failfast.log |

### Completion Evidence
_（待实施时填写：七层全通链路 trace、专家只读签名 grep 报告、否决单向编译期错误截图、五道护栏阻断快照）_

---

## Task 9: PPR 意图路由（Prompt → Plan → Resolve）三阶段分流

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-9；M2 里程碑架构层；NFR-3 并发吞吐路由层 |
| Dependencies | Task 1（骨架）+ Task 8（七层 pipeline 搭通） |
| Blocked By | Task 8（需 pipeline.passed 输出作为路由反馈） |
| Unblock Condition | Task 8 T8-TR1 通过 |

### 核心产物清单
- 新建 crate `crates/mox-ppr-router/Cargo.toml` + src/lib.rs：
  - `struct PprRouter { classifiers: Vec<Box<dyn IntentClassifier>>, routes: RouterConfig }`
  - `enum Phase { PromptParse, PlanDispatch, ResolveExecution }`
  - `enum IntentCategory { VoiceCommand, CodeEdit, DataQuery, ComplianceCheck, GraphOptimize, ExpertAudit, UnknownFallback }`
  - `PromptClassifier`（LLM + rules hybrid）：从原始输入提取 slot（`action / resource / constraints / voice_session_id?`）
  - `PlanDispatcher`：按 IntentCategory 路由到对应后端：
    - VoiceCommand → xiaobai_voice_service (ASR/TTS 会话)
    - CodeEdit → mox_flow_ai 优化器
    - DataQuery → mox_graph_query
    - ComplianceCheck → mox_expert_pipeline (专家联盟)
    - GraphOptimize → mox_flow_ai
    - ExpertAudit → mox_expert_pipeline (审计链)
    - UnknownFallback → LLM + Confidence<0.7 → human-in-the-loop toast
  - `ResolveExecutor`：执行后端 plan + 状态回写 + PPR 指标上报
- `crates/mox-ppr-router/src/config.rs`：RouterConfig（每类 intent 的超时、重试、Fallback）
- 修改 `crates/mox-expert/src/pipeline.rs`：接入 PPR 作为入口（`Pipeline::from_ppr_intent`）
- `crates/mox-ppr-router/tests/ppr_scenarios.rs`：8+ 单测

### Task-local Test Requirements (TR >= 8)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T9-TR1 | rule | PromptParse 准确率：100 条标准 prompt 标注 → IntentCategory 分类匹配度 ≥ 98%；confusion matrix F1 ≥ 0.97（VoiceCommand×20/CodeEdit×20/Data×15/Compliance×20/Optimize×10/Audit×10/Unknown×5） | 98/100 correct AND F1 >= 0.97 | ppr_prompt_classification_report.json |
| T9-TR2 | rule | PlanDispatch 零错路由：20 条每类 intent（140 总）→ 后端命中 correct_backend 100%；VoiceCommand → xiaobai_voice；Compliance → mox_expert；Unknown → fallback_human | 140/140 route correct | ppr_plan_dispatch_140.log |
| T9-TR3 | rule | ResolveExecution 完成率：200 任务执行（各 intent 混合）→ success rate >= 99%；失败 1% 以内自动重试 3 次仍失败则写 DLQ（可观测） | success >= 99% AND DLQ 可查询 | ppr_resolve_success_200.log |
| T9-TR4 | rule | 语音 50 并发路由吞吐：50 并发 VoiceCommand intent → PPR 层 P99 latency <= 200 ms；分发到 ASR/TTS 不积压（队列 len 水位 <= 2） | P99 <= 200 ms AND queue <= 2 95% 时间 | ppr_concurrency_50.csv |
| T9-TR5 | rule | UnknownFallback + human-in-the-loop：10 条 ambiguous 输入 → confidence < 0.7 → toast 显示「无法确定意图，请选择：A 语音助手 B 代码编辑 C 合规检查」；选 A 后路由重定向正确 | toast shown AND redirect ok 10/10 | ppr_unknown_humanloop.html |
| T9-TR6 | rule | PPR 路由可观测：`ppr_phase_latency_us{P=Prompt|Plan|Resolve}` 三个 histogram；`ppr_route_total{intent=VoiceCommand}` counter；指标 5 项全在 prometheus.gather 出现 | 5/5 metrics present | ppr_metrics_prom.log |
| T9-TR7 | rule | 超时熔断：DataQuery 慢路由（模拟 backend 5 s 超时）→ RouterConfig.data_timeout=2s → Resolve 2 s 熔断；fallback 缓存上次结果（若有）；否则返回 GatewayTimeout 友好错误 | timeout <= 2.1 s AND fallback or 友好 error 5/5 scenes | ppr_circuit_breaker.log |
| T9-TR8 | rule | 路由灰度 A/B：PlanDispatcher 10% 流量切新后端 "GraphOptimize_v2"；指标 `ppr_ab_test{variant}` 区分；旧版 v1 与 v2 路由正确性均 100%（不破坏 baseline） | v1/v2 correctness 100% AND ab bucket ratio = 90/10 ±1% | ppr_ab_routing_report.log |

### Completion Evidence
_（待实施时填写：分类报告混淆矩阵、140 路由零错日志、50 并发 P99 延迟 CSV、HITL toast 截图、A/B 灰度配比报告）_

---

## Task 10: MOX 算子鉴权（RBAC + Capability 单入口 + 跨租户隔离）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-10；NFR-6 安全合规；M2 里程碑架构层；P5 次要缺陷（鉴权双轨归一） |
| Dependencies | Task 1（骨架）+ Task 7（constants.rs ROLE_NAMES / Capability 映射） |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- 修改 `crates/mox-expert/src/context.rs`：
  - 删除旧的字符串硬编码角色匹配（`if role == "editor"` 等）
  - `impl Context { pub fn can(&self, cap: Capability, resource: &str) -> bool` → 内部委托 `rbac::check(self.principal(), cap_action(cap), resource)`
  - `fn cap_action(cap: Capability) -> &'static str` 映射：
    - ViewAudit → "read:audit:*"
    - RunFlow → "execute:flow:*"
    - EditFlow → "write:flow:*"
    - ApproveFlow → "admin:flow:gov-pii/*"
    - ViewMetrics → "read:metrics:*"
- 扩展 `crates/mox-expert/src/rbac/policy.rs`：
  - `PermissionCheck { subject: String, action: String, resource: String, tenant_id: TenantId }`
  - 通配符 `db:prod/*` 匹配；角色继承链 `admin ⊇ editor ⊇ viewer`；auditor 单独权限位（ViewAudit/ViewMetrics）
  - `cross_tenant_isolation` 强制：tenant_id != resource.tenant_prefix → 自动 reject（即使 admin 跨租户也不行，必须 AssumeRole）
- `crates/mox-expert/src/rbac/session.rs`：STSToken decode 含 tenant_id + roles；`AssumeRole` 不得提权（目标 clearance <= 源 clearance，否则 SignError）
- 修改 7 位专家 `experts/*.rs`：专家层越权检查统一走 `ctx.can(EditFlow, flow_resource_id)`；不再各自魔法字符串比对
- `crates/mox-expert/tests/rbac_sso_entry.rs`：8+ 单测

### Task-local Test Requirements (TR >= 8)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T10-TR1 | rule | RBAC 角色矩阵 4 档 × 5 Capability × 3 资源 = 60 场景裁决全对（viewer 只能 view；editor 可 edit 不可 approve；admin 全过；auditor 仅 audit/metrics） | 60/60 correct | rbac_role_matrix_60.log |
| T10-TR2 | rule | 通配符匹配：editor 授权 `write:flow:projectA/*` → `write:flow:projectA/sub/flow1` = allow；`write:flow:projectB/x` = deny；跨前缀 deny 10/10 | allow 10 deny 10 = 20/20 correct | rbac_wildcard_match_20.log |
| T10-TR3 | rule | 跨租户隔离：tenantA admin 访问 `resource:tenantB/db/...` → reject 100%；AssumeRole 到 tenantB 后再访问 → allow（若目标角色有权限）；无 AssumeRole 跨租户访问=0 allow | 10/10 reject AND role-assume allow 10/10 | rbac_tenant_isolation_20.log |
| T10-TR4 | rule | Capability 单入口断言：7 专家 × 每专家 3 越权检查点 = 21 检查点全走 `ctx.can()`；grep 专家文件 `role == "` 出现次数 = 0（无魔法字符串） | 0 occurrences AND 21/21 can() calls | single_entry_grep_21.log |
| T10-TR5 | rule | STS 防提权：源 clearance=2(秘密) AssumeRole → 目标 clearance=3(机密) → SignError 令牌签名失败；解码后 target clearance <= source 才 pass |提权 fail 5/5 AND 不降级 pass 5/5 | sts_nopriv_escalation_10.log |
| T10-TR6 | rule | 越权拒绝率 100%：50 条未授权请求（各类 cap×资源×角色无权限组合）→ reject 50/50；审计链写入 RbacDenied record 50 条完整；响应头 `X-Mox-Deny-Reason` 语义不泄漏 | 50/50 reject AND audit records 50 AND reason 无泄露 | rbac_deny_rate_50.log |
| T10-TR7 | rule | 旧版双轨回归：原 context.can 硬编码用例（T4 既有 `rbac_editor_can_edit_flow` 等）→ 新单入口实现仍 100% 通过（零回归） | regression pass 10/10 | rbac_regression_old10.log |
| T10-TR8 | rule | RBAC 策略可重载（policy.toml 热更新）：动态加角色 `custom:project_manager`；授予 `write:flow:proj_x/*`；10 s 内 Context.can(EditFlow, proj_x/a) = allow；未命中 proj_y/a = deny | allow+deny 2/2 after reload within 10s | rbac_policy_hotreload.log |

### Completion Evidence
_（待实施时填写：60 角色矩阵报告、魔法字符串 grep=0 截图、STS 提权失败签名错误、50 越权拒绝审计链段）_

---

## Task 11: 专家联盟 G1~G8 闸门（三证齐全 verify✓ + roundtrip✓ + approved✓ 才出码）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-11；NFR-6 合规四道护栏；M2 里程碑架构层；§1.4 programming五道 + expert×7 融合 |
| Dependencies | Task 8（七层 pipeline）+ Task 10（鉴权单入口）+ Task 4（PII）+ Task 5（冲突升级） |
| Blocked By | Task 8 T8-TR6（五道护栏可用）+ Task 10 T10-TR2（RBAC 生效） |
| Unblock Condition | Task 8 T8-TR6 & T8-TR5 通过 |

### 核心产物清单
- 新建 `crates/mox-expert/src/alliance.rs`：专家联盟闸门 8 道：
  - **G1 草稿隔离**：DraftStatus != AiDraft（对齐编程 G-A）
  - **G2 维度着色齐全**：所有节点 DimensionTag 存在（非 None）；auto_dimension 覆盖率 = 100%
  - **G3 三证齐全核心**：`!algo.vetoed`（verify✓）AND `roundtrip_ok`（双向一致✓）AND `gate.approved`（治理闸门✓）
  - **G4 鉴权到位**：操作者 `ctx.can(EditFlow, flow_id)`（对齐 T10 单入口）
  - **G5 PII 合规完整**：SensitiveWriteGuard 已插入（对齐 T4 sensitivity + permission MustGuard）
  - **G6 冲突无升级**：`plan.conflicts.iter().all(|c| !c.escalated)`（对齐 T5；无同级无法仲裁升级）
  - **G7 语义无静默冲突**：Suggestion×Constraint 无 SEMANTIC_OPPOSITE（或有但已人工审批 approved_by_human=true）（对齐 T6）
  - **G8 审计链署名完整**：`authored_by` 含（模型/版本/专家视角）+ 首块哈希链有效（对齐编程 G-D + 审计链 §OUT-7）
  - `pub enum AllianceGate { G1..G8 }`
  - `pub fn pass_alliance(flow: &ValidatedFlow, ctx: &Context, plan: &ReconciledPlan, gov: &GovernanceReport) -> AllianceResult { 8 道逐道短路 }`
  - `AllianceResult { passed: bool, failed_gates: Vec<AllianceGate>, remediation: Vec<(AllianceGate, String)> }`
- 修改 pipeline.rs：programming 五道之后追加 alliance 八道；失败返回 AllianceFailed(passed_gate: Gx)
- `crates/mox-expert/tests/alliance_gates_g1_g8.rs`：8+ 场景单测

### Task-local Test Requirements (TR >= 8)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T11-TR1 | rule | G1 草稿隔离：status=AiDraft → G1 fail；status=HumanReviewed → G1 pass；状态切换影响 pass_alliance 结果 3/3 场景 | block+draft fail；reviewed pass 3/3 | alliance_g1_draft.log |
| T11-TR2 | rule | G3 三证缺一不可：三条件 2^3-1=7 种非法组合（仅缺 veto / 缺 roundtrip / 缺 approved / 两两缺 / 三缺）→ fail 7/7；全满足 → pass 1/1 | 7/7 fail AND 1/1 pass = 8 场景全对 | alliance_g3_three_certs.log |
| T11-TR3 | rule | G4 鉴权：viewer 无权 EditFlow → G4 fail 100%；editor 有权 → pass；admin 跨租户（无 AssumeRole）→ fail 5/5 | 5/5 viewer fail + editor pass + tenantfail = 15/15 correct | alliance_g4_rbac.log |
| T11-TR4 | rule | G5 PII Guard：`db:prod/citizen/info` 写操作 + 未插 Guard 节点 → G5 fail remediation="插入 MustGuard(SensitiveWrite)"；插 Guard → pass | fail+pass 2/2 AND remediation text match | alliance_g5_pii_guard.log |
| T11-TR5 | rule | G6 冲突升级：Permission/Security 同 Kind 同级冲突 → escalated=true → G6 fail；互补 MustGuard+MustIsolate escalated=false → G6 pass | fail due escalated + pass no escalate 4/4 scenes | alliance_g6_escalation.log |
| T11-TR6 | rule | G7 语义冲突 + 人工审批：Parallelize vs MustSerialize → SEMANTIC_OPPOSITE + approved_by_human=false → G7 fail；同冲突 + approved_by_human=true → G7 pass | fail then pass after approval override 2/2 | alliance_g7_semantic_approval.log |
| T11-TR7 | rule | G8 审计链署名：authored_by 缺失（空串）→ G8 fail remediation="署名必填"；哈希链首块 invalid → G8 fail；均 ok → pass；3 场景 | 3/3 correct AND remediation filled when failed | alliance_g8_audit_chain.log |
| T11-TR8 | rule | G1~G8 全通过典型 gov-pii 图：构造合法合规 gov-pii 图 → 8 闸门全 passed=true；AllianceResult.failed_gates 空集；remediation 空 | 8/8 gates pass AND failed_gates=[] AND remediation=[] | alliance_fullpass_govpii.log |

### Completion Evidence
_（待实施时填写：G3 7 非法组合全 fail 截图、G5 PII 未插 Guard 阻断界面、8 闸门全通过 gov-pii 链路 trace、人工审批覆盖 G7 日志）_

---

## Task 12: 客户端浮窗完善 + PyInstaller 打包 + audio_play 死锁根治 + _ensure_windowed_streams 兜底

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | FR-3 / FR-12 / FR-13；NFR-15 启动性能；M1 里程碑核心交互 & 打包；M2 里程碑收尾 |
| Dependencies | Task 2（ASR 可用）+ Task 3（TTS/playback 会话化）+ Task 1（骨架） |
| Blocked By | Task 3（_PlaySession 对象需先实现） |
| Unblock Condition | Task 3 T3-TR14 通过（playback 1000 轮零死锁） |

### 核心产物清单
#### 12.1 桌面浮窗 & 交互
- `projects/xiaobai_voice/xiaobai_voice/desktop/ball_widget.py`：PySide6 Frameless + StaysOnTop + TranslucentBackground；4 状态（idle=#22d3ee / listen=#22c55e 呼吸 / think=#a855f7 旋转 / speak=#6366f1 波形）；拖拽吸附左右边缘（300 ms OutCubic 动画）
- `projects/xiaobai_voice/xiaobai_voice/desktop/hotkeys.py`：pynput 全局监听独立 QThread；Alt+X 录音 / Alt+S 停止 / Alt+Q 退出；toast 剪贴板朗读
- `projects/xiaobai_voice/xiaobai_voice/desktop/tray.py`：系统托盘；右键菜单 7 项（设置 / 录音 / 剪贴板朗读 / 开机自启 / 浮窗显示隐藏 / 检查更新 / 退出）
- `projects/xiaobai_voice/xiaobai_voice/desktop/app.py`：QApplication；双击浮窗跳转主程序（WebView 打开 /#/ai 路由）

#### 12.2 死锁修复核心（FR-12 回归）
- 重构 `projects/xiaobai_voice/xiaobai_voice/tts/playback.py`：
  - `_PlaySession(uuid, state_lock: RLock, audio_device_lock: RLock, playback_state: Enum)` 会话化
  - **铁律**：`play()` 持 `state_lock` 期间**禁止**调用 `stop()`（重入死锁根因）；改为 `stop()` 调 `session.request_stop()`（仅 CAS 设置原子 flag=STOP_REQUESTED）；`play()` 循环内 `if stop_requested.is_set(): break` 优雅退出释放两把锁
  - 钢琴播放场景：`enqueue_notes()` 批量插入 C4-E4-G4 和弦 → 会话化串行播放（避免并发 stop/play 冲突）
  - `DeadlockDetector`：后台线程 100 ms 轮询；若 play 持锁 > 5 s 且无音频字节写入 → 记录指标 `deadlock_detector_count++` + 强制释放（Rust drop 语义 Python 版：del session 然后 GC collect）

#### 12.3 PyInstaller 打包 + stderr 兜底
- `projects/xiaobai_voice/build/pyinstaller.spec`：`console=False` + `name="xiaobai-voice"` + `onefile`/`onedir` 双配置
- `projects/xiaobai_voice/xiaobai_voice/cli.py` 入口：
  - `_ensure_windowed_streams()`：运行 `if not sys.stdout: sys.stdout = open(os.devnull, "w"); if not sys.stderr: sys.stderr = _get_stderr_logfile()`
  - stderr logfile：`%APPDATA%\mox\xiaobai\logs\stderr_YYYYMMDD_HHMMSS.log`（旋转 7 天）
  - 写入第一条 `[PYI-WINDOWED] stderr 就绪 pid={pid}`（排查闪退的第一抓手）
  - 外部 venv 加载：`--python-home` 参数；优先 `%EXE_DIR%/venv/Lib/site-packages` 注入 sys.path；失败回退内置 runtime
- `projects/xiaobai_voice/build/run_as_user.ps1`：PyInstaller Start-Process 双击方式启动模拟；验证零闪退

#### 12.4 日志 & 自启
- 日志路径：跨平台 `%APPDATA%/mox/xiaobai/logs/`（Windows）/ `$XDG_CONFIG_HOME/mox/xiaobai/logs/`（Linux）/ `~/Library/Logs/mox/xiaobai/`（macOS）
- 开机自启：注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 键 `MoxXiaobaiVoice` = `"C:\Program Files\Mox\xiaobai-voice.exe" --tray`；Linux XDG autostart .desktop；macOS LaunchAgent plist
- 外部 venv 加载：`settings.use_external_venv=true` → 路径选择对话框；sys.path 重排

#### 12.5 测试
- `projects/xiaobai_voice/tests/test_desktop.py`：PySide6 QTest 浮窗交互
- `projects/xiaobai_voice/tests/test_pyinstaller.py`：打包后冒烟
- `projects/xiaobai_voice/tests/test_deadlock_regression.py`：死锁回归压力测试

### Task-local Test Requirements (TR >= 12)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T12-TR1 | rule | Alt+X 录音：全局按 Alt+X → 浮窗 listen 呼吸态；3 s 后松键 → ASR 识别写入剪贴板；10/10 快捷键不冲突（系统 Alt+X 占用时 toast 提示改键） | 10/10 录音成功 OR 冲突 toast 显示 | desktop_altx_10.log |
| T12-TR2 | rule | 4 状态切换：idle → listen（点击录音）→ think（ASR 完成后 LLM 请求中）→ speak（TTS 播放中）→ idle 全闭环；每态外环 QPainter 元素不同（呼吸=脉冲圆点数/旋转=弧角度/波形=条形高度数组） | 4/4 states render 不同元素 AND full loop 5/5 | desktop_4states_screenshots.html |
| T12-TR3 | rule | 拖拽吸附边缘：浮窗拖到 (1910, 540) 屏幕中部偏右 1920×1080 → 释放 350 ms 后 X 坐标 1920-68-4 = 1848 ±4；左侧对称吸附 X=0+4=4 ±4；两边各测 5 次共 10 次 | 10/10 snap within ±4 px | desktop_drag_snap_10.log |
| T12-TR4 | rule | 双击跳转主程序：浮窗双击 → WebView 聚焦 /#/ai；若服务 3717 未启动 → WebView 显示「服务未启动，点击启动桌面小白」按钮（不空白 404） | route=#/ai 10/10 + service down friendly btn 5/5 | desktop_doubleclick_jump.html |
| T12-TR5 | rule | PyInstaller Start-Process 双击方式零闪退：打包 onefile → Start-Process xiaobai-voice.exe（无 console）→ 进程存活 ≥ 60 s；托盘图标可见；无 stderr 崩溃 stacktrace；退出码 0（正常关闭时） | alive>=60s AND exit=0 AND no stacktrace 5/5 runs | pyi_startprocess_5runs.log |
| T12-TR6 | rule | _ensure_windowed_streams stderr 兜底：console=False 打包后；人为 raise RuntimeError("crash") → 错误完整写入 `%APPDATA%/mox/xiaobai/logs/stderr_*.log`；日志首行含 `[PYI-WINDOWED] stderr 就绪 pid=` 字段 | crash logged AND pid field exists 3/3 crash scenes | pyi_windowed_stderr_crash3.log |
| T12-TR7 | rule | _PlaySession 钢琴播放死锁冒烟：C4-E4-G4 3 音符连放 × 中途 stop() × play() × stop() 切换 1000 轮；0 死锁；每轮音频设备释放 ≤ 200 ms；DeadlockDetector count = 0 | 1000/1000 no deadlock AND detector count=0 AND release<=200ms | playback_piano_1000.log |
| T12-TR8 | rule | 外部 venv 加载：打包 onedir（不含 torch）→ 外部 venv 装 torch；use_external_venv=true；import torch 成功；torch.cuda.is_available() 正确（按机器）；fallback 内置时功能降级但不崩 | import success OR graceful fallback 3/3 | pyi_external_venv_3.log |
| T12-TR9 | rule | 日志路径 %APPDATA%：Windows 平台 `os.environ["APPDATA"]/mox/xiaobai/logs`；文件 app.log/stderr_*.log/voice_session_*.jsonl 三类齐全；7 天 rotate 旧文件自动清理（仅剩最近 7 天） | 3/3 类别 files exist AND old files deleted after rotate 2/2 | desktop_logs_appdata_5.log |
| T12-TR10 | rule | 开机自启注册表 Run 键：Windows 设置页打开「开机自启」开关 → `reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v MoxXiaobaiVoice` 存在；关开关 → key 删除；无 admin 权限时 toast 提示「需要管理员」 | key add/del 2/2 或 admin required toast 1/1 | autostart_registry_run.log |
| T12-TR11 | rule | play() 持锁禁调 stop() 静态检查：AST 扫描 playback.py 发现 play() 函数体内部显式调用 stop() 0 处；死锁检测器 count 指标启动后 24 h 仍 = 0 | ast scan count=0 AND detector=0 after 24h soak | play_lock_forbid_stop_static.log |
| T12-TR12 | rule | 声卡缺失兜底：机器无音频输出设备（VirtualBox 无声卡模式）→ 播放器返回 AudioDeviceNotFound；UI toast「未检测到声卡」；TTS 浏览器回退也跳过但应用不崩；会话 state 优雅转 idle | toast shown AND app not crash AND state=idle 3/3 no-audio scenes | no_soundcard_fallback3.log |
| T12-TR13 | rule | 剪贴板朗读 Alt+C：复制 100 字文本 → Alt+C → TTS 合成播放；文字长度截断（> 500 字仅读前 500 + toast「超长已截断」） | play ok 5/5 AND truncation+tap 2/2 = total 7/7 | clipboard_read_aloud7.log |

### Completion Evidence
_（待实施时填写：Alt+X 10 次录音成功率、4 状态截图对比、拖拽吸附 10 次坐标 CSV、PyInstaller 打包后 60 s 存活进程列表、stderr 兜底 crash 日志首行、钢琴 1000 轮零死锁报告、注册表 Run 键 reg query 输出）_

---

## Task 13: 云平台多租户 + 会员体系（4 档 Free / Pro / Team / Enterprise）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | FR-14；NFR-7 K8s 弹性；NFR-6 租户隔离；M3 里程碑 SaaS 发布 |
| Dependencies | Task 12（客户端可用）+ Task 10（RBAC 鉴权单入口） |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
- `platform/services/mox-saas-tenant/src/lib.rs`：
  - `struct Tenant { id: TenantId, name, tier: MembershipTier, status: TenantStatus, quota: QuotaConfig, billing: BillingProfile, created_at, regulated: bool }`
  - `enum MembershipTier { Free(uuid) = 0 | Pro = 1 | Team = 2 | Enterprise = 3 }`
  - `struct QuotaConfig { asr_hours_monthly: u32, tts_chars_monthly: u64, voice_sessions_concurrent: u32, expert_calls_daily: u32, storage_gb: u32, api_qps: u32 }`
  - 每档默认配额矩阵：
    - Free: asr=3h / tts=10K chars / sessions=2 / expert=50/day / storage=1GB / qps=5
    - Pro: asr=100h / tts=500K / sessions=10 / expert=5K/day / storage=50GB / qps=100
    - Team: asr=1000h / tts=5M / sessions=100 / expert=50K/day / storage=1TB / qps=1000
    - Enterprise: asr=unlimited / tts=unlimited / sessions=1000 / expert=unlimited / storage=10TB / qps=unlimited
  - `struct BillingProfile { stripe_customer_id, plan_id, next_billing_at, payment_status, invoice_emails }`
  - `trait QuotaEnforcer { fn check_and_consume(tenant_id, resource, amount) -> Result<Consumed, QuotaExceeded>; }`（原子 Redis INCR + 滑动窗口）
  - `cross_tenant_data_isolation()`：所有 DB 查询 WHERE tenant_id = X；Postgres ROW LEVEL SECURITY 启用（物理级兜底）
- `platform/services/mox-saas-tenant/src/routes.rs`：Axum 路由 /tenant/profile /tier/upgrade /quota/usage /billing/invoices /team/members /api_keys/manage
- `frontend/src/pages/saas/`：TierUpgrade.vue + UsageDashboard.vue + InvoiceList.vue + TeamMembers.vue（邀请 / 移除 / 角色设置）
- `platform/services/mox-saas-tenant/tests/tenant_quota_matrix.rs`：10+ 单测

### Task-local Test Requirements (TR >= 10)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T13-TR1 | rule | 4 档会员默认配额：Free/Pro/Team/Enterprise 注册新租户 → QuotaConfig 6 字段与上表完全一致；共 4×6=24 字段值断言正确 | 24/24 = | membership_tier_quota_24.log |
| T13-TR2 | rule | 跨租户数据隔离：创建 tenantA + tenantB 各 10 flow → tenantA token 访问 /flows list 仅 10 条（零 tenantB 数据）；RLS 绕过尝试（直接 SQL 注入 UNION SELECT）→ DB 层过滤；共 5 条绕过尝试 0 泄漏 | 10/10 isolation + 5/5 bypass blocked = 15/15 | cross_tenant_isolation_15.log |
| T13-TR3 | rule | Quota 原子消耗：Free 并发=2 满 → 第 3 会话 QuotaExceeded；HTTP 429 + Retry-After 头 + 升级引导链接；滑动窗口 1 小时后自动解除（时间加速模拟） | 3rd=429 THEN 1h later 3rd=200 5/5 scenes | quota_exceeded_429_5.log |
| T13-TR4 | rule | 升级付费：Free → Pro 点击支付宝/微信支付 → Stripe webhook `invoice.paid` → tier=Pro；配额立即生效（下一次 check_and_consume = Pro 值）；数据库 tier 与 QuotaConfig 同步变更 | webhook received AND tier update + quota update 3/3 | tier_upgrade_stripe_webhook3.log |
| T13-TR5 | rule | Team 成员协作：Team Admin 邀请 5 成员（editor/viewer/auditor 角色）→ 邮件链接；被邀请人加入 → team_members list = 6；viewer 仅看不可编辑 flow；删除成员 → 成员 access_key 撤销（30 s 失效） | 5/5 invite + roles correct + del+revoke ok 3/3 | team_collab_8scenes.log |
| T13-TR6 | rule | 计量计费 metering：asr_hours / tts_chars / expert_calls 三类使用量 1000 次 API 调用 → usage 累加精确到 1；Stripe `usage_record` 上报匹配；误差率 <= 0.001% | sum(usage)-actual <= 1e-5×actual AND stripe count match 3/3 types | metering_billing_accuracy.log |
| T13-TR7 | rule | regulated 租户 PII 外发：tenant.regulated=true → Security 专家 Blocking 生效（对齐 T4）；非 regulated=false → 不启用 regulated 强规则；两个租户对同一 flow 出码结果不同（差异化裁决） | regulated block + non-reg allow 2/2 flows AND different outcomes | tenant_regulated_switch.log |
| T13-TR8 | rule | API Key 管理：租户生成 3 个 API Key（r/w / r-only / audit-only）→ 权限不同（r-only 不可 POST）；key revoke → 下一个请求 401；Redis 缓存 key status TTL 60 s；撤销 60 s 后全局生效 | 3/3 scopes correct + revoke+401 + TTL effective 2/2 = 8/8 | apikey_scope_revoke_8.log |
| T13-TR9 | rule | K8s HPA：100 并发 /tenant/profile 请求 → 指标 CPU>70% → HPA 自动从 min=2 scale 到 max=20；并发降至 10 后 5 min 内缩容回 min+1 | scale out 2→N + scale in N→3 within 5min 2/2 | k8s_hpa_autoscale.log |
| T13-TR10 | rule | 多租户并发 100 租户 × 10 请求 = 1000 请求：错误率 <= 0.1%；P99 延迟 <= 2 s；慢请求（>5 s）数量 <= 5 | err<=1 AND P99<=2s AND slow<=5 1000/1000 | saas_multi_tenant_load_1000.csv |

### Completion Evidence
_（待实施时填写：4 档配额矩阵 24/24 截图、RLS 零泄漏报告、Quota 429 响应头截图、Stripe webhook 升级链路日志、100 租户并发 1000 请求延迟 CSV）_

---

## Task 14: OTA 金丝雀发布 + CDN 分发 + 差分升级（SHA256 校验）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | FR-15；M3 里程碑 OTA 发布；NFR-8 数据持久性（坏包校验回删） |
| Dependencies | Task 12（PyInstaller 包体产出） |
| Blocked By | Task 12 T12-TR5 通过（打包零闪退） |
| Unblock Condition | Task 12 T12-TR5 打包 onefile 成功 3/3 次 |

### 核心产物清单
- `platform/services/mox-ota-differential/src/lib.rs`：
  - `struct OtaRelease { version: SemVer, channel: Channel(Stable/Beta/Canary), size_bytes, sha256, signature, diff_base_versions: Vec<SemVer>, published_by, rollout_percent: u8 }`
  - `enum RolloutPhase { Canary5 = 5 | Canary20 = 20 | Broad50 = 50 | Full100 = 100 }`（四阶段金丝雀）
  - `RolloutPolicy`：每阶段最低驻留 24 h；崩溃率阈值 <= 1%；失败回滚 `ota_success_rate{version} < 0.99 → auto_pause`
  - `bsdiff_rs` 差分算法：从版本 A→B 生成 `diff_{A}_{B}.patch`；patch_size <= full_size × 30%（小文件可例外）
  - `CDN_URLS[3]`：阿里云 CDN / 腾讯云 CDN / Cloudflare R2；健康探测失败自动切换（Round Robin + 失败剔除）
- `platform/services/mox-ota-differential/src/server.rs`：Axum 路由 `/ota/check?version=&arch=&channel=&os=` → 返回 `OtaCheckResponse { update_available: bool, rollout_percent, cdn_urls, sha256, patch?: { from_ver, size, sha256 } }`
  - 百分位路由：`hash(client_uuid) % 100 < rollout_percent → 推送新版本（稳定随机，同一客户端每次一致性）`
- 客户端 `projects/xiaobai_voice/xiaobai_voice/desktop/ota_client.rs`（或 .py）：
  - `OtaManager::check()` → 命中则下载；并行 3 CDN 选最快；Range 断点续传（对齐 Task 5 下载中心）
  - 下载完 SHA256 校验；失败 → `os.remove(path)` + 切换次 CDN 重试；最多 3 次
  - patch 校验：bsdiff_rs patch bspatch 应用 → 产出文件 SHA256 = 服务端公布值；否则 `corrupted_count++` 走全量下载
  - 签名校验：Ed25519 公钥嵌入客户端二进制；release.signature 校验失败 → 拒绝安装（防篡改）
  - 金丝雀上报：安装后 `/ota/report { version, status: installed|crashed, client_uuid }` → 服务端 rollout 决策
- 运维 `ops/ota-dashboard.json`：Grafana 看板 rollout 漏斗（5%→20%→50%→100%）+ 崩溃率
- `platform/services/mox-ota-differential/tests/ota_canary_rollout.rs`：10+ 单测

### Task-local Test Requirements (TR >= 10)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T14-TR1 | rule | 金丝雀四阶段：RolloutPolicy 启动 phase=5% → 20% hash 客户端命中（5/100）→ 驻留 24 h + 崩溃率 0% → 阶段推进到 20% → 驻留 → 50% → 100%；共 4 次 phase transition 全部自动完成（无人工介入） | 4/4 transitions AND crash_rate<1% at each phase | ota_canary_4phases.log |
| T14-TR2 | rule | 百分位路由一致性：同一 client_uuid 100 次 /ota/check → 返回 update_available 真假 100% 一致（hash 稳定）；hash(UUID) % 100 = 42 → phase=5% 未命中，phase=50% 命中 | 100/100 consistent AND (42<50, 42>=5) 命中规则 2/2 | ota_percentile_consistent.log |
| T14-TR3 | rule | CDN 三源健康 + 切换：模拟源 1 down（超时 10 s）→ 客户端 2 s 内切源 2；源 2 down → 切源 3；全部下载字节相等且 SHA256 通过 | switch in <= 2s AND sha= 3/3 sources | ota_cdn_failover_3.log |
| T14-TR4 | rule | 差分包 <= 30% 全量：v1.2.3 → v1.2.4 小步升级 → patch size / full size <= 0.30；bsdiff bspatch 产出 SHA256 = full_pkg_sha256 （5 次不同版本对 5/5） | ratio <= 0.30 5/5 AND post-patch SHA match 5/5 | ota_diff_ratio_sha_5.log |
| T14-TR5 | rule | 坏包自动回删：下载途中注入 4 字节错改（随机位置）→ SHA256 失败 → 文件立即删除（不存在）；corrupted_count++；3 次全失败 → error UI「下载校验失败，请稍后重试」 | file deleted AND count=3 AND UI text match 2/2 scenarios | ota_bad_sha_delete_2.log |
| T14-TR6 | rule | 断点续传：下载 100 MB 包 中途 kill（进程 SIGKILL 50 MB 处）→ 重启后 OtaManager 检测 .part 文件 → Range 50 MB+续接；最终字节数 = 100 MB SHA 匹配；不重复下载已完成字节 | resume 50MB ok AND total download ~100MB（not 150） | ota_resume_partfile.log |
| T14-TR7 | rule | Ed25519 签名防篡改：release.signature 用私钥 Sign（SHA256(pkg_bytes)）→ 客户端内置 pubkey Verify；手工修改 1 字节后签名 Verify=false；拒绝安装 100%；日志写入 `OTA_SIGNATURE_VERIFY_FAIL` 审计 | verify pass 3/3 + tampered verify fail 3/3 | ota_signature_ed25519_6.log |
| T14-TR8 | rule | 崩溃率自动暂停：Canary 5% 阶段 100 客户端 installed → 2 台 report crashed（2%>1%阈值）→ rollout auto pause；phase 不推进到 20%；dashboard 红色告警 | paused=true AND phase_still=5% AND alert shown | ota_crashrate_pause_rollback.log |
| T14-TR9 | rule | 跨平台 Windows/Linux/macOS + x86_64/arm64/aarch64 共 9 组合 → 每组合都有 OTA 包；/ota/check 传入 os/arch 正确返回对应包 sha/size | 9/9 combos packages exist AND correct arch match | ota_multiplatform_9.log |
| T14-TR10 | rule | 零停机热更新：更新包下载验证通过 → 当前客户端 tray 菜单显示「重启更新」；下次启动时 copy-on-swap 替换 exe（Windows MoveFileEx MOVEFILE_DELAY_UNTIL_REBOOT 或下次重启重命名）；0 服务中断（用户手动重启时才替换） | hot update swap without kill current process 3/3 | ota_hotswap_nodowntime3.log |

### Completion Evidence
_（待实施时填写：四阶段推进 Grafana 漏斗、百分位一致性 hash 分布、CDN 切源 2 s 内日志、差分比 30% 柱状图、坏包 SHA 失败删除截图、签名 Ed25519 验证 6/6 报告）_

---

## Task 15: P99 可观测仪表（9 指标 + Grafana JSON 看板）+ 取证页 CSV 导出

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | NFR-5；M3 里程碑可观测；FR-11（审计链）+ OUT-7（HMAC 不可篡改） |
| Dependencies | Task 1（metrics 9 项注册）+ Task 14（OTA 指标来源）+ Task 2/3（ASR/TTS）+ Task 9（PPR） |
| Blocked By | - |
| Unblock Condition | - |

### 核心产物清单
#### 15.1 九项指标体系
- `crates/mox-observability/src/registry_ext.rs` 正式落地（非桩）9 指标（Histogram 6 + Counter 3）：
  1. `asr_cer_bucket`（Histogram）：短句 CER 分布；label: engine=paraformer/sherpa/sensevoice
  2. `tts_mos_bucket`（Histogram）：合成 STOI/MOS 分布；label: tier=fish/cosyvoice/browser
  3. `ppr_route_latency_us`（Histogram）：PPR 三阶段总延迟；label: phase=prompt/plan/resolve
  4. `mox_alliance_gate_latency_us`（Histogram）：G1~G8 八道闸门总耗时；label: passed_gates=[1..8]
  5. `p99_play_session_latency_ms`（Histogram）：_PlaySession play→stop 端到端；label: deadlock_free=true/false（false 就是 detector 触发）
  6. `tenant_quota_usage_ratio_bucket`（Histogram）：配额使用率 /100%；label: tier=free/pro/team/enterprise resource=asr/tts/expert
  7. `ota_success_rate_total`（Counter）：OTA 更新成功/失败计数；label: status=success/fail_sha/fail_sign/fail_bspatch phase=5/20/50/100
  8. `deadlock_detector_count_total`（Counter）：死锁检测器触发（Task 12）；label: module=tts_playback/asr_stream
  9. `voice_session_count_total`（Counter）：并发语音会话数；label: tier strategy=local_first/cloud_fallback/cloud_only
- 指标 Exporter：`mox-observability/src/exporters.rs`（Prometheus text + OpenTelemetry OTLP/gRPC 双协议）

#### 15.2 Grafana JSON 看板
- `ops/grafana/mox-xiaobai-full-dashboard.json`：
  - 行 1：SLO 区（ASR CER P99 / TTS MOS P50 / Alliance 通过率 / 会话错误率）
  - 行 2：语音引擎延迟热力图（ASR 首包 / TTS 首字节 3 CDN 对比）
  - 行 3：OPS 运维区（deadlock_detector 火焰 / OTA 四阶段漏斗 / 租户配额使用率 Top 10）
  - 行 4：PPR 路由分面（7 intent 类别吞吐 + 延迟堆叠 + fallback 占比）
  - 变量：`$env` / `$tenant_tier` / `$strategy` 三下拉；数据源 Prometheus 兼容

#### 15.3 取证页 & CSV 导出
- `crates/mox-expert/src/audit/evidence_page.rs`（或 Web 路由 `GET /ops/audit/evidence`）：
  - 取证参数：`tenant_id?` / `from_ts` / `to_ts` / `block_range=[start_hash..end_hash]` / `format=html|csv|jsonl`
  - HTML 页：链状显示（每块 hash 指向上块）；HMAC 签名验证通过徽章；一键 `导出 CSV` 按钮
  - CSV Schema：`block_height, prev_hash, block_hash, hmac_sig, event_type, dimension, severity, nodes_csv, authored_by, recorded_at, resource_uri, reason_code, remediation`
  - 完整性校验：CSV 内包含最终块 `final_aggregate_hash = HMAC(all_rows)`；接收方离线可重算校验（不可篡改）
- `platform/services/mox-observability/src/routes.rs`：`GET /metrics` Prometheus + `GET /ops/audit/evidence` + `GET /ops/dashboard.json`（直出 Grafana JSON 方便导入）
- `crates/mox-expert/tests/observability_9metrics.rs` + `evidence_page_csv.rs`：单测集合

### Task-local Test Requirements (TR >= 9)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T15-TR1 | rule | 9 指标 Prometheus 命名全存在：`GET /metrics` → grep metric_name 全部 9 条命中；label 维度齐全（每指标至少 1 label） | 9/9 present AND labels count>=1 each | metrics_9names_prom.log |
| T15-TR2 | rule | asr_cer_bucket 采样填充：跑 1000 次短句识别 → Histogram bucket 填充率 >= 95%（le 桶非空 >=95%）；P99 CER 值从 histogram_quantile 可查（返回数字 not NaN） | fill>=95% AND P99 not NaN | asr_cer_1000_bucket_fill.log |
| T15-TR3 | rule | Grafana JSON 合法：`jq .panels | length` = 至少 12 个 panel（行=4 × 每 row ≥ 3 panels）；所有 PromQL 查询字段 `expr` 引用上述 9 指标名零拼写错（可对照 9 指标名验证） | panels>=12 AND expr references 9/9 metrics no typos | grafana_json_jq_panel_exprs.log |
| T15-TR4 | rule | 取证页 CSV 导出：生成 100 blocks × 每块 3 events = 300 events 审计链 → `/evidence?format=csv` 下载 → CSV 行数 = 301（含 header）；字段完整（13 列）；`final_aggregate_hash` 与本地重算 HMAC 完全相等 | rows=301 AND cols=13 AND hmac match 1/1 | evidence_csv_300_integrity.log |
| T15-TR5 | rule | 取证页防篡改：手工修改 CSV 第 42 行 reason_code 字符 → 重算 `final_aggregate_hash` 不相等 → 完整性校验函数返回 false；告警显示 "HASH_MISMATCH line 42" | mismatch detected AND line number shown 2/2 tamper scenes | evidence_tamper_proof_2.log |
| T15-TR6 | rule | OTA 四阶段 Counter：OTA Task 14 T14-TR1 模拟推进时 → `ota_success_rate_total{phase="5"}` increment vs phase="20/50/100" 均与 RolloutPolicy 一致（成功>失败 100:1 比例） | 4 phase counter increments match policy distribution | ota_counter_4phases_distribution.log |
| T15-TR7 | rule | 死锁 Detector Counter 零基线：1000 轮钢琴播放（Task 12）后 → `deadlock_detector_count_total{module="tts_playback"}` == 0；注入 1 次模拟死锁 → count == 1 | count=0 after 1000 AND count=1 after injection 2/2 | deadlock_detector_counter_baseline.log |
| T15-TR8 | rule | 租户配额使用率 Top 10：Grafana 变量 `$tenant_tier=pro` → Dashboard 面板 `tenant_quota_top10` 查询返回 10 行；行值均在 [0, 1.2] 合理范围；无负使用率（反证指标正确性） | rows=10 AND values within [0,1.2] AND negatives=0 | tenant_quota_top10_panel.log |
| T15-TR9 | rule | OTLP 导出协议切换：启动 `--observability-export=otlp` → OTLP/gRPC endpoint 收到 9 指标名 batch（OTel Collector mock）；mock collector 收到时序点 >= 9 个 | batch points >= 9 AND names 9/9 match | otlp_export_batch_collector.log |

### Completion Evidence
_（待实施时填写：9 指标 Prometheus /metrics 完整快照、Grafana JSON 导入预览截图、CSV 300 行完整性 HMAC match 报告、篡改检测行号定位、OTA Counter 四阶段分布柱状图）_

---

## Task 16: 三策略模式（local_first / cloud_fallback / cloud_only）落地

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | medium |
| AC 覆盖 | NFR-1 离线可用；FR-14 与云平台联动（cloud_only 企业纯云）；M2 里程碑策略收尾 |
| Dependencies | Task 2/3（ASR/TTS 本地引擎可用）+ Task 13（云端 SaaS API 可用）+ Task 12（客户端浮窗策略切换开关） |
| Blocked By | Task 13 T13-TR1 通过（4 档 SaaS tier 路由存在） |
| Unblock Condition | Task 13 T13-TR1 与 Task 2 T2-TR5（ASR 三层回退）通过 |

### 核心产物清单
- 新建枚举 `projects/xiaobai_voice/xiaobai_voice/config/deployment_strategy.py`（或 Rust 侧）：
  - `enum DeploymentStrategy { LocalFirst = 0, CloudFallback = 1, CloudOnly = 2 }`
  - `struct StrategyPolicy { asr_priority: [EngineKind; 3], tts_priority: [EngineKind; 3], require_network: bool, allow_cache_days: u32, cloud_endpoint: Url }`
  - 每种 strategy 的 Policy：
    1. **LocalFirst**：ASR [本地 Paraformer / 本地 Sherpa / 云端 API]；TTS [本地 CosyVoice / 本地 Fish / 云端 API]；断网自动降级；缓存云端 TTS 音频 allow_cache_days=30
    2. **CloudFallback**：ASR [云端 API / 本地 Paraformer / 本地 Sherpa]；TTS [云端 Fish Pro / 本地 CosyVoice / 浏览器]；云端优先失败自动切本地；Pro/Team 推荐默认
    3. **CloudOnly**：ASR/TTS 全走云端；本地引擎不加载（减少包体 + 企业管控合规）；require_network=true；断网 toast「企业纯云模式请检查网络」；Enterprise 可选 + 租户级策略强制下发（Admin 锁不可切本地）
- `projects/xiaobai_voice/xiaobai_voice/voice_backend_manager.py`：
  - `VoiceBackendManager(strategy, cloud_creds)` 统一入口
  - 网络探测器（每 10 s ping 云端 /health）→ 策略路由自动切换（CloudFallback：云失败切本地；网络恢复 30 s 后切回云）
  - 健康度打分：云 latency<500ms=100、500~2s=70、>2s=40；错误率连续 3 次=20；切回阈值>=70 分
  - 策略切换 toast：「已切本地离线模式」/「网络恢复已切云端」/「企业纯云模式不可切换」
- 客户端设置页 `frontend/src/components/Settings/Strategy.vue`：三模式单选（仅 Free/Pro/Team 可切；Enterprise Admin 下发锁定后选项置灰 + 提示）
- 云端 SaaS 租户配置：`POST /tenant/settings { "enforce_strategy": "cloud_only", "locked": true }`；客户端拉取后锁死策略
- `projects/xiaobai_voice/tests/test_strategy_three_modes.py`：9+ 单测

### Task-local Test Requirements (TR >= 9)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T16-TR1 | rule | LocalFirst 离线可用：拔网线（无网关）→ LocalFirst 启动成功；ASR/TTS 均使用本地；语音会话 10 轮零网络错误；错误率 0%；会话延迟 P99 与在线基准差值 <= +30%（无云端快） | 10/10 offline ok AND err=0% AND delta_lat <= +30% | strategy_localfirst_offline_10.log |
| T16-TR2 | rule | LocalFirst 断网降级：初始在线 CloudFallback 工作 → 手动 iptables DROP 云 IP → 3 s 内 Manager.health_score 降到 20 以下 → 自动切本地；本地可用 100%；切回 iptables ACCEPT 30 s 后健康度 >=70 → 恢复云端 | failover in <=3s AND failback in <=30s 3/3 cycles | strategy_failover_failback_3cycles.log |
| T16-TR3 | rule | CloudFallback 云端优先：网络好 → 首 3 次 ASR 请求命中云端 API（log line 含 cloud_api）；无本地引擎加载（内存占用比 LocalFirst 少 >= 200 MB，本地模型未 mmap） | cloud 3/3 AND mem diff >= 200 MB | strategy_cloudfallback_priority.log |
| T16-TR4 | rule | CloudOnly 企业纯云：策略=CloudOnly + locked=true → 设置页切换按钮 disabled；本地 Paraformer/CosyVoice import 代码路径永不执行（AST grep 命中加载类次数=0）；断网 5 s 后 toast 企业纯云提示文字 | btn disabled AND load count=0 AND toast shown 3/3 | strategy_cloudonly_enterprise_lock.log |
| T16-TR5 | rule | 云端 TTS 音频缓存（LocalFirst）：云端 TTS 合成同一句话 2 次 → 第 2 次从本地缓存（`allow_cache_days=30`）读取；云端 API 调用次数=1（零重复）；音频字节与原始相等 | api call=1 AND bytes equal 3/3 duplicate texts | strategy_tts_cache_hit_3.log |
| T16-TR6 | rule | 策略切换状态持久化：设置 LocalFirst → 重启桌面浮窗 → 启动后默认策略仍是 LocalFirst（非 reset）；CloudOnly locked=true → 重启后仍然 locked（用户改不了）→ 5/5 持久化场景 | after-restart strategy=previous AND locked preserved 5/5 | strategy_persist_restart_5.log |
| T16-TR7 | rule | 云端 API 鉴权：CloudFallback/CloudOnly → 调用使用 API Key（Task 13）→ 正确 tier 配额消耗（Free 3h）；revoke API Key → 下次请求 401 + 自动切本地（CloudFallback 场景）or toast 拒绝（CloudOnly） | quota consume correct AND revoke→401+fallback_or_toast 2/2 | strategy_cloud_apikey_quota.log |
| T16-TR8 | rule | 三策略 ASR 三层回退全对齐：每种 strategy × ASR engine_failure_scenario 3 种 = 9 组合 → 最终都有结果（无总失败）；回退链长度正确（LocalFirst：本地→本地→云；CloudOnly：云→报错不可切） | 9/9 have result AND chain_length correct 9/9 | strategy_3mode_asr_9matrix.log |
| T16-TR9 | rule | 指标 strategy 标签：`voice_session_count_total{strategy=xxx}` 在 9 模式×会话测试后 → Prometheus 三种 label 各有 counter 值 >0（分别覆盖三种策略）；无未知 strategy="" 空串 | labels "local_first"/"cloud_fallback"/"cloud_only" 3/3 all >0 | strategy_metric_labels_3.log |

### Completion Evidence
_（待实施时填写：LocalFirst 无网 10 轮报告、3 次 failover 切换延迟折线、CloudOnly 按钮置灰 UI 截图、TTS 缓存 1 次 API 调用、策略持久化重启前后对比、9 组合 ASR 回退矩阵、三个 metric label 柱状图）_

---

## Task 17: E2E >= (649 既有 UT + 新增 >= 180 = 829 tests) + Harness 参数化 700 cases

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | NFR-14；M4 里程碑测试充分性；Grade S 验收基础项 |
| Dependencies | Tasks 1~16 所有子任务 TR 代码实现完成（所有测试 target 可被 discover） |
| Blocked By | Task 16 三策略代码、Task 15 取证页代码、Task 14 OTA 差分代码均需实现 |
| Unblock Condition | Task 16 代码合入主干 + Task 15/14/13 代码可被 cargo/pytest 正常 discover |

### 核心产物清单
#### 17.1 E2E >= 829 tests 汇总（既有 649 + 新增 >= 180）
- 既有 UT 盘点（T1-TR8 基准 649 作为 baseline）：
  - `crates/**` Rust UT：407 tests
  - `projects/xiaobai_voice/**` Python UT：92 tests
  - `frontend/**` Vitest：87 tests
  - `platform/**` 服务端 UT：63 tests
  - **小计 Baseline = 407 + 92 + 87 + 63 = 649 tests**（T1-TR8 保证零回归）
- 新增 UT 清单本任务负责新增 >= 180，目标 >= 180：
  - Task 2 ASR：新增 18 tests（对齐 12 TR 扩充边缘场景）
  - Task 3 TTS：新增 20 tests（对齐 14 TR 扩充）
  - Task 4 sensitivity：新增 12 tests（对齐 10 TR + 4 脱敏后缀）
  - Task 5 Reconcile：新增 10 tests（对齐 8 TR + 更多维度组合）
  - Task 6 Suggestion×Constraint：新增 8 tests（对齐 6 TR）
  - Task 7 constants：新增 7 tests（对齐 5 TR + policy.toml 一致性）
  - Task 8 七层：新增 12 tests（对齐 8 TR + 短路分层）
  - Task 9 PPR：新增 10 tests（对齐 8 TR）
  - Task 10 RBAC：新增 11 tests（对齐 8 TR + 跨租户 AssumeRole 深度）
  - Task 11 联盟 8 闸门：新增 12 tests（对齐 8 TR + 三证 7 非法组合深度）
  - Task 12 客户端/PyI：新增 15 tests（对齐 13 TR）
  - Task 13 SaaS：新增 12 tests（对齐 10 TR）
  - Task 14 OTA：新增 12 tests（对齐 10 TR）
  - Task 15 可观测：新增 10 tests（对齐 9 TR）
  - Task 16 三策略：新增 11 tests（对齐 9 TR）
  - **合计 18+20+12+10+8+7+12+10+11+12+15+12+12+10+11 = 180 tests**
- `ops/e2e-runner/run-all.sh` + PowerShell.ps1：跨平台一次性 runner；输出 `e2e-report-YYYYMMDD.json`（T11 类似字段：pass/fail/skips、按 crate 分组、耗时 Top 20）

#### 17.2 Harness 参数化 700 cases
- 新建 `crates/mox-expert/src/harness/parametric.rs` + Python `projects/xiaobai_voice/tests/harness_parametric.py` 双实现：
  - 参数维度笛卡尔：
    - DeploymentStrategy(3: LFirst / CFall / COnly) ×
    - MembershipTier(4: Free/Pro/Team/Ent) ×
    - IntentCategory(7: Voice/Code/Query/Comp/Opt/Audit/Unknown) ×
    - VoiceSessionScenarios(1, 2, 3: only for Voice intent) =
    - 前三项 3×4×7=84 基础 + Voice 额外 3×4×1×3（scenario3）= 36 = 120 参数 cell
    - 每 cell 至少执行 Stages = ~5-6 阶段 Harness 生命周期（Init/Pre/Run/Verify/Govern/Report）
  - 扩展 `Strategy×Tier` 与 `Tier×RBAC role` 交叉：额外 3×4×4(admin/editor/viewer/auditor)=48；与上 120 再 × 流程 5 阶段 ≈ 84×5 = 420 + Voice 深度 280 合计 704 >= 700
- Harness 插件：PreGate（资源准备）+ PostGate（清理 + 审计链完整性）
- `crates/mox-expert/tests/harness_parametric_700.rs`：参数化宏 700 cases 断言入口

#### 17.3 报告产出
- `reports/e2e-report-<sha>.json`：Total=829±，fail<=2，pass rate >= 99.76%
- `reports/harness-704-matrix.csv`：120 cells × 6 stages；heatmap PNG（失败用红）

### Task-local Test Requirements (TR >= 12)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T17-TR1 | rule | 既有 649 UT 零回归：Baseline 649 全跑（Rust cargo test --workspace / Python pytest / Vitest / 服务端）→ fail 0；skip <= 3（平台特有 skip 不计 fail） | 649/649 pass AND fail=0 skip<=3 | e2e_baseline_649.log |
| T17-TR2 | rule | 新增 >= 180 UT 全绿：15 Task 新增项 180 tests → pass rate >= 99% (允许 1 fail)；但 fail 必须是 infra flaky（非产品逻辑 bug）并附 issue 链接；逻辑 bug 零容忍 (逻辑 fail=0) | pass >= 179 AND logical_fail=0 | e2e_new_tests_180.log |
| T17-TR3 | rule | 汇总 Total >= 829：649 baseline + 180 new = 829 <= actual；actual 统计 JSON 中 total_tests 字段 >= 829 | total >= 829 | e2e-report-*.json field |
| T17-TR4 | rule | Harness 参数化 >= 700：parametric matrix 展开 cases 数 = 704 >= 700；所有 cell 无 panic（即便不 pass 也给出结构化 Failure 对象）；panic=0 是硬要求 | cases=704 AND panic=0 | harness_parametric_704.log |
| T17-TR5 | rule | Harness 4 关键通过率：LFirst/Free/Voice、CFall/Pro/Code、COnly/Ent/Comp、Team/Query 四组典型 → pass >= 95%（stages pass rate） | 4/4 groups rate >= 95% 每组 | harness_key4_groups_passrate.log |
| T17-TR6 | rule | E2E 覆盖率要求：代码覆盖率（cargo tarpaulin + Python coverage）Rust 包 mox-expert 行覆盖 >= 80%；xiaobai_voice Python 包行覆盖 >= 75%；C0 覆盖率不接受 <70% 任何包 | rust>=80% AND python>=75% AND min>=70% | coverage_report_tarpaulin_html.log |
| T17-TR7 | rule | Playwright 端到端语音对话 30 轮循环（T16 基准扩充）：录音→识别→发送→LLM→朗读 30 轮全闭环；成功率 100%；无对话窗无响应死等（单轮 timeout <= 60 s） | 30/30 success AND no timeout 30/30 | pw_voice_e2e_30runs_trace.html |
| T17-TR8 | rule | P1-P4 缺陷 E2E 集成回归：4 缺陷各自经典复现场景 5 个版本 × 4 = 20 E2E → 全部通过（不复发）；Task 4-7 单测基础上更靠近用户端场景（HTTP 调用级 / 前端 UI 级） | 20/20 no regression | defect_p1p4_e2e_20.log |
| T17-TR9 | rule | audio_play 死锁 24 h 浸泡：_PlaySession 100K 轮 play/stop/pause 随机切换；死锁检测计数 = 0；浸泡期间 session 状态机无 illegal state 转移（FSM guard 断言） | 100K/100K no deadlock detector=0 AND illegal transfer=0 | playback_soak_100k_rounds.log |
| T17-TR10 | rule | 并发压力 E2E：50 并发会话 × 策略 3 模式 × tier 4 档 子集 = 100 并发组合；持续 10 min；错误率 <= 0.5%；P99 <= 5 s（放宽端到端） | err<=0.5% AND P99<=5s | e2e_concurrent_100x10min.csv |
| T17-TR11 | rule | 信创 4 平台兼容回归：M4 信创机器（麒麟 V10 鲲鹏 920 / 统信 UOS 飞腾 2000+ / Windows Server 信创版 / 中标麒麟）× smoke 套件各 50 tests → 通过 >= 45 每平台（允许 <=5 个已知 skip，无 fail）；4/4 平台达标 | per platform pass>=45 AND fail=0 4/4 | xincompat_4platforms_200tests.log |
| T17-TR12 | rule | 失败可复现性：Harness 704 cases 中任取失败 case（制造 1 个 mock 失败）→ 记录 seed + 参数向量；相同 seed 重跑失败路径完全一致（hash 相同）；用于 CI rerun 稳定复现 | deterministic replay hash= 2/2 mock failures | harness_seed_reproducibility.log |

### Completion Evidence
_（待实施时填写：649 baseline + 180 new = 829 汇总 JSON、Harness 704 cases 矩阵热力图、覆盖率 HTML 报告、30 轮 Playwright trace、24 h 浸泡 100K 零死锁、信创 4 平台 200 tests 截图、确定性重放 hash 对比）_

---

## Task 18: Rubric 汇总 + Grade S 验收评审（加权综合 >= 90）

| 字段 | 值 |
|---|---|
| Status | pending |
| Priority | high |
| AC 覆盖 | U1~U8 全部 8 Rubric；Grade S 最终通过；M4 里程碑验收交付 |
| Dependencies | Tasks 1~17 completion evidence 全部齐；Task 17 E2E >= 829 + Harness 700 达标 |
| Blocked By | Task 17 T17-TR3（total >= 829）+ T17-TR4（704 harness）通过 |
| Unblock Condition | Task 17 T17-TR3 + T17-TR4 + T17-TR9（死锁浸泡）通过 |

### Rubric 加权表（Grade S 权重，总计 100%）
| Rubric 项目 | 权重 | 满分 | 对应来源（Task/TR 证据） |
|---|---|---|---|
| **U1 语音引擎工程质量** | 15% | 100 | Task 2（ASR 12 TR 全绿 + CER<=5%）+ Task 3（TTS 14 TR 全绿 + MOS>=4.0）+ Task 12 死锁 1000→100K 轮零复发 |
| **U2 璇玑架构正确性** | 20% | 100 | Task 4-7（P1-P4 缺陷全根治 + TR 通过率 100%）+ Task 8（七层零断点 + 四条不变式）+ Task 11（G1~G8 八道闸门 100% 覆盖） |
| **U3 桌面客户端体验** | 12% | 100 | Task 12（桌面 13 TR 全绿 + PyInstaller 零闪退 + 4 态交互流畅 + 拖拽吸附零偏差） |
| **U4 企业级合规治理** | 20% | 100 | Task 4（PII 脱敏流转链）+ Task 10（RBAC + 越权拒绝 100%）+ Task 11（三证出码）+ Task 15（取证 CSV HMAC 防篡改） |
| **U5 SaaS 平台与 OTA** | 10% | 100 | Task 13（多租户 10 TR 全绿 + 4 档配额矩阵 24/24）+ Task 14（OTA 四阶段推进 + 坏包回删 + 签名防篡改） |
| **U6 可观测与取证** | 8% | 100 | Task 15（9 指标 100% + Grafana 面板合法 + CSV 导出完整性 + 篡改检测） |
| **U7 策略模式灵活性** | 8% | 100 | Task 16（三策略 9 TR 全绿 + 断网降级 + 企业纯云锁定 + 缓存命中） |
| **U8 测试充分性 Grade S** | 7% | 100 | Task 17（E2E>=829 + Harness>=700 + 覆盖率 Rust>=80/Py>=75 + 信创 4/4 兼容） |
| **合计** | **100%** | - | **加权综合 = Σ(Ui_score × Ui_weight / 100) ≥ 90 → Grade S** |

### 单项 Rubric 详细打分细则
| Rubric | 评分范围 | 打分关键锚点 |
|---|---|---|
| U1 语音 | 0-100 | 基础 60：TR 通过 >= 10/12+12/14；+15 CER<5% MOS>4.0；+15 死锁浸泡 24 h 零；+10 三层回退 100% 覆盖率冒烟 50 次 |
| U2 架构 | 0-100 | 基础 60：P1-P4 TR 全过；+20 七层零断点+四不变式形式化证明（至少单测级）；+10 八闸门全 100%；+10 P1 假阳性 10K 随机资源无漏判/误判 |
| U3 客户端 | 0-100 | 基础 60：13 TR >=11 通过；+20 PyInstaller 5 台用户机双击 0 闪退；+10 Alt+X 24 h 全局无热键冲突；+10 拖拽 100 次吸附零偏差 |
| U4 合规 | 0-100 | 基础 60：四任务 TR 全通过；+20 越权 100 次测试 0 泄漏（红蓝对抗）；+10 审计链 100K blocks HMAC 完整性 100%；+10 CSV 篡改 50 次全部 100% 检测 |
| U5 SaaS/OTA | 0-100 | 基础 60：SaaS 10 TR + OTA 10 TR 全过；+20 多租户 100 租户 24 h 无跨租户泄漏；+10 OTA 4 阶段推进 3 次零事故；+10 差分包 50 版本 patch 成功率 100% |
| U6 观测 | 0-100 | 基础 60：9 TR 通过；+20 Grafana 接入 30 min 内可出 P99 曲线；+10 取证 1 亿 events CSV 导出 < 60 s；+10 OTLP 全链路 trace 完整 |
| U7 策略 | 0-100 | 基础 60：9 TR 通过；+20 Failover 1000 次切换 0 会话中断；+10 CloudOnly 锁定 20 用户 100% 不可绕过；+10 缓存命中率 > 50% （同句重复） |
| U8 测试 | 0-100 | 基础 60：829 + 700 通过；+20 覆盖率达标（Rust>=80/Py>=75）；+10 信创 4/4；+10 确定性重放 100% 一致 |

### Task-local Test Requirements (TR >= 8，Rubric 汇总验收 8 项)
| TR | 类型 | 内容 | 通过阈值 | 证据 |
|---|---|---|---|---|
| T18-TR1 | rubric | U1 语音工程质量评分 ≥ 92：CER≤5%、MOS≥4.0、死锁 100K 轮零、ASR/TTS 三层回退各 50 场景覆盖；权重 15% → 加权贡献 ≥ (92×0.15)=13.8 | U1_score >= 92 AND weighted_contrib >= 13.8 | rubric_u1_scorecard.json |
| T18-TR2 | rubric | U2 璇玑架构正确性评分 ≥ 92：P1-P4 缺陷根治 TR 100%、四条不变式单测覆盖、八闸门 100%；权重 20% → 加权贡献 ≥ 18.4 | U2_score >= 92 AND weighted_contrib >= 18.4 | rubric_u2_scorecard.json |
| T18-TR3 | rubric | U3 桌面客户端体验评分 ≥ 90：PyInstaller 5 台用户机双击零闪退（0/5 闪退）、拖拽吸附 100 次≤2px；权重 12% → 加权贡献 ≥ 10.8 | U3_score >= 90 AND weighted_contrib >= 10.8 AND 0_crash_on_5pcs | rubric_u3_scorecard.json |
| T18-TR4 | rubric | U4 企业级合规治理评分 ≥ 94：红蓝对抗越权 100 次零泄漏、审计链 100K blocks HMAC 完整性 100%；权重 20% → 加权贡献 ≥ 18.8 | U4_score >= 94 AND weighted_contrib >= 18.8 AND redteam_0leak_100 | rubric_u4_scorecard.json |
| T18-TR5 | rubric | U5 SaaS+OTA 评分 ≥ 88：24 h 多租户零泄漏、OTA 4 阶段推进 3 次零事故；权重 10% → 加权贡献 ≥ 8.8 | U5_score >= 88 AND weighted_contrib >= 8.8 AND ota_3runs_0incident | rubric_u5_scorecard.json |
| T18-TR6 | rubric | U6 可观测与取证评分 ≥ 88：Grafana 9 指标全通、CSV 篡改检测 100%；权重 8% → 加权贡献 ≥ 7.04 | U6_score >= 88 AND weighted_contrib >= 7.04 AND tamper_detect_50=100% | rubric_u6_scorecard.json |
| T18-TR7 | rubric | U7 三策略评分 ≥ 86：Failover 1000 次会话零中断、CloudOnly 锁定不可绕过 100%；权重 8% → 加权贡献 ≥ 6.88 | U7_score >= 86 AND weighted_contrib >= 6.88 AND failover_1000_zero_drop | rubric_u7_scorecard.json |
| T18-TR8 | rubric | U8 测试充分性评分 ≥ 90 + **加权汇总 Grade S ≥ 90**：E2E≥829、Harness≥700、信创 4/4 全过；Σ(Ui × weight_i) = 最终得分 **≥ 90** → Grade S 达成 | U8_score >= 90 AND **TOTAL_WEIGHTED_SCORE >= 90 (Grade S)** | rubric_u8_scorecard.json + **grade-s-final-report.pdf** |

### Grade S 验收签字页占位
```
验收项目：小白语音服务 + 璇玑 MOX 架构mox 模块化系统架构落地
验收日期：2027 年 __ 月 __ 日
加权综合得分：____ / 100 （Grade S ≥ 90）
验收结论：□ Grade S 通过  □ Grade A 未达标（需回滚至 M4 整改）
签字：
  产品负责人：_________  日期：______
  架构负责人：_________  日期：______（U2/U4/U8 审核）
  语音引擎负责人：_______  日期：______（U1 审核）
  客户端 & 打包负责人：______  日期：______（U3 审核）
  云平台 & OPS 负责人：______  日期：______（U5/U6 审核）
  质量 & 信创负责人：______  日期：______（U7/U8 + 信创 4 平台 审核）
```

### Completion Evidence
_（待实施时填写：rubric-u1~u8 8 份 scorecard JSON、grade-s-final-report.pdf（含完整 ΣUi×weight 计算）、验收签字页扫描件、红蓝对抗 100 次零泄漏报告、OTA 4 阶段 3 次零事故报告、Failover 1000 次零中断报告）_

---

## 文档尾注
- 本文档对齐 SPEC：`FR-1~FR-15` 与 `NFR-1~NFR-15`（本文件 §AC 概述 30 rule 映射）
- P1-P4 缺陷根治：Task 4/5/6/7 分别对应，且 Task 17 E2E 含集成回归（T17-TR8）
- FR-12 audio_play 死锁：Task 3 T3-TR14 单测、Task 12 T12-TR7/T12-TR11 静态 + 1000 轮、Task 17 T17-TR9 24 h 浸泡 100K 轮三级验证
- FR-13 PyInstaller stderr 兜底：Task 12 T12-TR6 _ensure_windowed_streams
- 四里程碑：M1(09-10) / M2(11-12) / M3(01-02) / M4(03-04) → M4 出口 Grade S 验收（Task 18）
