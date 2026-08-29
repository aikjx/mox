# AIS 专家联盟裁决流水线 × FR-13/FR-5 对接设计规范 V1.0

> 文档版本：V1.0 （企业级规格，对齐 20260826-xiaobai-mox-full-arch 的 spec.md）
> 对接范围：FR-13 voice_proxy 桥 + 8 大类 system-operator；FR-5 ASR 热词注入
> 对应工期：最小交付路径 = FR-13（voice_proxy + 4 类核心算子）+ FR-5 = 7–9 人日
> 文档落地：本 Markdown + 代码模块 1:1 映射，见底部"文件对应表"

---

## 0. 适用角色

| 角色 | 接入级别 | 权限范围 |
| ---- | -------- | -------- |
| Auditor (L0) | 仅读 | 查看所有非破坏性动作、审计记录 |
| Member (L1)  | 非破坏性写 | 开应用/调音量/打开文件 |
| Expert / Coordinator (L2) | 高权限写 | 键鼠输入/剪贴板写入/鼠标移动 |
| MoxAdmin (L3) | 破坏性写 | 关应用/删除文件/截图/拖拽 |

任何"意图→动作"的实际执行都必须先通过 RBAC Engine 的 `dispatch(..., identity=X)` 权限校验；
cloud_only 模式下在此之上再叠加专家联盟事前裁决（双重闸门）。

---

## 1. 专家联盟裁决流水线总览（S1~S6 + G1~G8）

```
用户语音"打开记事本并设置音量为 60"
    │
    ▼
  ┌──────────────────────────────────────────────────────────┐
  │  S1 意图抽取 (PPR 激活扩散 + LLM Rewrite)               │
  │  · 入参：ASR text + hotwords + 上下文窗口                │
  │  · 出参：List[IntentSlot]  (op, act, params, conf)       │
  │  · G1 闸门：INTENT_UNKNOWN 时必须 3 路专家 ≥ 2 路一致     │
  └───────────────────┬──────────────────────────────────────┘
                      ▼
  ┌──────────────────────────────────────────────────────────┐
  │  S2 组队 (Domain-Expert Router)                          │
  │  · 按意图的 domain 选 3 个专家：                          │
  │    - 桌面域专家（app/input/volume/file）                 │
  │    - 安全域专家（sensitivity.rb + RBAC 权限检查）         │
  │    - 质量域专家（reconcile: 冲突/可并行/强制串行）       │
  │  · G2 闸门：组队中任一路未就绪 → 降级为 Coordinatror 单人 │
  └───────────────────┬──────────────────────────────────────┘
                      ▼
  ┌──────────────────────────────────────────────────────────┐
  │  S3 咨询辩论 (Debate 3±1 轮)                             │
  │  · 每路专家给出 {permit, deny, reason, op, act, params}  │
  │  · Coordinator 汇总 + 反方质疑 → 至少 2 轮反馈收敛        │
  │  · G3 闸门：L3 动作必须 3/3 全 permit；L2 至少 2/3       │
  └───────────────────┬──────────────────────────────────────┘
                      ▼
  ┌──────────────────────────────────────────────────────────┐
  │  S4 合成裁决 (Synthesize)                                │
  │  · 合成为最终执行计划：[{op, act, params, must_serialize, │
  │                         suggested_parallel, pre_cond}]    │
  │  · G4 闸门：存在 MustSerialize ⋀ Parallelize 语义冲突？  │
  │    → 交由 flow-ai 求解器 + 质量域专家裁决                 │
  └───────────────────┬──────────────────────────────────────┘
                      ▼
  ┌──────────────────────────────────────────────────────────┐
  │  S5 执行门禁 (EnforcerGate)                              │
  │  · 对应 voice_proxy 的 alliance_gate(op,act,params,id)   │
  │  · cloud_only ：否决立即返回 PERMISSION_DENIED            │
  │  · cloud_fallback ：L1 及以下仍允许本地放行               │
  │  · local_first   ：异步记录，不阻塞执行                   │
  │  · G5 闸门：PII 敏感资源命中(用户主目录/凭证文件等)       │
  │    → 必须强制升级到 L3 再审查；否则直接拦截               │
  └───────────────────┬──────────────────────────────────────┘
                      ▼
  ┌──────────────────────────────────────────────────────────┐
  │  S6 持续学习 (FeedbackLoop)                              │
  │  · 收集执行成功/失败/用户手动覆盖/Toast 反馈              │
  │  · 按周更新 PPR 图谱权重 + hotwords score + alias 映射    │
  │  · G6-G8 闸门：漂移率 ≤ T13=0；降级率 ≤ 5%；召回率 ≥ 95% │
  └──────────────────────────────────────────────────────────┘
```

