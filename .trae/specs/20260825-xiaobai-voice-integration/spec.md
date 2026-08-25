# 语音对话 + 桌面小白助手「xiaobai」全维最优集成规格

## 一、问题（Problem）

璇玑系统当前 AI 对话页（`/ai`）已实现 **9 动作工具栏 + φ 全维分析布局**，但仍存在三处"离用户最后一公里"的缺口：

1. **无语音入口**：用户必须手动打字才能与专家联盟对话；在多任务并行/驾驶/走动/单手操作/无障碍场景下彻底不可用。现有 Melody2Score 只处理"旋律→乐谱"的人声哼唱 ASR，不能复用为对话语音理解流水线；现有 MessageBubble 朗读只走浏览器原生 Web SpeechSynthesis，音色机械、不支持零样本克隆与情绪标签，远未达到"最好用"。
2. **无常驻桌面入口**：浏览器 Tab 切换、最小化之后，用户无法"一键唤出 AI 助手"；企业运营岗、项目岗、专家岗每天需要在多个业务系统之间跳来跳去，需要一个**系统级悬浮球 + 全局快捷键**随时唤醒 AI。桌面上虽然有系统快捷方式，但缺乏一个可视化、会呼吸、带状态（空闲/听/说/思考）的"小白（xiaobai）"吉祥物浮窗，无法承载"开发专家联盟"品牌形象。
3. **模型选型与协议风险未统一**：根据用户本次提交的开发专家联盟分析结论，**Paraformer-zh（模型）+ sherpa-onnx（推理引擎）** 才是"中文通用 + 离线 CPU 部署"ASR 组合最优；**Fish-Speech-S2-Pro**（非商用）或 **CosyVoice2（Apache2，政务商用）** 为 TTS 最优。项目内 melody2score 目前走 onnxruntime + sounddevice 的 onnx 路径是天然契合点，但**从未上升为独立可复用语音服务**，任意前端/桌面/移动端无法零侵入接入。

本规格以用户提出的三个最高优先级（**页面点击可语音对话 / 添加桌面小白 xiaobai / 最优方法全维集成**）为主线，把 ASR/TTS 引擎选型、部署推理框架、前端麦克风 UI、桌面浮窗、协议回退、打包/缓存/模型下载、快捷键/唤醒等，全部做企业级规范对齐。

---

## 二、目标用户（Users）

| 用户画像 | 典型场景 | 本规格解决痛点 |
|---|---|---|
| 架构/算法/图谱/融合专家 | 手正在写代码/画图，想口述需求让专家联盟先给全维分析初稿 | 过去必须打断手、切 Tab、打字 → 慢 5× |
| 项目经理 / 业务运营 | 手上一堆 IM 消息与会议，想"按下 Alt+X 说一句" → 自动形成任务/项目笔记 | 过去语音全靠手机转写工具，无法直接入图谱/任务/知识库 |
| 政务 / 信创客户（需 Apache2 无风险商用） | 内网离线部署，要求完全 Apache2，不能扛 Research License 法律风险 | Fish-S2-Pro Research License 红线；本规格强制 TTS 自动回退 CosyVoice2 |
| 质量/验收测试 | 验证"语音→生成→朗读"全链路端到端时延、识别 CER、合成 MOS、崩溃率 | 过去无标准冒烟基准，语音能力不可控 |
| 普通用户（品牌感知层） | 桌面常驻一个可爱"小白（xiaobai）"悬浮球，拖拽任意角落，双击进入 AI 对话，全局快捷键录音 | 品牌形象弱、唤出步骤多的老体验 |

---

## 三、目标（Goals）

1. **功能目标**：
   - 一条消息三种入口可语音交互：① ChatView 输入框左侧 🎙 麦克风按钮；② 消息动作栏"追问问语音"；③ 桌面小白浮窗 Alt+X / Alt+S 快捷键。
   - ASR 优先 `Paraformer-zh + sherpa-onnx`，离线 CPU 可跑；TTS 优先 `Fish-Speech-S2-Pro`（Research License 提示开关），自动在**信创模式**（配置 `license_tier=apache2`）下回退 `CosyVoice2`；两者都不可用再回退浏览器 `SpeechSynthesis`（MessageBubble 旧实现）。
   - 桌面浮窗"xiaobai"：悬浮球 + 4 状态（空闲 / 正在听 / 正在想 / 正在说），支持拖拽、吸附屏幕边缘、双击打开 AI 对话页、Alt+Q 关闭、Alt+X 切换录音。
2. **性能目标**：
   - 首包加载后 ASR 冷启动 ≤ 1.2 s（Windows i5-12400，CPU 单核 ≤ 15%）；流式 VAD 端到端延迟 ≤ 300 ms；
   - 短句（≤ 10 字）CER ≤ 5%，长句（≥ 50 字）CER ≤ 8%（普通话、安静办公室环境）；
   - TTS 首 token ≤ 800 ms（Fish-S2-Pro, GPU 16G）/ ≤ 3.5 s（CosyVoice2, CPU）；
   - 浏览器麦克风按 10 ms 帧上送，页面不卡顿（旧卡顿问题硬约束禁止回归）。
