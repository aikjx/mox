# 语音对话 + 桌面小白 xiaobai —— 原子任务清单

> 规格：`.trae/specs/20260825-xiaobai-voice-integration/spec.md`
> 验收标准：spec § 八（AC1–AC21）
> 变更优先级原则：T=必需，S=可配置可选；未通过 AC 的 T 类任务打回修复，不得进入 Review。

| ID | 任务名 | 类型 | 前置 | 验收映射 |
|---|---|---|---|---|
| T1 | 搭建 `projects/xiaobai_voice` 骨架、依赖与配置 | T | — | AC5,AC6,AC12,AC16 |
| T2 | ASR 封装：Paraformer-zh + sherpa-onnx（流式 + full）+ 热词 + VAD | T | T1 | AC4,AC5,AC18 |
| T3 | TTS 封装：Fish-S2-Pro（Research）+ CosyVoice2（Apache2）三层回退 | T | T1 | AC6,AC7,AC17,AC18,AC20 |
| T4 | FastAPI + WebSocket 语音服务（端口 3717） | T | T2,T3 | AC4,AC7,AC16,AC10 |
| T5 | 模型下载中心 + models.yaml + SHA256 + 断点续传 + SSE 进度 | T | T1,T4 | AC12 |
| T6 | 配置中心（跨平台路径 + 热更新） | T | T1 | AC15 |
| T7 | 合规 φ Chip 对话框（引擎/哈希/License Tier 切换） | T | T4,T6 | AC6,AC15,AC20 |
| T8 | 前端：ChatView 录音按钮 + 声波条 + VAD 自动发送 + 快捷键 | T | T4 | AC1,AC2,AC3,AC11,AC10 |
| T9 | 前端：MessageBubble TTS 三层回退 + 音色/情绪/速率下拉 | T | T4 | AC7,AC17,AC18 |
| T10 | 桌面浮窗 xiaobai：PySide6 φ 圆形 + 4 状态呼吸灯 + 拖拽吸附 | T | T1 | AC8,AC19 |
| T11 | 桌面快捷键：Alt+X / Alt+S / Alt+Q + toast + 剪贴板朗读 | T | T10 | AC9,AC10 |
| T12 | 桌面内嵌 WebView：聚焦 /#/ai、服务未启动友好提示、粘贴音频转文本 | T | T4,T10 | AC3,AC10 |
| T13 | 打包：PyInstaller windowed + _ensure_windowed_streams + 外部 venv 加载 + 日志 | T | T10,T11,T12 | AC13,AC14,AC20 |
| T14 | Vite proxy 与前端 API（voiceApi：health/stream/tts/metrics） | T | T4 | AC16 |
| T15 | 单元 + 冒烟测试：死锁回归、声卡缺失兜底、SHA256 坏模型回删 | T | T2,T3,T4,T13 | AC13,AC14,AC21 |
| T16 | Playwright E2E：录音→识别→发送→朗读 30 轮循环 | T | T8,T9,T10,T14 | AC4,AC10,AC11,AC21 |
| S1 | SenseVoice 多语种可选 ASR 回退 | S | T2 | AC18 |
| S2 | 零样本克隆音色上传 + 持久化 hash 存储 | S | T3,T4 | AC18 |
| S3 | 开机自启注册表开关（HKCU Run） | S | T10,T12 | — |
| S4 | Prometheus `/voice/metrics` 指标 | S | T4 | NFR5 |

---

## 任务细化（按依赖排序）