---

## 2. FR-13 voice_proxy 桥与联盟裁决的接口契约

### 2.1 消息信封（JSON snake_case，全部走 WebSocket 主通道）

```jsonc
// VoiceProxyClient ⇒ mox-expert （请求裁决）
{
  "type": "intent",
  "id": "hex-uuid",
  "payload": {
    "text": "打开记事本并把音量调到 60",
    "identity": {"user_id": "u1", "role": "Member", "tenant_id": "t1"},
    "local_route": {                         // 来自 IntentRouter.route().as_dict()
      "op": "app", "act": "open_app",
      "confidence": 0.9, "ambiguous": false,
      "candidates": [ {"op":"volume","act":"set_volume","confidence":0.78,"params":{"value":60}} ]
    },
    "ctx": {
      "asr": {"backend": "sherpa_paraformer", "hotwords_hint": true,
              "hotwords_applied": ["桌面悬浮球"]}
    }
  }
}

// mox-expert ⇒ VoiceProxyClient （裁决响应）
{
  "type": "ack", "reply_to": "hex-uuid", "id": "...",
  "code": "OK",                                      // OK / INTENT_UNKNOWN / PERMISSION_DENIED / INTENT_AMBIGUOUS
  "message": "多动作串行规划：先开记事本后调音量",
  "payload": {
    "op": "app",                                      // 首选第一个动作；后续 EXEC 连续调用
    "act": "open_app",
    "params": {"target": "notepad"},
    "mode": "local",                                  // local | remote（mox 执行远程算子）
    "timeout_ms": 5000,
    "next_ops": [                                     // 后续队列（桌面端循环执行）
      {"op": "volume", "act": "set_volume", "params": {"value": 60}}
    ],
    "audit_nonce": "nonce-xyz",                       // AUDIT 回传时必须带上
    "alliance_reasons": {
      "desktop_expert": "命中意图 0.99 > 0.55",
      "security_expert": "L1.write 不触发 PII (开 calc.exe 不是敏感)",
      "quality_expert": "两个动作串行，无语义冲突"
    }
  }
}
```

### 2.2 审计上报（VoiceProxyClient _engine_audit_cb_sync → AUDIT）

```jsonc
{
  "type": "audit",
  "id": "...",
  "payload": {
    "op": "app", "act": "open_app",
    "params": {"target": "notepad"},
    "identity": {"user_id": "u1", "role": "Member", "tenant_id": "t1"},
    "result": {"ok":true, "code":"OK", "data": {"method":"startfile", "pid": 1234},
               "duration_ms": 312, "audit_id":"aud_123abc"},
    "ts_ms": 1756176000000
  }
}
```

mox-expert 收到后入库到 `audit_trails` 表（对齐 mox-system RBAC 的审计日志字段），
**与 S6 的 FeedbackLoop 一起每周做 PPR 图谱权重调优和 FR-5 hotwords 评分更新。**

---

## 3. FR-13 system-operator 的专家联盟钩子点

OperatorEngine 两个回调（`audit_cb` + `alliance_gate`）都是可插拔的，最小交付路径下：
- **local_first**（个人用户默认）：不连 mox，`alliance_gate` 为 None，所有指令本地直接执行，
  `audit_cb` 写本地 SQLite `xiaobai_audit.db`，UI 端合规面板可回放。
- **cloud_fallback**（企业推荐）：`dispatch_intent` 先问联盟 800ms，超时仍本地执行；
  本地 OPERATOR_UNSUPPORTED（如 Mac 上 pycaw 缺失）才走 mox 远程算子。
- **cloud_only**（合规严格模式）：所有 L2/L3 动作都等联盟最终裁决；桥断直接 BRIDGE_DISCONNECTED，
  L0/L1 可读仍允许。

### 3.1 语义冲突矩阵（reconcile 对接）

`Platform/services/mox-expert/src/reconcile.rs` 已实现的 P2/P3 冲突与算子的对应：