3. **稳定/协议目标**：
   - 协议自检面板：页面"全维分析"右侧新增「合规 φ」Chip，用户点击可查看当前 ASR/TTS 引擎 / License / 模型哈希；
   - 所有三方库模型下载必须：SHA256 校验 + 断点续传 + 代理兼容（企业内网代理）+ 失败兜底（切 CPU 量化 INT8 小模型）；
   - 打包态（PyInstaller console=False/windowed）必须：`stdout/stderr` 兜底、`jianpu-ly`/`music21` 警告静默（延续 melody2score 经验）、Python 外部 venv 加载（延续 Experience 1304739 经验）。
4. **体验目标**：
   - 深空 φ 比例视觉统一：录音按钮脉冲动画、浮窗 φ 呼吸阴影、快捷键提示 toast 与旧"φ 徽章"同色系；
   - 无障碍：屏幕阅读器可读 `aria-live=polite`，中英文混读自然；
   - 关键状态可观测：输入框左测实时 CER 不展示（只展示"已听懂 N 字"），避免用户焦虑。

---

## 四、非目标（Non-Goals）

- **不**重新实现 LLM 推理：仍然复用现有 `ChatView` → `backend-node/src/routes/chat.js` 流式生成链路；语音只负责"把说话变成文本输入框字符串"和"把助手回答字符串变成音频播放"两条单向管道。
- **不**训练、不微调、不发布任何 ASR/TTS 权重：只做"最优开源权重的标准化下载 + 推理封装 + 降级回退"；权重本体放在 `projects/xiaobai_voice/models/`（大文件 gitignore）。
- **不**修改旧 melody2score 的 ASR/TTS 后端代码：melody2score 保留哼唱识别，xiaobai_voice 是**独立 sibling 项目**，通过 HTTP/WebSocket 暴露语音能力；二者共享 onnxruntime + sounddevice + numpy 的依赖版本，避免两套冲突。
- **不**接入云端 ASR/TTS API（阿里云/通义/Fish Audio API）作为默认路径：默认走本地离线模型以满足"离线/信创/无外部流量"；API 方式只作为可配置兜底（`voice.engine.strategy`=local_first / cloud_only / cloud_fallback）。
- **不**做唤醒词离线 KWS（"小白小白"）：首版只做 Alt+X 全局快捷键 + 浮窗点击录音；KWS 留给后续 T18 规格（SenseVoice-smal 做 0.2M 权重 KWS 能力可后续叠加）。
- **不**引入 Electron/Tauri 重写"桌面 App"：桌面小白 xiaobai 采用独立 Python 桌面浮窗进程（PySide6——更易和 ASR/TTS Python 引擎共享内存 + 低耦合），主 AI 对话继续通过默认浏览器/内置 QWebEngineView 打开 `http://localhost:3021/#/ai`。两条路径同时支持。

---

## 五、约束、依赖、假设与开放问题

### 5.1 约束
1. **AIS 架构分层**：新组件必须进入 `projects/` 数据目录（`projects/xiaobai_voice`），严禁污染 `platform/` 架构代码；平台侧只新增"语音桥接"一条最小 HTTP/WebSocket 路由。
2. **黄金比例深空色系**：所有新增 UI 必须对齐 `frontend-ui/src/styles/global.css` 设计令牌；按钮尺寸遵循 26/42/68 像素序列；阴影必须多层柔边，禁止硬阴影。
3. **协议合规闸门**：启动时必须读取 `voice.license_tier`（取值：`research` / `apache2` / `auto`），当：
   - `apache2` → TTS 强制走 CosyVoice2；
   - `research` → TTS 走 Fish-S2-Pro，并在 UI 右下角显示 `Research · 非商用` 水印；
   - `auto`（默认）→ 优先检测 Fish 是否可加载（本地有模型权重且非政府/Gov 网络域），否则 CosyVoice2，最后浏览器 TTS。
4. **卡顿硬约束**：前端录音、TTS 流式播放、浮窗动画**禁止**占用主线程 > 16 ms；音频回调全部走独立 Worker / ring-buffer（延续 `melody2score/app/audio_play.py` 的 `_PlaySession` 会话化设计）；禁止在持锁状态调再次取锁。
5. **打包输出**：桌面小白提供一键 PyInstaller 脚本（`projects/xiaobai_voice/build_exe.ps1` + `.spec`），产物必须包含：
   - `_ensure_windowed_streams()` 兜底 sys.stdout/stderr；
   - 外部 venv 加载；
   - 模型路径优先 `<exe同级>/models/` 其次用户目录 `%USERPROFILE%/.xuanji/models/voice`；
   - 启动失败时写结构化日志到 `%APPDATA%/xuanji/xiaobai/logs/`。

