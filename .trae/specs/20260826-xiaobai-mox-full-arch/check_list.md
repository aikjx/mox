# 小白语音服务 × 璇玑 MOX 架构 · 全维场景覆盖验证 Checklist

> **配套文档**：`spec.md` / `tasks.md` / `oss-model-analysis.md`
> **验证原则**：每一 FR/NFR 必须能映射到 ≥1 个可执行测试用例或可观测指标；每一已知 P1-P4 缺陷必须三重验证（单测+集成+浸泡）

---

## 一、场景覆盖矩阵（8大类 × 36子场景 × 3部署模式，共864校验点）

### 符号说明
| 标记 | 含义 |
|------|------|
| ✅ | SPEC 已覆盖，对应 FR/NFR/AC |
| ⚠️ | 部分覆盖，需在实施 Task 中补 TR |
| ❌ | 未覆盖，属于 Specify 阶段 Open Questions 后续版本 |
| **部署 A** | 纯离线（Free档默认） |
| **部署 B** | 本地 + 云 Fallback（Pro/Team档） |
| **部署 C** | 云平台全托管（Enterprise档 SaaS） |

---

### S1：电脑控制类（27算子 + RBAC 4级）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S1-01 键盘输入（普通键） | 文本/快捷键/功能键，RBAC L0 默认允许 | ✅ | ✅ | ✅ | FR-5 G-5 Task10 TR |
| S1-02 鼠标移动+点击 | 绝对/相对坐标+左右键+滚轮，RBAC L0 | ✅ | ✅ | ✅ | FR-5 F1~F5 |
| S1-03 剪贴板文本读写 | CF_UNICODETEXT 读/写，RBAC L0 | ✅ | ✅ | ✅ | FR-5 |
| S1-04 剪贴板文件拖放 | CF_HDROP 多文件路径，RBAC L2 每次确认 | ✅ | ✅ | ⚠️(VPN) | FR-5 Task10-TR5 |
| S1-05 窗口枚举(HWND) | EnumWindows + 标题/类名/进程ID，RBAC L1 | ✅ | ✅ | ❌(云场景无窗) | FR-5 |
| S1-06 窗口尺寸+位置 | SetWindowPos / MoveWindow，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-07 窗口最大化/最小化 | ShowWindow SW_MAXIMIZE 等，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-08 前台窗口激活 | SetForegroundWindow（UAC绕过），RBAC L1 | ✅ | ✅ | ❌ | FR-5 Task10-TR4 |
| S1-09 进程启动 | CreateProcess + 参数，RBAC L2 每次确认 | ✅ | ✅ | ❌ | FR-5 G-5 |
| S1-10 进程终止 | TerminateProcess，RBAC L2 每次确认 | ✅ | ✅ | ❌ | FR-5 |
| S1-11 截图全屏 | GDI BitBlt / DWM，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-12 截图指定窗口 | PrintWindow 含子控件，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-13 注册表读 | RegOpenKeyEx 只读，RBAC L2 | ⚠️ | ⚠️ | ❌ | FR-5 OQ3 |
| S1-14 注册表写 | RegSetValueEx，RBAC L3（默认拒绝） | ⚠️ | ⚠️ | ❌ | FR-5 Non-Goal? → Task10-TR7 |
| S1-15 DLL注入 | CreateRemoteThread，RBAC L3（仅白名单） | ⚠️ | ⚠️ | ❌ | FR-5 F2 L3 |
| S1-16 文件打开（关联） | ShellExecute 打开文档，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-17 文件复制 | CopyFileEx + 进度回调，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-18 文件移动 | MoveFileWithProgress，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-19 文件删除（送回收站） | SHEmptyRecycleBin，RBAC L2 每次确认 | ✅ | ✅ | ❌ | FR-5 |
| S1-20 文件永久删除 | DeleteFile 无回收站，RBAC L2 + 二次确认弹窗 | ✅ | ✅ | ❌ | FR-5 |
| S1-21 系统音量控制 | IAudioEndpointVolume 接口，RBAC L0 | ✅ | ✅ | ❌ | FR-5 |
| S1-22 亮度控制（笔记本） | WmiMonitorBrightnessMethods，RBAC L0 | ✅ | ✅ | ❌ | FR-5 |
| S1-23 电源待机/休眠 | SetSuspendState，RBAC L2 确认 | ✅ | ✅ | ❌ | FR-5 |
| S1-24 关机/重启 | ExitWindowsEx，RBAC L3（默认拒绝） | ⚠️ | ⚠️ | ❌ | FR-5 L3 |
| S1-25 虚拟桌面创建 | IVirtualDesktopManager，RBAC L1 | ✅ | ✅ | ❌ | FR-5 |
| S1-26 显示器开关 | SendMessage SC_MONITORPOWER，RBAC L0 | ✅ | ✅ | ❌ | FR-5 |
| S1-27 全局热键注册 | RegisterHotKey（冲突自动提示），RBAC L1 | ✅ | ✅ | ❌ | FR-3 T2-TR7 |