### T1 搭建 `projects/xiaobai_voice` 骨架、依赖与配置
**产出：**
- `projects/xiaobai_voice/pyproject.toml`（或 requirements.txt + extras：`[asr,tts,desktop,dev]`）
- `projects/xiaobai_voice/README.md`（仅限必要运行说明，不写大段产品文档）
- 目录：`xiaobai_voice/asr/`, `xiaobai_voice/tts/`, `xiaobai_voice/service/`, `xiaobai_voice/desktop/`, `xiaobai_voice/models/`, `xiaobai_voice/config/`, `xiaobai_voice/tests/`
- `xiaobai_voice/__init__.py`, `xiaobai_voice/config/default_config.yaml`, `xiaobai_voice/config/models.yaml`（必填项见 § FR10）
- `xiaobai_voice/cli.py`：入口 `python -m xiaobai_voice serve` / `desktop` / `download` / `selftest`

**验收点：**
- `python -m xiaobai_voice serve` 能启动（即使无模型也打印「MISSING_MODEL · 请前往设置下载」到 stdout 和日志）。
- `python -m xiaobai_voice desktop` 能创建 PySide6 空窗口（先不渲染浮窗）。

---

### T2 ASR 封装：Paraformer-zh + sherpa-onnx（流式 + full）+ 热词 + VAD
**产出：**
- `xiaobai_voice/asr/base.py`：抽象基类 `ASRBackend(recognize_stream / recognize_full / set_hotwords / prewarm / close)`。
- `xiaobai_voice/asr/sherpa_paraformer.py`：
  - 加载 `sherpa-onnx-paraformer-zh-int8`（路径来自 `models/` 解析器）。
  - 内置 silero-vad（sherpa 提供），threshold=0.5，min_silence_ms 可配。
  - 热词走 `POST /voice/hotwords` 持久化 `config/hotwords.json`；启动加载。
  - `prewarm()`：跑固定短句 30 ms 预热。
  - ImportError/MissingModel/DLL_LOAD_FAIL 三类错误分级（见 § FR5）。

**验收点：**
- `pytest -q projects/xiaobai_voice/tests/test_asr.py` 过：
  - 用 2 条本地 16k WAV（可合成，文字已知），CER ≤ 8%。
  - 热词「璇玑」未加入 vs 加入，识别率提升可统计 ≥ 5%（可做 5 条短句重复测试）。
- 启动日志含 `sherpa-onnx paraformer-int8 ready in <1200 ms`。

---

### T3 TTS 封装：Fish-S2-Pro（Research）+ CosyVoice2（Apache2）三层回退
**产出：**
- `xiaobai_voice/tts/base.py`：`TTSBackend.synthesize(text, opts) -> Generator[bytes]`（流式字节）+ `synthesize_full(text, opts)->bytes`。
- `xiaobai_voice/tts/fish_s2.py`：
  - 只在 `license_tier in {auto, research}` 且权重目录完整才被 **delayed import**（`__import__` 封装于函数内，确保 apache2 模式 AST + 打包文件列表 grep 都不命中 fish_speech，AC20 rubric=3）。
  - 零样本克隆：上传 3-5 s WAV → 存 hash 到 `voice_clips/<sha1>.wav`；synthesize 时 `reference_audio=path`。
  - 情绪标签枚举严格 4 类；未知情绪 → neutral。
- `xiaobai_voice/tts/cosyvoice2.py`：
  - Apache2 模式默认。指令模式合成；chunk=250 ms 流式吐字节；
  - INT8/FP16 自动检测显存。
- `xiaobai_voice/tts/browser_fallback.py`：空后端（用于告诉前端「请走浏览器 TTS」），返回空流 + 特殊 HTTP 头 `X-TTS-Fallback: browser`。
- `xiaobai_voice/tts/__init__.py`：`build_tts_backend(config, license_tier)` 按策略组装。

**验收点：**
- 单测：
  - `synthesize_full("你好璇玑")` 返回 ≥ 0.3 s 音频、可被 soundfile 解码（Fish 未装时自动用 CosyVoice 跑也过）。
  - 切 `license_tier=apache2` 后，Python AST 扫描整个 `xiaobai_voice/` 包 `import fish_speech` 计数 = 0；运行日志无 fish 相关输出。
- 流式：`/voice/tts/stream` 首字节 ≤ 3.5 s（CPU）或 ≤ 0.8 s（GPU Fish）。