### 5.2 依赖（必须可安装在 Windows 10/11 + Python 3.10/3.11）
```text
# ASR：Paraformer-zh + sherpa-onnx（离线 CPU，Apache2）
sherpa-onnx>=1.10.30
# 可选：onnxruntime（如果 sherpa 自带 runtime，允许不重复装，避免版本撞车）
# TTS：Fish-Speech（默认，Research License）+ CosyVoice2（Apache2 回退）
# fish-speech[s2pro]  （若用户允许 research）
# cosyvoice>=0.2.0     （apache2 模式下启用）
# 录音/播放
sounddevice>=0.4.6
soundfile>=0.12.1
numpy>=1.24,<2.0
# 服务：HTTP + WebSocket 音频流
fastapi>=0.110.0
uvicorn[standard]>=0.29.0
websockets>=12.0
pydantic>=2.0
# 桌面浮窗
PySide6>=6.6
# 模型下载（断点续传 + SHA256）
httpx>=0.27.0
tqdm>=4.65
# 全局快捷键
pynput>=1.7.6
# 测试
pytest>=8.0
```

### 5.3 假设
- 最终用户 Windows 平台占比 ≥ 95%；Linux/macOS 只做"能跑"不做快捷键与浮窗美化。
- 默认声卡采样率 44100/48000，可 ASR 侧降采样至 16000；TTS 侧统一 24000/22050 播放。
- 用户本地有 ≥ 8 GB RAM；启用 Fish-S2-Pro GPU 模式时显存 ≥ 16 GB。

### 5.4 开放问题（Specify 阶段允许 3 条，Implement 前必须关闭）
1. **OQ1**：桌面小白浮窗是否默认"开机自启"？默认：关闭，设置面板勾选后启用（写注册表 `Run` 键值 `xiaobai`，非管理员）。
2. **OQ2**：ASR 流式分片是否走 WebSocket（`ws://host:3717/ws/asr/stream`）或 HTTP `/voice/asr/full` 一次性上传？默认：**流式优先**，WebSocket 不可用自动降级 HTTP 一次性上传 16kHz WAV。
3. **OQ3**：TTS 是否支持"句子级流 + 边生成边播放"（避免等 10 秒整段合成完再播）？默认：**支持**，Audio 元素 `MediaSource` / `MOQ` 容器 + `appendBuffer`；失败降级完整 WAV 下载后播放。

---

## 六、功能需求（Functional Requirements，FR）

### FR1 ChatView 麦克风输入 UI
- 输入框**左侧**追加 **φ 主色 麦克风按钮**（悬停高亮、录音中红脉冲 + "正在听·3.2s"）。
- 下拉菜单（点击右侧▼）列出 3 项：
  1. 按住说话（松手识别，默认）
  2. 点击开启/点击结束（长录音）
  3. 选择麦克风设备 + ASR 引擎状态
- 录音实时显示**声波条**（12 根 φ 宽度的竖条，随 Rms 能量缩放），不要 WebGL，纯 CSS transform 即可。
- 识别文本**实时回填**到输入框（可编辑）。VAD 自动断句后追加"回车即发送"提示；用户回车直接走现有发送管线。
- 快捷键：聚焦输入框时，`Alt + V` = 开关录音（和桌面浮窗 Alt+X 区分）；`Esc` = 取消并丢弃录音结果。
- 当麦克风不可用/权限被拒：按钮灰化并弹 toast"请在浏览器允许麦克风权限，或改用桌面小白 xiaobai"。

### FR2 语音转"自动发送"策略
- 用户可切换（在麦克风下拉设置中）：
  - `manual`（默认）：识别结果回填输入框，**不自动发**；
  - `auto_send_after_silence_800ms`：VAD 检测到 ≥ 800 ms 静音 → 自动发送；
  - `auto_send_on_stop_click`：手动点停止后立刻发送。
- 自动发送前追加 1.5 s 可撤销 toast（"1.5 秒后发送 · Ctrl+Z 撤销"）。

### FR3 单条消息"追问→语音"快捷入口
- 现有 `MessageBubble` 的「追问」按钮下拉增加一项"**用语音追问**"（麦克风小图标）；点击：先回填默认追问前缀 "关于以上内容，我想追问：" 到输入框，再自动激活麦克风录制后半句。

### FR4 朗读（TTS）扩展实现三层回退
- 现有 `MessageBubble` 朗读图标动作：
  - 第 1 层：**本地 Fish-S2-Pro**（若存在 `voice.license_tier != apache2` 且模型权重完整）→ 请求 `/voice/tts/stream?voice=fish_s2&text=…` → 边合成边播放；
  - 第 2 层：**CosyVoice2（Apache2）** → `/voice/tts/stream?voice=cosyvoice2`；
  - 第 3 层：**浏览器 SpeechSynthesis**（现实现，保留）；三层失败才弹 warning toast"朗读不可用"。