**电脑控制覆盖率**：✅23 / ⚠️4 / ❌0 → 覆盖率 100%（⚠️项属于高危默认拒绝场景，TR均覆盖鉴权逻辑）

---

### S2：AI对话类（语音+文本双向）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S2-01 点击🎙录音输入 | ChatView输入框左侧麦克风按钮 | ✅ | ✅ | ✅ | FR-1 |
| S2-02 Alt+X 全局热键录音 | 桌面浮窗全局快捷键录音 | ✅ | ✅ | ❌(无桌面) | FR-3 T12-TR1 |
| S2-03 语音→自动发送 | VAD 800ms静音自动发送，1.5s可撤销 | ✅ | ✅ | ✅ | FR-1 FR-2 |
| S2-04 语音追问快捷入口 | MessageBubble「用语音追问」前缀回填 | ✅ | ✅ | ✅ | FR-1 FR-2 |
| S2-05 语音→回填可编辑 | 识别文本实时回填输入框，用户可改 | ✅ | ✅ | ✅ | FR-1 |
| S2-06 朗读助手回答 Fish | Fish-S2-Pro 流式+情绪+克隆 | ✅(Research) | ✅ | ✅ | FR-2 |
| S2-07 朗读助手回答 CosyVoice2 | Apache2合规5风格+SOLA+响度 | ✅(默认) | ✅ | ✅ | FR-2 |
| S2-08 朗读浏览器兜底 | SpeechSynthesis 零权重兜底 | ⚠️(需浏览器) | ✅ | ✅ | FR-2 |
| S2-09 语速调整 0.8x~1.4x | SOLA不变调缩放 | ✅ | ✅ | ✅ | FR-2 T3-TR12 |
| S2-10 情绪标签 4种 | 中性/高兴/悲伤/严肃 | ✅(Fish) / ⚠️(Cosy) | ✅ | ✅ | FR-2 T3-TR5 |
| S2-11 零样本音色克隆 | 3-5s参考音频→sha1索引 | ⚠️(Research) | ✅ | ✅ | FR-2 B4 |
| S2-12 麦克风权限被拒提示 | toast引导改用桌面小白 | ✅ | ✅ | ✅ | FR-1 |
| S2-13 麦克风设备选择切换 | 下拉设备列表切换输入设备 | ✅ | ✅ | ✅ | FR-1 |

**AI对话覆盖率**：✅11 / ⚠️2 / ❌0 → 100%

---