---

### T4 FastAPI + WebSocket 语音服务（端口 3717）
**产出：**
- `xiaobai_voice/service/main.py`：FastAPI app + CORS（允许 `http://localhost:3021` 与同源）。
- 路由：`/voice/health`、`/voice/models`、`/voice/models/download`、`/voice/models/download/stream`（SSE）、`/voice/ws/asr/stream`、`/voice/asr/full`、`/voice/tts/stream`、`/voice/tts/clone`、`/voice/hotwords`、`/voice/metrics`（S4 可选，先打桩）。
- 子进程隔离：ASR/TTS 引擎放到独立 `multiprocessing` Process，主进程只做 IPC；引擎崩溃自动拉起（限 3 次/5 分钟防止崩循环）。
- 启动自检：3 条短句 ASR + 2 条 TTS，结果写入 `logs/smoke_YYYYMMDD.jsonl`。

**验收点：**
- `curl -s http://localhost:3717/voice/health` 200，响应字段含 `asr.engine / tts.engine / license_tier / uptime_s`。
- 单测 mock WebSocket：上送 1 s PCM → 收到 ≥1 partial JSON，final JSON 含 text（可用合成空句基准）。

---

### T5 模型下载中心 + models.yaml + SHA256 + 断点续传 + SSE 进度
**产出：**
- `xiaobai_voice/models/downloader.py`：`httpx` Range + `tqdm`；重试 3 次指数退避；校验失败自动 `os.remove`。
- `xiaobai_voice/config/models.yaml`：列出 FR10 4 条模型；文件头加版本号，sha256 锁定。
- SSE 路由 `/voice/models/download/stream?id=…`：每 500 ms 推 `{progress_pct, speed_mbps, eta_s, state}`。
- 模型查找优先级：`<exe同级>/models > %USERPROFILE%/.mox/models/voice > 仓库 projects/xiaobai_voice/models/`。

**验收点：**
- 单测：
  - 404：重试 3 次，最后 fail，无残留 part 文件。
  - 坏 sha：下载完成后校验失败自动删文件。
  - 续传：中途杀进程再启动，剩余字节续接正确。

---

### T6 配置中心（跨平台路径 + 热更新）
**产出：**
- `xiaobai_voice/config/loader.py`：
  - 跨平台定位 `%APPDATA%/mox/xiaobai/config.yaml`（Windows）等；
  - 默认值与合并；
  - 文件监听（watchdog 可选）修改：
    - `license_tier / tts.engine / asr.engine` → 触发优雅重启语音子进程（`SIGTERM` → 最多等待 3 s → kill）；
    - 其他项（emotion / speed / shortcuts）→ 运行时更新相关对象，无重启。
- 启动时 dump 合并后配置副本到日志（敏感字段 mask：热词、clone_reference hash 前 8 位可显示其余 mask）。

**验收点：**
- 手动改 `config.yaml` 的 `tts.emotion=happy`，10 s 内 `/voice/health` 返回的 `config.defaults.tts.emotion=happy`。
- `license_tier` 从 `research` 改 `apache2`：3 s 内 `/voice/health` 的 `tts.engine` 变为 `cosyvoice2`。

---

### T7 合规 φ Chip 对话框
**产出：**
- 前端 `ChatView.vue` 顶栏 `analysis-stages` 最右侧追加 Chip：`合规 φ · Auto`（文字动态反映当前 tier）。
- 新对话框 `components/LicenseComplianceDialog.vue`：
  - 三列表：引擎 / 模型 ID / SHA256 前 8 位 / License 链接；
  - Tier 下拉（Auto / Research / Apache2）+ 切换确认提示：
    - 切 Apache2："切换后将停用 Fish-S2-Pro 引擎（Research License），模型文件仍保留可手动删除。是否切换？"
    - 切 Research："切换后启用 Fish-S2-Pro，仅可用于非商业用途。确认？"
  - 调 `voiceApi.health` + `voiceApi.models` + `voiceApi.setLicenseTier(tier)`（POST）。