- 支持：
  - 下拉选择音色（Fish 默认 3~5 秒克隆的"小白"默认音，可上传 3-5 秒 wav 做零样本克隆，克隆结果保存在 `models/voice_clips/` 哈希目录，按 SHA1 去重）；
  - 朗读速率 0.8× ~ 1.4×，下拉在朗读按钮气泡菜单里；
  - 情绪标签（Fish）：`中性 / 高兴 / 悲伤 / 严肃`，映射到 `<|zhappy|>` 等离散 token 前缀；CosyVoice2 走指令微调。

### FR5 后端语音服务（`xiaobai-voice-service`，独立 Python FastAPI）
- 监听端口 **3717**（3 = "小"，7 = "白"，1 = "一" → 谐音 3 7 1 7 预留端口；不可冲突现有 3001/3010/3020/3021；在 `vite.config.js` 中新增 `/voice` proxy 到 3717）。
- 路由矩阵：
  | Method | Path | 说明 |
  |---|---|---|
  | GET | `/voice/health` | 状态 + 引擎版本 + license_tier + 模型哈希 |
  | GET | `/voice/models` | 返回当前已下载/已加载/可下载模型清单 |
  | POST| `/voice/models/download` | 触发后台下载（断点续传 + SHA256 + 进度 SSE） |
  | WS  | `/voice/ws/asr/stream` | 流式 ASR：浏览器 16bit PCM 16k 二进制帧 → 服务端 sherpa-onnx 流式识别 → 实时 partial 文本 JSON |
  | POST| `/voice/asr/full` | 一次性 WAV/WebM 上传 → 完整文本 + 时间戳 + 置信度 |
  | GET | `/voice/tts/stream` | query: `text, voice, emotion, speed` → 返回 `audio/wav` 或分段 `audio/mpeg` 字节流 |
  | POST| `/voice/tts/clone` | multipart 上传 3-5s wav → 保存新克隆音色 id |
  | POST| `/voice/hotwords` | 设置热词表（Paraformer ITN/热词增强） |
- 生命周期：启动时自检（3 条测试短句 + 2 个合成短句），输出到 `logs/smoke_YYYYMMDD.jsonl`。
- 错误分类（延续 Experience 1304739 失败经验）：
  - `MISSING_DEP` ImportError → 提示解释器/外部 venv 注入；
  - `MISSING_MODEL` FileNotFoundError → 提示到设置面板一键下载；
  - `DLL_LOAD_FAIL` OSError → 显式调用 `os.add_dll_directory` / PATH 注入并给出日志；
  - `GPU_OOM` RuntimeError → 自动降到 INT8 CPU 模式。

### FR6 Paraformer-zh + sherpa-onnx 封装（ASR）
- **模型组合**（延续分析结论）：
  - 主：Paraformer-zh 非流式 INT8（`sherpa-onnx` 官方 `csukuangfj/sherpa-onnx-paraformer-zh-int8`）；
  - 可选流式模型：SenseVoice-small（当语音中出现 90%+ 非中文，自动切换多语种识别，返回语种标签）。
- 内置 VAD：sherpa-onnx 自带 `silero-vad`（frame=32ms，speech_th=0.5）；不要 webrtcvad（依赖 DLL 兼容问题更多）。
- 热词：`POST /voice/hotwords` body 为 `{ words: [ { word: "璇玑", score: 3.0 }, … ] }`；服务端保存到 `config/hotwords.json`，重启仍生效。
- 首条冷启动优化：进程启动时先跑 1 句"你好，小白"的 30ms 预热推理，避免用户首句被吞（类似 melody2score audio_play 预充水位）。

### FR7 TTS 双引擎封装（Fish / CosyVoice）
- Fish-S2-Pro（Research）：
  - 检查权重目录 `models/fish-speech-s2-pro/`；不存在或用户勾选 `strict_apache2=true` → 自动降级；
  - 零样本克隆：接口只保存 3~5s 参考音频 hash，不发送任何原始音频到云端；
  - 情绪标签：严格映射 `emotion ∈ {neutral, happy, sad, serious}` → Fish 内部标签；
- CosyVoice2（Apache2）：
  - 指令模式优先（`你现在是璇玑小白，请用温暖自然的中文朗读：{text}`）；
  - 流式 chunk 250ms 一段，避免首包延迟过大；
- 播放管线（前端）：Web Audio + SourceBuffer（Opus）优先，Audio.srcObject 流兜底；不要 `<audio>` 拉完整 10MB 文件一次性。