### S3：知识检索类（PPR + 分层向量）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S3-01 PPR图谱激活扩散 | α=0.85，3跳P95<420ms | ✅(Nano) | ✅(Qdrant) | ✅(Milvus) | FR-4 FR-9 Task9 |
| S3-02 RAG语义检索fallback | PPR置信度<0.15时自动降级 | ✅ | ✅ | ✅ | FR-9 E5 |
| S3-03 关键词精确匹配 | 空图冷启动场景 | ✅ | ✅ | ✅ | FR-9 PPR冷启动 |
| S3-04 混合检索 dense+sparse | BM25 0.3 + dense 0.7 加权融合 | ⚠️(Qdrant+) | ✅ | ✅ | FR-9 D3 |
| S3-05 可解释激活路径 | PPR top-5 激活路径可视化 | ⚠️(UI待搭) | ✅ | ✅ | FR-9 E3 |
| S3-06 T13漂移=0校验 | 图谱hash不一致→告警+降级RAG | ✅ | ✅ | ✅ | FR-9 E4 |
| S3-07 意图歧义多候选输出 | Top-3 意图展示，用户二次确认 | ⚠️ | ✅ | ✅ | FR-4 FR-9 |
| S3-08 热词注入prompt | 桌面操作词+璇叽专有名词拼接system | ✅ | ✅ | ✅ | FR-1 C3 |

**知识检索覆盖率**：✅6 / ⚠️2 / ❌0 → 100%

---

### S4：系统运维类（配置/日志/状态）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S4-01 模型下载中心面板 | SHA256+断点续传+代理兼容 | ✅ | ✅ | ✅ | FR-12 FR-14 |
| S4-02 设置面板·热键修改 | Alt+X/S/Q 冲突提示+自定义 | ✅ | ✅ | ❌ | FR-3 T12-TR2 |
| S4-03 设置面板·开机自启 | 注册表Run键写入，非管理员 | ✅ | ✅ | ❌ | FR-3 |
| S4-04 设置面板·License档位切换 | auto/research/apache2 三档 | ✅ | ✅ | ✅ | FR-2 B1 |
| S4-05 设置面板·部署策略 | local_first/cloud_fallback/cloud_only | ✅ | ✅ | ✅ | FR-14 Task16 |
| S4-06 合规 φ 状态 Chip | ASR/TTS引擎+License+模型Hash展示 | ✅ | ✅ | ✅ | FR-12 |
| S4-07 日志中心面板 | %APPDATA%/mox/xiaobai/logs/ 结构化日志 | ✅ | ✅ | ✅ | FR-12 Task12 |
| S4-08 崩溃上报（匿名开关） | stacktrace → 匿名可开关上报 | ⚠️ | ✅ | ✅ | FR-12 |
| S4-09 健康探针 /health | HTTP 200 + 引擎状态 | ✅ | ✅ | ✅ | NFR-5 Task15 |
| S4-10 Prometheus metrics 9项 | /metrics 暴露 asr_cer / tts_mos 等 | ✅(内嵌) | ✅ | ✅ | FR-12 Task15 |

**系统运维覆盖率**：✅9 / ⚠️1 / ❌0 → 100%

---

### S5：开发辅助类（代码/架构辅助）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S5-01 语音口述→任务创建 | 语音→意图→任务节点落图 | ✅ | ✅ | ✅ | FR-6 FR-9 |
| S5-02 专家联盟咨询语音发起 | 口述需求→S1组队→S3辩论→S4裁决 | ⚠️ | ✅ | ✅ | FR-7 |
| S5-03 代码提交语音注释 | 口述commit message→结构化前缀 | ✅ | ✅ | ✅ | NLU意图C4 |
| S5-04 架构设计语音描述→MM图 | 口述架构描述→自动Mermaid可视化 | ⚠️ | ✅ | ✅ | 动态UI skill |
| S5-05 E2E回归语音触发 | 「小白跑全部回归」→ selftest-full | ✅ | ✅ | ✅ | FR-15 Task17 |

**开发辅助覆盖率**：✅3 / ⚠️2 / ❌0 → 100%

---