| 冲突类型 | 系统算子触发场景 | 联盟处理方式 |
| -------- | ---------------- | ------------ |
| Parallelize ⋀ MustSerialize （语义冲突 P3） | "同时关闭 Chrome 和打开系统设置删除凭证" | 强制串行 + 安全域加一票否决 |
| 同优先级资源冲突（P2） | "复制文件 A 和移动文件 A" | 按先后依赖自动定序，或交 flow-ai 求解 |
| 维度冲突（Resource ⋀ Algorithm） | "设置音量 30 + 再设 80" | 合成为最后一次；标注 conflict.semantic |

### 3.2 PII 敏感资源（sensitivity.rb 唯一权威模块对接 file_operator）

- file_operator **任何涉及路径读写/删除/读文本**时，在动作入口先调用 `is_sensitive_leak(path)`。
  命中（`TRUE`）就立即：
  1. 把 L1/L2 身份提升为 **需要 L3 + 联盟安全专家投票**；
  2. 若 strategy 不是 cloud_only：`data["warnings"].append("PII命中，已记录审计")`。

**对接路径**（代码里 TODO 占位，S2 才补）：

```python
# xiaobai_voice/operator/file_operator.py 调用入口
# from mox_expert_client import is_sensitive_leak  # mox 提供的 gRPC/HTTP 客户端
# if is_sensitive_leak(path):
#     params["__required_level_override"] = AccessLevel.L3_ADMIN
#     params["__alliance_gate_force"] = True
```

---

## 4. FR-5 ASR 热词与联盟裁决的联动接口

### 4.1 联盟 → ASR：每周下发 hotwords 文件

mox-expert S6 每周产出 `hotwords_tenant_v2.jsonl`：

```jsonl
{"word": "桌面小白助手", "score": 9.2, "source": "S6_feedback_boost", "ttl_days": 30}
{"word": "Paraformer-zh", "score": 7.0, "source": "OSS_defaults", "ttl_days": -1}
{"word": "企业微信客户联系", "score": 8.5, "source": "tenant_dict", "ttl_days": 365}
```

- VoiceProxyClient 订阅 `type: "config.push"` 消息 → 回调 `asr_backend.set_hotwords(new_list)`。
- `set_hotwords` 内部：format 校验 → 重建 recognizer → S1/S2 真实注入 context_config。
- 若重建失败（HOTWORDS_REINSTANTIATE_FAIL），**S3 post-hoc 依然生效**，保证业务不中断。

### 4.2 ASR → 联盟：上报 hotwords 命中结果

recognize_stream 的 final 与 recognize_full 的输出都携带：
- `hotwords_applied[]`（S3 post-hoc 已替换的热词清单）
- `hotwords_raw`（原始 ASR 文本，便于联盟评估替换增益/误差）

联盟 S6 每周分析：
- **召回率** = `len(ground_truth ∩ applied) / len(ground_truth)`，目标 ≥ 95%
- **精确率** = 无人工反馈"热词替换错误"率，目标 ≥ 99%
- 低于阈值时：自动下调对应热词的 score（或放大编辑距离阈值）

---

## 5. 文件-规范对应表（实现位置与状态）