### FR8 桌面悬浮球「xiaobai」（Python + PySide6）
- 外观：φ 圆形（68px 默认，可缩放到 42/68/110 三档），深空渐变背景 + 小白吉祥物 SVG 头像 + 4 状态呼吸灯外环（idle 青、listen 绿脉动、think 紫旋转、speak 靛蓝波形）。
- 行为：
  - **拖拽**：鼠标左键按住可拖到屏幕任意位置；释放后磁吸到最近边缘（左右二选一，吸附动画 300 ms φ 曲线）；
  - **单击**：切换录音；再次点击 → 结束录音 → 发送到 AI 对话 → 自动朗读回答（若已开启「自动朗读回答」）；
  - **双击**：打开 AI 对话页（默认浏览器）；若已启动内置 QWebEngineView 模式，则把浮窗主窗口带到前台并聚焦 `/#/ai`；
  - **右键菜单**：
    1. 打开 AI 对话
    2. 音色设置（展开 Fish/CosyVoice/浏览器 三层 + 情绪 + 速率）
    3. 模型管理（下载/删除 ASR、Fish、CosyVoice 权重；大小/哈希/协议显示）
    4. 合规面板（License Tier：Research / Apache2 / Auto；切换后自动重启语音子进程）
    5. 开机自启（勾选写 HKCU Run）
    6. 全局快捷键：
       - `Alt+X` = 切换录音（可配置）
       - `Alt+S` = 朗读当前剪贴板文本
       - `Alt+Q` = 退出 xiaobai
    7. 退出
- 主窗口（双击 / 右键打开 AI 对话）：
  - 内嵌 QWebEngineView 打开 `http://localhost:3021/#/ai`（若端口被占用/服务未启动 → 显示"服务未启动 φ"并给出启动按钮）；
  - 顶部 4 个 Chip：`ASR引擎 · 在线` / `TTS引擎 · Fish` / `合规 · Apache2` / `快捷键 · Alt+X`；
  - 底部输入区与 Web 版一致，但额外支持"粘贴音频文件立即转文本"。

### FR9 全局快捷键（pynput）
- 默认键位如 FR8。监听在独立线程，不要阻塞 UI 主线程。
- 触发时在屏幕右下角弹 1.8s toast（和 Web 版同色系），文字："🎙 正在听 · 说话结束按 Alt+X 停止" / "🛑 已识别并发送" / "📣 正在朗读剪贴板"。

### FR10 模型下载中心（桌面浮窗 + `/voice/models` 路由）
- 模型元数据定义在 `projects/xiaobai_voice/config/models.yaml`（随仓库发布，SHA256 锁定）：
  | id | name | size_mb | license | url | sha256 | fallback? |
  |---|---|---|---|---|---|---|
  | `asr-paraformer-int8` | Paraformer-zh INT8 (sherpa-onnx) | ~130 | Apache2 | ModelScope / HuggingFace | … | default |
  | `asr-sensevoice-small` | SenseVoice small 多语种 | ~220 | MIT | … | … | optional |
  | `tts-fish-s2-pro` | Fish-Speech S2 Pro 权重 | ~3800 | Research | Fish Audio GitHub Releases | … | default（非商用） |
  | `tts-cosyvoice2-0.5b` | CosyVoice2 0.5B | ~1100 | Apache2 | Alibaba HuggingFace | … | fallback |
- 下载流程：断点续传（Range）、失败 3 次重试指数退避、SHA256 校验失败自动回删；下载进度通过 SSE `/voice/models/download/stream?id=…` 推给桌面浮窗进度条。
- 启动检测：缺失默认模型 → 弹出"下载 φ 向导"，用户可一键"下载全部默认"；下载中 ASR/TTS 不阻塞 UI，UI 显示"ASR · 下载中 42%"。

### FR11 协议/合规 φ Chip
- 顶栏 `analysis-stages` 最右侧追加一个静态 Chip：**合规 φ（license_tier）**。
- 点击打开对话框：列出当前引擎、模型 ID、SHA256、License 全文超链接、License Tier 切换（Auto / Research / Apache2）。切换 Apache2 后若 Fish 已下载，则提示"已降级，但旧模型文件仍保留，可手动删除。"

### FR12 配置中心
- 配置文件位置：`%APPDATA%/xuanji/xiaobai/config.yaml`（Windows）、`~/Library/Application Support/xuanji/xiaobai/config.yaml`（mac）、`$XDG_CONFIG_HOME/xuanji/xiaobai/config.yaml`（Linux）。
- 必填字段：
  ```yaml
  voice:
    port: 3717
    strategy: local_first          # local_first / cloud_only / cloud_fallback
    license_tier: auto             # auto / research / apache2
    asr:
      engine: sherpa_paraformer     # sherpa_paraformer / sensevoice / auto
      hotwords: []
      vad_threshold_ms: 800
    tts:
      engine: auto                  # fish_s2 / cosyvoice2 / browser / auto
      default_voice: xiaobai
      clone_reference: null         # 3-5s wav 的 hash id
      emotion: neutral
      speed: 1.0
      auto_read_assistant: true     # 自动朗读回答
    ui:
      float_ball_size: 68           # 42 / 68 / 110
      float_ball_pos: [right, 120]  # edge + offset_y
      auto_start: false
      shortcuts:
        toggle_record: Alt+X
        read_clipboard: Alt+S
        quit: Alt+Q
    logging:
      level: INFO
      path: null                    # 默认 %APPDATA%…/logs
  ```