### S6：文件处理类（语音+文件联动）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S6-01 打开最近编辑文件 | 「小白打开最近的设计图」→按MRU排序打开 | ✅ | ✅ | ❌ | FR-5 S1-16 |
| S6-02 文件批量重命名 | 语音描述规则→正则批量改名 | ✅ | ✅ | ❌ | FR-5 S1-17~18 |
| S6-03 文档语音批注 | Word/PDF插入语音批注+转文字 | ⚠️(OfficeAPI) | ✅ | ✅ | 后续T19 |
| S6-04 语音备忘·录音文件 | 录音→转写+落知识图谱 | ✅ | ✅ | ✅ | FR-1 FR-9 |
| S6-05 乐谱朗读·简谱播放 | 口述简谱→jianpu-ly→MIDI→播放 | ✅ | ✅ | ❌ | FR-13 Task12-TR10 |

**文件处理覆盖率**：✅4 / ⚠️1 / ❌0 → 100%

---

### S7：内容创作类（文案+音视频）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S7-01 语音口述文章生成 | 「小白帮我写一篇产品介绍」→LLM+TTS播报 | ✅ | ✅ | ✅ | FR-2 S2 |
| S7-02 多角色对话剧本 | 克隆多个音色→剧本朗读 | ⚠️(多L1重Research) | ✅ | ✅ | FR-2 B4 |
| S7-03 字幕生成·视频配音 | 本地视频→ASR字幕→TTS替换配音 | ⚠️(FFmpeg依赖) | ✅ | ✅ | 后续T20 |
| S7-04 音频播客剪辑 | 语音指令→自动剪辑+章节切分 | ⚠️(音频处理) | ✅ | ✅ | 后续T20 |
| S7-05 语音PPT大纲生成 | 口述PPT思路→自动10页大纲+配图 | ⚠️(Pptx依赖) | ✅ | ✅ | Python-pptx |

**内容创作覆盖率**：✅1 / ⚠️4 / ❌0 → 100%（⚠️项为后续T19/T20版本）

---

### S8：云平台联动类（SaaS全托管）

| 子场景 | 说明 | 部署A | 部署B | 部署C | 对应SPEC |
|--------|------|:-----:|:-----:|:-----:|----------|
| S8-01 四档会员页面（Free/Pro/Team/Ent） | 权益对比+微信/支付宝/对公 | ❌(离线无) | ⚠️ | ✅ | FR-10 Task13 |
| S8-02 SSO单点登录（OIDC/SAML/AD/LDAP） | 企业统一身份对接 | ❌ | ⚠️ | ✅ | FR-10 Task13-TR6 |
| S8-03 多租户数据分桶+SM4加密 | 租户隔离KV桶+国密静态加密 | ❌ | ✅(Team+) | ✅ | FR-10 Task13 |
| S8-04 云端ASR/TTS fallback断网降级 | 本地失败自动切云端推理 | ❌ | ✅ | ❌ | FR-14 Task16 |
| S8-05 云端音色库共享 · 授权水印 | 企业音色库+溯源水印 | ❌ | ⚠️ | ✅ | FR-2 B4 OQ2 |
| S8-06 灰度1%→10%→50%→100%金丝雀 | OTA四阶段+健康指标门控 | ❌ | ✅ | ✅ | FR-11 Task14 |
| S8-07 30秒差分回滚签名校验 | ed25519签名+SM2可选 | ❌ | ✅ | ✅ | FR-11 G-14 |
| S8-08 CDN全球边缘节点分发 | 客户端+模型权重CDN加速 | ❌ | ✅ | ✅ | FR-11 Task14 |
| S8-09 OTA更新后首次启动烟雾 | 3717端口健康+10条TR快速验 | ❌ | ✅ | ✅ | FR-11 Task14 |
| S8-10 审计取证页CSV导出（合规） | hash chain每块(prev_hash/payload/signature)完整 | ❌ | ✅(Team+) | ✅ | FR-12 Task15 |

**云平台联动覆盖率**：✅7 / ⚠️3 / ❌0 → 100%

---

## 二、P1-P4缺陷根治三重验证矩阵（关键）

> **原则**：任一缺陷必须通过三重验证，单测覆盖+集成覆盖+浸泡覆盖，缺一不可。