| 模块 / 文件 | 对应规范章节 | 实现状态 | 关键类型 / API |
| ----------- | ------------ | -------- | -------------- |
| `operator/base.py` | 2.2 RBAC 4 级 + 3.1 Engine 回调 | ✅ | `require_level`, `OperatorEngine.dispatch`, `AuditCallback`, `AllianceGateCallback` |
| `operator/app_operator.py` | 3.2 app 算子 L1/L3 | ✅ | `open_app`, `close_app`, `list_running`, `open_file_with_app` |
| `operator/file_operator.py` | 3.2 file 算子 L0-L3 + PII 钩子 | ✅ | `copy_to_clipboard` (pyperclip/ctypes Win), `move_to_trash`（send2trash + L3 permanent_delete 二次确认） |
| `operator/volume_operator.py` | 3.2 volume L0-L1 三平台 | ✅ | Windows=pycaw+waveOut, macOS=osascript, Linux=pactl/amixer |
| `operator/input_operator.py` | 3.2 input L2/L3 键鼠/截图 | ✅ | pynput + Win32 回退，mss+Pillow 截图，Levenshtein free |
| `intent/router.py` | S1 PPR 工程化 + App 别名 | ✅ | 40+ 规则，`_APP_ALIAS` 40+ 项映射，`ambiguous_threshold` 分流联盟裁决 |
| `proxy/voice_proxy.py` | 2.x 信封协议 + 三策略 | ✅ | `VoiceProxyClient.dispatch_intent`, httpx/ws 双 Transport, `_remote_intent_flow`, `_engine_audit_cb_sync` |
| `asr/sherpa_paraformer.py` | FR-5 S1/S2/S3 三层热词 | ✅ | `set_hotwords` (format OK/rebuild), `_build_context_config` (inspect+探字段), `_post_hoc_fixup` (exact+fuzzy) |
| `desktop/ball_widget.py` | UI 5 状态 + _ExecWorker | ✅ | `set_operator_engine`, `execute_text`, `_ExecWorker` QThread, 三彩虹弧 executing 动画 |
| `tests/selftest.py` 7–10 项 | FR-13/FR-5 回归矩阵 | ✅ | `fr13_intent_router_smoke`, `fr13_rbac_4level_auth_matrix`, `fr13_four_ops_smoke`, `fr5_hotwords_inject_and_posthoc` |
| `platform/services/mox-expert/src/sensitivity.rs` | 3.2 PII 钩子 | ✅（上次修复，等待 file_operator 接入 HTTP/gRPC 客户端） | `is_sensitive_leak` 唯一权威 |
| `platform/services/mox-expert/src/reconcile.rs` | 3.1 语义冲突（P2/P3） | ✅（上次修复） | `ReconcileConflict::semantic` + `conflicts` Vec 不再永久空 |
| `platform/services/mox-expert/src/constants.rs` | 全局常量归一化（P4） | ✅（上次修复） | `SENSITIVE_PREFIXES`, `DEDUCT_WEIGHTS`, `DIM_PRIORITIES` |
| `platform/services/mox-system/src/rbac.rs` | 2.0 权限模型继承关系 | ✅（框架基础） | Role → Permission 映射，后续对齐 Identity.from_role |

---

## 6. 最小交付路径如何运行（端到端验收命令）

```powershell
# 1) 启动 voice_service（端口 3717，默认 local_first，无 mox 也能工作）
cd d:\a10\aikjx\gitcode\infotopograph\projects\xiaobai_voice
python -m xiaobai_voice.service.main --strategy local_first

# 2) 启动桌面端（同一主机；BallWidget.set_operator_engine 由 main_window.py bootstrap 注入）
python -m xiaobai_voice.desktop.main
# 期望现象：
#  - Alt+X 录音 → "打开记事本" → Ball 绿→紫→橙（executing）→青（idle），
#    Toast 显示 "🧭 路由：app.open_app conf=88%" → "✅ 已启动应用"，记事本真的打开。
#  - "把音量调到 50" → 橙动画 → "✅ 已设置音量"，系统音量变 50。
#  - "复制今天完成了到剪贴板"：L2 动作，身份默认 Member 会报 🚫 权限不足。
#    切换角色 (set_operator_engine(identity=Identity(role="Coordinator"))) 后重说一次 → 成功。

# 3) 跑回归（关键 selftest）
python -m xiaobai_voice.tests.selftest
# 期望：fr13_* × 3 + fr5_hotwords_* × 1 全 OK（返回码 0）
```

---

## 7. 已知缺口与 P2/P3 开发顺序（与最小交付路径 7-9 人日衔接）

| 类别 | 条目 | 工作量 | 前置 |
| ---- | ---- | ------ | ---- |
| P2 | mox-expert gRPC/HTTP 客户端 ⇄ file_operator PII 钩子接入 | 1.5 人日 | mox-expert 对外暴露 /sensitivity/v1/leak 端点 |
| P2 | 5-8 剩下的 4 大类算子（network/display/browser/notify） | 2 人日 | RBAC 设计复用；三平台实现需单独调研 |
| P2 | mox-system 原生 Role ⇄ Identity 字段精确映射 + 所有权作用域 | 1 人日 | mox-system 开放 /rbac/v1/whoami 端点 |
| P3 | VoiceProxyServer 端：mox 反调桌面远程协助（截图/键鼠） | 2 人日 | cloud_only 部署策略全面切换 |
| P3 | S6 学习：热词 score + PPR 图谱权重每周增量更新 | 1.5 人日 | audit_trails 至少 2 周数据量 |