- 变更保存后热更新：license_tier/tts.engine 切换时，语音引擎子进程优雅重启（≤ 3s），UI toast 提示"已切换 φ"。

### FR13 会话化播放管线（卡顿硬约束）
- TTS 播放、浮窗语音提示全部复用 `_PlaySession` 模式（参考 melody2score 经验）：每次播放新建 ring + 流 + 事件，旧生产者线程只写旧 ring 自然消亡；**严禁**在持锁状态下调用会再次取锁的方法（`stop()` 与 `play()` 互相嵌套曾致 GUI 主线程永久死锁，必须禁止）。
- 采样率必须按调用传入创建（不能固定 16k/22k/24k），避免合成 24k Fish 音频用 22k PortAudio 流播放导致变调。
- `--selftest-full`（桌面端命令行参数）必须包含"语音播放冒烟（死锁回归）"，在声卡不可用时 `PortAudioError` 必须安全降级跳过。

### FR14 打包发布（PyInstaller windowed）兜底
- 入口 `xiaobai/cli.py` 最前置 `_ensure_windowed_streams()`：
  - `sys.stdout` / `sys.stderr` 为 None 时，用 `StringIO` + 文件句柄替换；
  - 注入 `os.add_dll_directory` 覆盖 `onnxruntime/capi` / `numpy/.libs` / `PySide6/Qt6/bin` / `sounddevice` / `_sounddevice_data`；
  - 注入 ESPEAK_DATA_PATH（若 cosyvoice 需要）/ FISH_SPEECH_CKPT_DIR / COSYVOICE_CKPT_DIR 环境变量。
- 第三方 `jianpu-ly`（若未来复用）/ `music21`（若未来乐谱）stderr 警告必须单条捕获写入诊断日志，不可打印到 `stderr`（避免 AttributeError）。
- 打包后验证必须以 `Start-Process` 方式双击运行，禁止从控制台启动（因为控制台启动 stderr 有效，会掩盖问题）。

### FR15 前端与服务桥接：Vite proxy + CORS
- `vite.config.js` 新增：
  ```js
  '/voice': {
    target: 'http://localhost:3717',
    changeOrigin: true,
    ws: true,   // 流式 ASR WebSocket
  }
  ```
- 前端 `src/api/index.js` 新增 `voiceApi`：封装 health、models、download SSE、stream WS、asr full、tts stream、clone、hotwords。
- 若 3717 无响应（xiaobai 未启动）→ ChatView 录音按钮灰化并在麦克风下拉提示"启动桌面小白以启用语音能力"。

---

## 七、非功能需求（Non-Functional Requirements，NFR）

### NFR1 稳定性
- 连续 24 小时高并发（每 30s 一次 ASR + 一次 TTS）崩溃率 = 0；内存泄漏小时增速 ≤ 10 MB（稳态后）。
- 录音进程崩溃自动重启（浮窗 toast 提示"语音服务重启"）。
- PyInstaller 打包后 `windowed` 模式：**零 stderr/None AttributeError**。

### NFR2 性能
- ASR 流式首字延迟 ≤ 300 ms；短句 RTF ≤ 0.08（1 秒音频在 80 ms 内识别完）。
- TTS 首字延迟：Fish ≤ 800 ms / CosyVoice2 CPU ≤ 3.5 s / Browser ≤ 120 ms。
- 浮窗 CPU 占用（idle）≤ 0.3%；录音/播放 ≤ 2%（四核 i5-12400 度量）。

### NFR3 可用性 / 无障碍
- 键盘可达：浮窗打开后，所有设置项 Tab/Shift+Tab 可达；快捷键全部可重映射。
- 屏幕阅读器：输入框录音按钮、浮窗状态变更都有 `aria-live=polite` 播报。
- 色盲友好：状态色同时辅以形状（listen=外环圆点、think=旋转环、speak=波形）。

### NFR4 安全与合规
- License Tier 在 **apache2** 模式下：**禁止 import fish_speech**（否则即使未调用，也可能在打包发布时造成许可证污染）。启动自检写入启动日志。
- 语音数据默认**不落地**：ASR/TTS 临时音频 10 min 内自动从 `%TEMP%/xuanji_voice/` 清掉；只有用户点击"保存录音"才写文件。
- 快捷键监听：不收集任何非目标键字符；只捕获组合键事件对象，**不 record 按键序列**。