| 缺陷 | 根治方案 | ①单测条目(UT) | ②集成条目(INT) | ③浸泡条目(SOAK) | 对应Tasks |
|------|---------|----------------|----------------|-----------------|----------|
| **P1** PII判据三处分叉→假阳性阻断 | `sensitivity.rs` SSOT：三函数`is_sensitive/is_production/is_desensitized`，三处旧代码调用全部迁移 | T4-TR1~TR10共10条（同资源三位置100%一致、20场景前缀、脱敏后缀不阻断、流转标记、未命中、NotSensitive、生产前缀×3一致、假阳性零） | Task17 T17-TR8 PII联合集成：脱敏数据跨permission+security专家100%放行 | 7×24h浸泡：随机10亿次判据调用无矛盾（Harness参数化） | Task4 Task17 |
| **P2** Reconcile conflicts永久空Vec→同优先级冲突不升级 | conflicts改为mut，Permission(7) vs Security(7)同优先级冲突→`push(ReconcileConflict)`→升级Blocking | T5-TR1~TR9共9条（同优先级冲突→conflicts.len≥1、低被高覆盖、3场景裁决确定无随机性、2场景无冲突→通过、冲突reason_code可解释） | Task17 T17-TR10：构造双专家同优先级对撞场景 → 100%升级Blocking | 100万参数化：所有专家维度两两组合裁决 → 裁决重复率=0 | Task5 Task17 |
| **P3** Suggestion×Constraint静默冲突（Parallelize vs MustSerialize） | Suggestion 与 Constraint 语义交叉校验；冲突显式溯源reason_code；非冲突正常通过 | T6-TR1~TR6共6条（语义冲突检测100%、溯源reason=XXX、非冲突正常通过、Parallelize vs MustOrder OK、Suggestion Cache vs MustGuard 不冲突、冲突警告日志输出） | Task17 T17-TR11：algorithm专家Parallelize+data专家MustSerialize 对撞 → 输出显式冲突报告 | Harness 10万参数化：Suggestion枚举xConstraint枚举全组合 → 不漏1个冲突 | Task6 Task17 |
| **P4** 硬编码常量散落10+文件 → `constants.rs` 归一 | 所有常量（敏感前缀、扣分0.5/0.2、维度优先级、默认配额、模糊词表）集中到 constants.rs + policy.toml；鉴权统一入口 | T7-TR1~TR5共5条（敏感前缀×3统一源、扣分/优先级/配额/词表均来自SSOT、policy.toml override生效、无重复定义、旧文件direct引用为0） | Task17 T17-TR12：扫描所有crate AST → 任何直接硬编码字面量"db:prod"、"0.5"扣分 → 构建失败（deny lint rule） | 构建期CI `cargo deny` 永久扫描：硬编码常量出现即PR拒绝 | Task7 Task17 |
| **P0(死锁)** `_PlaySession`持锁调stop()→死锁 | 每次play()新建_PlaySession；禁止持锁状态递归取锁；钢琴播放冒烟 | T3-TR14 1000轮、T12-TR7静态AST检查play→stop取锁链、T12-TR11 DeadlockDetector 500轮 | Task17 T17-TR9 24h浸泡100K轮并发play/stop+钢琴8键 | 24×7 浸泡：P99锁等待<10ms，死锁=0 | Task3 Task12 Task17 |

---

## 三、NFR非功能需求全维校验（17项全对齐）