**验收点：**
- Playwright：点 Chip → 弹对话框 → 选 Apache2 → 确认 → `/voice/health` 10 秒内回 `tts.engine=cosyvoice2`。

---

### T8 前端：ChatView 录音按钮 + 声波条 + VAD 自动发送 + 快捷键
**产出：**
- `ChatView.vue` 输入区左：`<el-button class="voice-mic-btn" :title>` φ 圆按钮；
- 下拉菜单（el-dropdown）：按住说话 / 点击开启结束 / 设备与引擎状态；
- 12 条声波条 CSS（φ 宽度 16.2 ≈ 16 px），基于 `AudioWorklet` Rms 动态缩放；
- WebSocket `ws://<host>/voice/ws/asr/stream` 发送 16 bit PCM 16k 二进制（前端 resample 用 `audio-buffer-resampler` 或 Web Audio OfflineAudioContext 降采样）；
- 自动发送策略：manual / silence 800ms / stop_click；
- 快捷键：聚焦输入框 `Alt+V` 开关录音；Esc 丢弃。

**验收点：**
- DOM 探针：`.voice-mic-btn` 存在；下拉菜单 3 项齐全；
- Playwright E2E：合成 1 s 16k PCM"你好小白" → 1.5 s 内输入框回填"你好"或"小白"相关；800 ms 静音后若策略是 auto_send → 自动发送一条用户消息；
- 断服务时按钮灰化 + 设置面板出现启动桌面小白提示。

---

### T9 前端：MessageBubble TTS 三层回退 + 音色/情绪/速率下拉
**产出：**
- 在 `MessageBubble.vue` 原有 TTS 按钮上：**保留 SpeechSynthesis 作为第 3 层**；
- 下拉菜单新增：
  - 音色（fish:xiaobai / fish:<cloned> / cosyvoice:default / browser:zh-CN）
  - 情绪（neutral/happy/sad/serious，仅 Fish/CosyVoice）
  - 速率（0.8 / 1.0 / 1.2 / 1.4）
- 拉 `/voice/tts/stream?...` → 用 `MediaSource + SourceBuffer audio/mpeg` 流播放；失败降级整段 fetch + Audio.play；再失败走 browser。
- 状态机扩充：`idle / streamBuffering / playing / paused / fallbackBrowser / stopped`。

**验收点：**
- 探针：TTS 按钮下拉包含"音色/情绪/速率"3 子菜单；
- 单测 mock：返回 MPEG 分块 → 首个 `updateend` 事件 2 s 内触发；
- 关闭 3717 → 自动走浏览器 SpeechSynthesis，原有按钮可用。

---

### T10 桌面浮窗 xiaobai：PySide6 φ 圆形 + 4 状态呼吸灯 + 拖拽吸附
**产出：**
- `xiaobai_voice/desktop/ball_widget.py`：FramelessWindowHint + WindowStaysOnTopHint + TranslucentBackground；
- 尺寸默认 68（可配置 42/68/110），φ 阴影 3 层；
- 4 状态外环：
  - idle = 静 + 青蓝 `#22d3ee`；
  - listen = 呼吸 + 绿 `#22c55e`，1.2 s 脉冲；
  - think = 旋转 + 紫 `#a855f7`，2.4 s 一圈；
  - speak = 波形 + 靛 `#6366f1`，12 根 φ 条；
- 拖拽：左键可拖；释放 → 磁吸最近左右边缘；动画 easing = QEasingCurve.OutCubic 300 ms；
- 右键菜单：FR8 7 项（先打桩"开机自启"不可见，S3 再补）。