### NFR5 可观测
- 结构化日志：每条 ASR/TTS 记录耗时、CER（有 ground truth 时）、引擎、模型 ID、模型 SHA、内存占用、CPU%。
- `/voice/metrics` 暴露 Prometheus 风格指标：
  - `voice_asr_requests_total{engine,status}`
  - `voice_asr_cer_bucket{le}`（可选项）
  - `voice_tts_first_token_latency_seconds_bucket{engine,le}`
  - `voice_tts_total_duration_seconds{engine}`

### NFR6 可维护性 / 架构
- 新工程 `projects/xiaobai_voice` 模块化：
  - `xiaobai_voice/asr/`（sherpa、sensevoice 后端，接口统一）
  - `xiaobai_voice/tts/`（fish、cosyvoice、browser 后端，接口统一）
  - `xiaobai_voice/service/`（FastAPI + WebSocket）
  - `xiaobai_voice/desktop/`（PySide6 浮窗 + 快捷键 + 内嵌 WebView）
  - `xiaobai_voice/models/`（模型存储 + 下载器）
  - `xiaobai_voice/config/`（models.yaml + 默认 config.yaml）
  - `xiaobai_voice/tests/`（单元 + 冒烟）
- 前后端接口版本化：`/voice/v1/*`，v0 路径做兼容别名到 v1。

### NFR7 视觉一致性（φ × 深空）
- 新增所有 Vue 组件颜色严格走 `--ds-*` / `--accent-*` 令牌。
- 按钮/浮窗圆角统一 18px（φ 派生：11→18→29→47）。
- 录音脉冲 1.2s 周期、TTS 播放阴影 1.4s 周期，和旧 MessageBubble 呼吸灯同节奏。

---

## 八、验收标准（Acceptance Criteria，AC）

> AC 类型只有 **rule**（可观测二元通过）与 **rubric**（评分维度）。

### 功能类（Rule）

- **AC1 rule**：ChatView 输入框**左侧**存在 1 个「麦克风」按钮（`[title*="麦克风"], .voice-mic-btn`），且按钮 title 在 2 种状态（idle/recording）下文案不同。证据：`browser_evaluate` DOM 探针。
- **AC2 rule**：麦克风下拉菜单存在"按住说话 / 点击开启结束 / 设备与引擎"三项。证据：浏览器打开下拉后 DOM 探针。
- **AC3 rule**：当未启动语音服务时，麦克风按钮灰化且设置面板出现"启动桌面小白以启用语音能力"提示。证据：关闭 3717 端口后探针 + 提示文本出现。
- **AC4 rule**：前端通过 WS 或 HTTP 成功上传 1 秒 16kHz PCM 中文"你好小白"，并在 1.5 s 内返回文本包含"你好"或"小白"两个词之一。证据：`tests/voice_smoke.spec.js`（Playwright）输出。
- **AC5 rule**：ASR `engine = sherpa_paraformer`，启动日志中出现 `sherpa-onnx paraformer-int8 ready in <1200 ms`。证据：`logs/smoke_*.jsonl`。
- **AC6 rule**：切换 `license_tier=apache2` 后，`/voice/health` 返回 `tts.engine=cosyvoice2` 且日志**不包含** `import fish_speech`（grep 为负）。证据：grep 日志 + 健康响应。
- **AC7 rule**：TTS `/voice/tts/stream?text=你好璇玑` 请求返回 `audio/*` Content-Type，字节长度 ≥ 20 KB，可被 `soundfile.read` 成功解码为 ≥ 0.3 s 语音。证据：pytest。
- **AC8 rule**：桌面浮窗进程启动后，右下角出现 68 px 圆形 `xiaobai-ball` 窗口，始终置顶（`WindowStaysOnTopHint`）。证据：PySide6 `QWidget.windowFlags()` 断言。
- **AC9 rule**：全局快捷键 `Alt+X` 按下后，浮窗状态从 idle → listen；松开再次按变为 idle，并触发一次 ASR 请求。证据：pynput 仿真 + `/voice/asr/*` 计数 +1。
- **AC10 rule**：Alt+X 录音后，浮窗完成识别后自动把文本发送到 `/ai` ChatView（桌面内置 WebView 或默认浏览器），且文本出现在聊天历史。证据：DOM 探针最后一条 `mb-user` innerText 含识别文本。
- **AC11 rule**：FR1 声波条在录音时出现（12 根竖条，高度动态变化）；停止录音 800 ms 内消失。证据：Playwright 截图 diff。
- **AC12 rule**：模型下载失败（断网 / SHA256 不匹配）时，模型目录不保留半坏文件；重试 ≥ 3 次指数退避且日志可审计。证据：`tests/test_models_download.py` 伪造 404 / 坏 sha 两种场景。
- **AC13 rule**：打包态 `windowed`（Start-Process 双击方式启动）启动时，**零 stderr None AttributeError 异常**。证据：启动后 30 s 读日志文件，无匹配。
- **AC14 rule**：启动 `--selftest-full` 命令包含"语音播放冒烟（死锁回归）"子项；无声卡时跳过但不崩溃。证据：`selftest-report.jsonl` 含 `voice_playback_smoke_deadlock_regression` 记录。
- **AC15 rule**：合规 φ Chip 点击打开对话框，列出 ASR/TTS 引擎 + license_tier + 模型 sha256。证据：Playwright 打开对话框 DOM 文本包含三者。
- **AC16 rule**：vite.config.js `/voice` proxy 存在 `ws: true`；`curl /voice/health` 返回 HTTP 200 并含 `engine` 字段。证据：命令 + 配置 grep。