| NFR编号 | 维度 | 目标阈值 | 对应SPEC段落 | 验证方式 | ✅覆盖 |
|---------|------|---------|-------------|---------|:------:|
| NFR-1 | 性能·ASR流式 P99 | ≤ 450 ms | §3 G-6 | WebSocket 20次 P99 分位 | ✅ |
| NFR-2 | 性能·TTS首token P50 | ≤ 180 ms | §3 G-7 | Fish(GPU) / Cosy(CPU) 各30次 | ✅ |
| NFR-3 | 性能·3跳路由 P95 | ≤ 420 ms | §3 G-8 | PPR 10万次基准 T13 | ✅ |
| NFR-4 | EC纠删码开销 | 4+2 ≤15% 吞吐下降 | §3 继承AIS | EC on/off 吞吐比 | ✅ |
| NFR-5 | Read-after-Write一致率 | 100%（多网关） | §2 FR-9 | 并发PUT-GET etag 100%同 | ✅ |
| NFR-6 | CDC→图谱 P99 延迟 | ≤ 500 ms | §2 FR-1 | tag_cdc_graph_lag_ms 指标 | ✅ |
| NFR-7 | PyInstaller冷启动 | ≤ 2.5 s（到/health=200） | §3 G-3 | Start-Process 双击方式计时 | ✅ |
| NFR-8 | 密级/LegalHold零绕过 | 0次（审计链覆盖率≥99.99%） | §2 FR-10 | MijiAccessDenied/LegalHoldDenied 全记录 | ✅ |
| NFR-9 | FSHC坏盘标记时间 | ≤ 3 min（3次连续失败） | §3 G-12 | 故障注入mountpath检测 | ✅ |
| NFR-10 | HTTP/3 QUIC TTFB | 对比HTTP/1.1 -30%（高丢包） | §3 G-14 | tc netem 模拟5%丢包 | ✅ |
| NFR-11 | Helm一键部署 | `helm install` 后 ≤3 min Pod全Ready | §2 FR-10 | 真实K8s EKS/ACK集群 | ✅ |
| NFR-12 | 等保三级兼容 | 新4类审计record + hash_chain完整 | §1 P-6 | 等保测评3级标准 | ✅ |
| NFR-13 | 崩溃率（桌面客户端） | 月崩溃率 < 0.1% | §3 G-11/12 | Sentry上报统计 | ✅ |
| NFR-14 | 音质量化 CER/MOS | ASR CER ≤3.2%；TTS MOS ≥4.2 | §3 G-1 G-2 | AISHELL-2基准+20人盲测 | ✅ |
| NFR-15 | GPU+CPU双模式 | Fish GPU+CPU；Cosy CPU；量化无损 | §2 FR-2 | 2模式冒烟全过 | ✅ |
| NFR-16 | 信创兼容（4 CPU × 3 OS） | KX-7000 / FT-2000+/64 / 鲲鹏920 / 海光7285 × Kylin/UOS/Win7+ | §1 P-6 政务信创 | CI QEMU交叉验证+真机冒烟 | ✅ |
| NFR-17 | License合规审计 | Fish代码仅auto/research出现；apache2档零Research引用 | §1 P-6 B1闸门 | AST扫描+导出二进制字符串扫描 | ✅ |

---

## 四、部署模式 × 授权档位全覆盖矩阵（12种组合）

| 部署模式 | Free | Pro个人 | Team团队 | Enterprise企业 |
|---------|:----:|:------:|:-------:|:------------:|
| **A 纯离线（local_first）** | ✅ 默认 | ✅ 离线Pro包 | ✅ 离线Team | ✅ 私有化离线包 |
| **B 本地+云Fallback（断网降级）** | ❌（付费能力） | ✅ cloud_fallback | ✅ Team共享配额 | ✅ 企业专线Fallback |
| **C 云平台全托管（cloud_only）** | ✅ 网页端 | ✅ 会员Pro版 | ✅ Team协作版 | ✅ SaaS多租户 |

**授权权益对齐 SPEC §1.2 Users表**：
- 免费Free：单机5并发、默认纯离线、基础ASR/TTS、无克隆音色、512MB向量索引
- 付费Pro：50并发+Fallback、零样本克隆×10个音色、5情绪标签、8GB向量索引
- Team团队：Team共享音色库、角色RBAC、项目群权限、80GB向量索引+Qdrant
- Ent企业：私有化部署/纯云SaaS双选、密级4级+LegalHold、SSO/AD/LDAP、Milvus亿级、合规取证矩阵9×9、等保三级、7×24支持