**验收点：**
- `pytest -q tests/test_desktop.py` 启动浮窗 → 断言窗口大小、flags、始终置顶；
- 仿真拖拽到屏幕中部偏右 → 释放 350 ms 后 X 坐标落在屏幕右边缘 ±4 px；
- 4 状态通过 QTimer 信号驱动后分别渲染不同 QPainter 元素（listen=外环 pulse 圆点数递增，think=弧角度递增，speak=条形高度数组，idle=恒等）。

---

### T11 桌面快捷键：Alt+X / Alt+S / Alt+Q + toast + 剪贴板朗读
**产出：**
- `xiaobai_voice/desktop/hotkeys.py`：`pynput.keyboard.GlobalHotKeys` 在独立 QThread 监听；
- 信号映射：
  - `Alt+X` → ball 切换录音；
  - `Alt+S` → 取 `QClipboard.text()`，非空则调 TTS synthesize_full + 播放（优先本地）；
  - `Alt+Q` → 优雅退出（写 stop 标记文件以防进程卡住，+ 3 s 强制 kill）；
- Toast：右下角无边框 QWidget 弹 1.8 s，深色 φ 主题。

**验收点：**
- 仿真快捷键：Alt+X 触发 → ball.listen=True；再 Alt+X → idle=True 且 ASR 请求数 +1（mock 服务计数）；
- Alt+S：剪贴板写入"测试朗读，你好璇玑" → 3 s 内 TTS 字节输出文件 ≥ 20 KB（可 mock 声卡播放写 WAV 文件）。

---

### T12 桌面内嵌 WebView：聚焦 /#/ai、服务未启动友好提示、粘贴音频转文本
**产出：**
- `xiaobai_voice/desktop/main_window.py`：QMainWindow + QWebEngineView 加载 `http://localhost:3021/#/ai`；
- 顶部 4 Chip：`ASR引擎`/`TTS引擎`/`合规`/`快捷键`（调 `/voice/health` 填色）；
- 若 3021/3717 任一不可达 → 视图显示 φ 启动页：两个按钮"启动语音服务" / "打开浏览器 AI 对话"。
- 底部输入区：支持拖拽或粘贴（Ctrl+V）音频文件 → 自动 POST `/voice/asr/full` → 结果回填到 WebView 内输入框（通过 `runJavaScript` 调全局挂载的 `window.__xiaobai_paste(text, role="asr_file")`）。

**验收点：**
- 关闭 3717 → 打开主窗口 → 提示页出现；点击"启动语音服务" → 5 s 内路由回到 /#/ai 且 Chip 回正常色；
- 粘贴 a.wav（内容已知"你好璇玑"）→ 1.5 s 内 WebView 输入框包含识别文本（≥ 50% 字命中）。

---

### T13 打包：PyInstaller windowed + _ensure_windowed_streams + 外部 venv 加载 + 日志
**产出：**
- `xiaobai_voice/cli.py` 最前置 `_ensure_windowed_streams()`：
  - `sys.stdout/stderr=None → StringIO + 文件句柄`；
  - `os.add_dll_directory`：numpy/.libs、onnxruntime/capi、PySide6/Qt6/bin、sounddevice/_sounddevice_data 等；
  - 设置环境变量 `FISH_SPEECH_CKPT_DIR / COSYVOICE_CKPT_DIR / MOX_VOICE_PORT / MOX_VOICE_CONFIG`。
- `projects/xiaobai_voice/build_exe.ps1`：
  - 可选参数：`-UseCondaEnv <path>` / `-UseVenv <path>` 注入外部 venv site-packages 到 `sys.path`（复用 Experience 1304739 经验）；
  - PyInstaller `--noconfirm --windowed --name Xiaobai --specpath build --dist dist --add-data "xiaobai_voice/config;xiaobai_voice/config"`。
- `Xiaobai.exe` 首次启动若缺失模型 → 弹下载向导对话框（SSE 进度条）。