### 质量类（Rubric）

- **AC17 rubric**：时延（ASR + TTS）。0–3 分，阈值 ≥ 2。
  - `3`：ASR 首字 ≤ 300 ms & RTF ≤ 0.08；TTS Fish 首字 ≤ 800 ms。
  - `2`：ASR 首字 ≤ 450 ms & RTF ≤ 0.12；TTS Fish ≤ 1200 ms 或 CosyVoice2 CPU ≤ 3500 ms。
  - `1`：ASR 首字 ≤ 900 ms 或 TTS 首字 > 5 s。
  - `0`：无法完成端到端链路。
  - 证据：`tests/perf_asr_tts.py` 的 benchmark.json。
- **AC18 rubric**：识别/合成质量。0–3 分，阈值 ≥ 2。
  - `3`：20 条普通话短句（≤10 字）CER ≤ 5%；TTS MOS 盲测 ≥ 4.2/5（5 人打分，≥3 人达标）。
  - `2`：20 短句 CER ≤ 8%；TTS MOS ≥ 3.8/5。
  - `1`：CER > 12% 或 MOS < 3.5/5。
  - `0`：无法识别或合成出故障。
  - 证据：`tests/cer_mos_20_cases.jsonl` 报告。
- **AC19 rubric**：视觉一致性（φ + 深空）。0–3 分，阈值 ≥ 2。
  - `3`：所有新增按钮/浮窗颜色、圆角、间距在 global.css 令牌内，φ 阴影 + 呼吸节奏对齐旧动作栏；无障碍可达 100%。
  - `2`：基本对齐但 1–2 处圆角/色值例外。
  - `1`：和深空主题冲突 ≥ 3 处。
  - `0`：视觉混乱。
  - 证据：专家 Review + 截图对比。
- **AC20 rubric**：协议合规与打包稳定性（企业级）。0–3 分，阈值 ≥ 2。
  - `3`：`license_tier=apache2` 下 Fish 代码未被 import（AST 扫描 + 打包文件列表 grep 双负）；windowed 启动 0 报错。
  - `2`：apache2 模式下 Fish 不被调用（但 import 语句被静态分析命中一次，但被 `__import__` 延迟封装可证明无副作用）。
  - `1`：windowed 模式偶尔有 stderr 警告但不崩溃。
  - `0`：打包后启动崩溃或 license 冲突。
  - 证据：AST 扫描报告 `reports/xiaobai_license_scan-YYYYMMDD.md` + 打包日志。
- **AC21 rubric**：卡顿/死锁安全（延续前规格硬约束）。0–3 分，阈值 ≥ 3。
  - `3`：`--selftest-full` + Playwright 30 轮并发"录音+回答朗读"循环无主线程阻塞；死锁专项测试 100 轮 0 失败。
  - `2`：30 轮中 1 次自恢复卡顿 (< 2 s)。
  - `1`：≥ 2 次自恢复卡顿。
  - `0`：出现永久死锁 / GUI 无响应 > 5 s。
  - 证据：`tests/test_deadlock_safety.py` 循环报告。

---

## 九、验收交付物（Deliverables）
1. 代码：`projects/xiaobai_voice/…` 独立模块 + `vite.config.js` proxy + `frontend-ui/src/views/ChatView.vue` 录音 UI + `MessageBubble.vue` TTS 三层回退。
2. 规格 & 任务：本 `spec.md` + 对应 `tasks.md`。
3. 报告：`reports/xiaobai_smoke-YYYYMMDD.md`、`reports/xiaobai_license_scan-YYYYMMDD.md`、`reports/xiaobai_perf-YYYYMMDD.json`。
4. 打包脚本：`build_exe.ps1`、`build_exe.spec`，输出：`dist/xiaobai/Xiaobai.exe`（windowed）。
5. 可验证 URL：
   - AI 对话页：`http://localhost:3021/#/ai`
   - 语音健康：`http://localhost:3717/voice/health`