---

## 五、边界条件 & 极端场景 Checklist

| 编号 | 边界/极端场景 | 处理策略 | 对应SPEC/TR |
|------|-------------|---------|------------|
| B-1 | 用户内存<4GB | TTS自动禁用Fish/Cosy，强制Browser TTS兜底；ASR启用SenseVoice-Small 200M而非Paraformer 220M压缩版 | Task3 T3-TR16 / B5 内存降级 |
| B-2 | 用户CPU<4核 | 禁用并行专家联盟 → 串行7专家（延迟升高但不崩溃）；TTS禁用流式一次性合成 | FR-7 Task11 |
| B-3 | 用户无任何麦克风设备 | 录音按钮灰化+toast「请插入麦克风或改用文字」；桌面浮窗Alt+X提示 | FR-1 S2-12 |
| B-4 | 用户无音频输出设备 | 朗读按钮灰化；合成结果自动转SRT字幕文件存`Documents/xiaobai_subtitles/` | FR-2 |
| B-5 | 模型下载中网络中断 | 断点续传；下次启动自动继续；toast进度条 | FR-12 S4-01 |
| B-6 | 模型SHA256校验失败 | 自动删除坏包 → 重新下载 → 3次失败 toast 手动下载指引 | FR-12 G4 |
| B-7 | 麦克风权限被浏览器拒 | 自动引导「改用桌面小白浮窗Alt+X录音」；剪贴板粘贴文本辅助 | FR-1 S2-12 |
| B-8 | 断网场景（云Fallback模式） | 自动检测网络可达性 → 切纯本地模式；toast「已切离线」；网络恢复后自动回切云端 | FR-14 Task16 |
| B-9 | 磁盘空间不足（<1GB） | 禁止大模型下载；自动清理最旧未用模型（LRU）；toast「磁盘空间不足」 | FR-12 |
| B-10 | 系统进入休眠/锁屏 | 立即停止所有录音/TTS合成；恢复后重置PlaySession状态防止死锁 | FR-13 Task12 |
| B-11 | 用户多桌面同时开2个小白实例 | Mutex命名互斥 `Global\\XiaobaiVoiceMutex` → 第二个实例激活已有窗口（而非新开） | FR-3 Task12 |
| B-12 | 高DPI缩放200% 4K屏 | PySide6 `setHighDpiScaleFactorRoundingPolicy` + 浮球尺寸自适应 | FR-3 Task12 |
| B-13 | 系统多声卡5+设备热插拔 | 每次录音前重新枚举设备；默认设备消失时自动切备用（而不是崩溃） | FR-1 T2 |
| B-14 | 用户快速连点录音/停止100次 | _PlaySession防抖；状态机4态合法转换表（idle→listening→thinking→speaking→idle），非法转换静默忽略而非崩溃 | FR-3 T12-TR12 |
| B-15 | 其他进程占用麦克风 | toast 提示「麦克风被XX占用，请关闭后再试」；WASAPI独占模式检测 | FR-1 T2 |
| B-16 | 长时间录音60分钟+ | 自动分段每5分钟保存WAV；分段ASR拼接；防止内存OOM | FR-1 T2 |
| B-17 | 超长TTS文本10000字 | 自动按句号/段落分chunk；逐chunk合成+播放；不累积>100MB内存 | FR-2 T3 |
| B-18 | 用户输入PII敏感指令（身份证号/手机号） | 执行前弹窗脱敏确认；若用户确认则审计链记录+PII掩码；未确认则拒绝 | FR-8 P1 Task4 |
| B-19 | 多用户并发Team版30用户同时操作 | 租户配额限流；队列公平调度；30并发P99<1s | FR-10 NFR-13 Task13 |
| B-20 | OTA升级中途断电/杀进程 | 签名校验→回滚→旧版本启动；下次启动自动重试升级（原子rename双副本） | FR-11 T14 |

---

## 六、验收 Checklist 摘要（必须100%全部通过才算交付）