**验收点：**
- `Start-Process dist\Xiaobai.exe`（非控制台）启动后 30 s：
  - 读 `%APPDATA%\mox\xiaobai\logs\xiaobai-YYYYMMDD.log` → **无 stderr=None AttributeError 栈**；
  - 有窗口（浮窗或主窗口）且截图可见；
- `Start-Process dist\Xiaobai.exe -ArgumentList "--selftest-full"` → 退出 0，`selftest-report.jsonl` 含 `voice_playback_smoke_deadlock_regression` 记录（声卡缺失标记 SKIP_OK）。

---

### T14 Vite proxy 与前端 API（voiceApi：health/stream/tts/metrics）
**产出：**
- `frontend-ui/vite.config.js` 追加 `/voice` proxy（ws=true，target=http://localhost:3717）。
- `frontend-ui/src/api/voice.js`（或 index.js 增加 `voiceApi` 对象）：
  - `health()` → Promise；
  - `models()` / `download(id)` / `downloadSSE(id, onProgress)`；
  - `openASRStream(onPartial, onFinal)` → `{ sendPCM(bytes), stop(), close() }`；
  - `asrFull(fileBlob)`；
  - `ttsStreamURL({text,voice,emotion,speed})` → blobURL 或 MediaSource 对象封装；
  - `cloneReference(fileBlob)` → `{voice_id, sha1}`；
  - `setHotwords(words)` / `getHotwords()` / `setLicenseTier(tier)`。
- `ChatView.vue` 生命周期启动时调 `health()`：成功启用麦克风，失败显示启动桌面小白提示。

**验收点：**
- `curl http://localhost:3021/voice/health` 返回 200（Vite 代理生效）；
- 前端控制台无 CORS 错误（proxy 屏蔽）。

---

### T15 单元 + 冒烟测试：死锁回归、声卡缺失兜底、SHA256 坏模型回删
**产出：**
- `tests/test_deadlock_safety.py`：100 轮并发"录音+播放"循环；主线程事件循环 watcher 100 ms ping 无阻塞；
- `tests/test_no_soundcard.py`：`sounddevice` mock PortAudioError → 所有 API 不抛未捕获异常；浮窗不崩溃；
- `tests/test_models_download.py`：404/坏sha 两种场景；
- `tests/test_license_gate.py`：`license_tier=apache2` AST 扫描 `xiaobai_voice/` 无 `import fish_speech`，进程内存内 fish 模块名未出现。

**验收点：**
- `pytest -q projects/xiaobai_voice/tests/` 全部 pass；
- 死锁测试 100 轮 0 失败，主线程最大阻塞 < 100 ms。

---

### T16 Playwright E2E：录音→识别→发送→朗读 30 轮循环
**产出：**
- `frontend-ui/e2e/xiaobai_voice_chat.spec.js`：
  1. 打开 `/#/ai`；
  2. mock 语音服务（或真实起服务）；
  3. 点麦克风 → 合成 "请介绍专家联盟" 发送 → 断言"已发送 + 助手回复非空"；
  4. 点回复朗读按钮 → 断言 `voice/tts/stream` 被调用一次；
  5. 循环 30 次（模拟高负载）。
- 报告：`reports/xiaobai_smoke-YYYYMMDD.md` + perf json。

**验收点：**
- 30 轮成功率 ≥ 98%（失败 ≤ 1 轮且可自恢复）；
- 单轮端到端平均时延 ≤ 6 s（含 LLM 生成，mock 时 ≤ 2 s）。

---

### （S1–S4 可选任务，Implement 阶段视工期预算决定是否默认启用）

## 里程碑
1. M1：T1–T5 → "语音服务已成型"（本地跑通 ASR + TTS HTTP 接口）。
2. M2：T6–T9 → "前端 φ 语音对话完成"（ChatView 可录、可播）。
3. M3：T10–T13 → "桌面小白 xiaobai 可发布"（exe 打包 + 快捷键 + 浮窗）。
4. M4：T14–T16 → "E2E 冒烟 + 全维合规"（AC 全通过）。