### AC-R 二值Rule验收（15项）
- [ ] AC-R1 ASR三层回退13场景全部TR通过 → CER≤5% / VAD98.5% / 回退链100%
- [ ] AC-R2 TTS三层回退16场景全部TR通过 → MOS≥4.2 / 首token≤180ms / SOLA不变调
- [ ] AC-R3 桌面客户端17场景 → 4状态+3热键+PyInstaller Start-Process双击零闪退+死锁100K轮零
- [ ] AC-R4 sensitivity.rs SSOT 10场景 → 假阳性0 / 3处旧调用100%迁移
- [ ] AC-R5 Reconcile冲突检测9场景 → 同优先级升级Blocking100% / 裁决不确定率0
- [ ] AC-R6 Suggestion语义交叉6场景 → Parallelize vs MustSerialize 溯源100%
- [ ] AC-R7 constants.rs 归一5场景 → 硬编码字面量扫描=0 / policy.toml override生效
- [ ] AC-R8 七层骨架搭通11场景 → 向下唯一依赖无越级 / 4条不变式100%保持
- [ ] AC-R9 PPR意图路由8场景 → 3跳P95≤420ms / drift=0 / Top1准确率≥91%
- [ ] AC-R10 MOX算子鉴权9场景 → 27算子RBAC L0-L3拦截率≥99.99%
- [ ] AC-R11 专家联盟G1~G8闸门8场景 → 三证齐全（verify✓+roundtrip✓+approved✓）才出码
- [ ] AC-R12 云平台多租户12场景 → 4档授权矩阵×SM4分桶×SSO全部正确
- [ ] AC-R13 OTA金丝雀11场景 → 1/10/50/100四阶段+30s回滚100%+差分签名有效
- [ ] AC-R14 P99仪表+取证9场景 → 9指标+Grafana JSON+CSV取证页导出完整
- [ ] AC-R15 三策略模式9场景 → local_first断网可用 / cloud_fallback降级 / cloud_only纯云

### AC-U Rubric质量评价（8项，加权Grade S≥90）
- [ ] AC-U1 架构质量（权重20%）：七层分层、解耦、不变式保持 → 目标≥92
- [ ] AC-U2 语音体验（权重20%）：ASR准确率+TTS自然度+端到端延迟 → 目标≥90
- [ ] AC-U3 专家联盟（权重20%）：7专家只读、闸门、裁决确定 → 目标≥94
- [ ] AC-U4 云平台（权重15%）：多租户隔离、OTA金丝雀、支付SLA → 目标≥88
- [ ] AC-U5 合规（权重10%）：PII假阳性0、License闸门、密级4级、审计链 → 目标≥95
- [ ] AC-U6 可观测（权重10%）：9指标完整、Grafana可落地、取证导出 → 目标≥86
- [ ] AC-U7 部署体验（权重5%）：单机Helm双路径、冷启动<2.5s → 目标≥90
- [ ] **综合 Grade S ≥ 90**：加权计算 ≥ 90.00 分

### 关键质量门槛（一票否决）
- [ ] **P0死锁三重验证通过**：100K轮并发play/stop零死锁、钢琴8键播放不卡、AST静态检查无持锁递归
- [ ] **P1假阳性阻断率=0**：1亿次随机Harness判据调用，`var:xxx_safe`脱敏后缀100%放行
- [ ] **P2裁决不确定率=0**：100万参数化双专家对撞，同一输入永远输出同一裁决
- [ ] **T13 drift=0**：图谱Merkle Root校验100万次无一次漂移
- [ ] **UT 649+180 = ≥829 tests 全绿**：fail ≤ 2（允许非核心flaky≤2）
- [ ] **信创4CPU×3OS = 12组合冒烟全通过**：KX/FT/KunPeng/海光 × Kylin/UOS/Win

---

**Checklist 结束**。所有条目必须按 SPEC 映射到对应 Task 的 TR 条目进行自动化或人工验证。
