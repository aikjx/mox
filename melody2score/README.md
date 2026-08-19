# 哼唱旋律转歌谱应用（melody2score）

嵌入式开发板端「录音/音频 → 提取旋律 → 生成简谱 / musicxml 歌谱」的端到端应用，
并已作为**独立能力域 D13** 挂入项目「信息关联关系图（关图规范 GR-STD-V1.0）」。

- **PC 验证原型**：本目录，直接可跑（支持 mp3 直读 + 现场录音）。
- **开发板移植版本**：见 `board/`。
- **信息图谱融合**：见 `graph/`（领域子图，可被 `tools/info-graph` 直接加载）。

---

## 一、工程结构（对应架构图分层）

```
melody2score/
├── melody2score_demo.py     # PC 入口（参数解析 + 编排）
├── core/                    # 核心分层模块（对应架构图每一层）
│   ├── config.py            # 所有可调参数；Config.pc() / Config.board()
│   ├── capture.py           # 采集层：文件加载 + pyaudio/arecord 录音
│   ├── preprocess.py        # 预处理层：去直流偏移 + 归一化 + 谱减降噪(可关)
│   ├── pitch.py             # 音高检测层（可插拔后端：crepe_onnx / torchcrepe / pyin）
│   ├── analysis.py          # 音乐解析层：BPM + 调式 + 音符分割(含颤音/滑音滤波)
│   ├── score.py             # 歌谱生成层：music21 musicxml + 简谱数字串 + 标准歌谱图片
│   ├── score_sheet.py       # 标准歌谱图片渲染：PNG / PDF / SVG 规范简谱
│   └── pipeline.py          # 编排层：串联各层
├── board/
│   ├── board_config.py      # 开发板调优配置（限核/关降噪/tiny）
│   └── run_board.py         # 开发板入口（alsa 录音 → 转谱）
├── classic_corpus.py       # 经典旋律语料库（公版旋律 + 多音色合成）
├── gen_classic_melodies.py # 合成经典旋律数据集（不同乐器/人声/纯音乐）
├── run_recognition.py      # 真实识别：跑 core 流水线，采集音高恢复精度
├── audio/                  # 合成音频 + manifest.json（ground truth）
├── results/                # classic_results.json（真实识别结果）
├── graph/
│   ├── build_melody_graph.py     # 构建旋律领域 info-graph 子图
│   └── melody_infograph.json     # 生成的子图（info-graph 同构 schema）
├── tests/                  # 离线自测
├── app/                    # 企业级可视化界面（FastAPI 后端 + 零构建前端）
│   ├── webui.py            # 后端 API：识别 + 保存 MD + 导出标准歌谱图片
│   ├── frontend/index.html # 企业级单页：选择/录音→实时简谱+五线谱+标准歌谱→一键导出 PNG/PDF/SVG
│   ├── exports/            # 导出的 Markdown 报告（自动落盘）
│   └── start.ps1           # 启动脚本
├── requirements.txt
├── board_run.sh
└── README.md
```

---

## 二、PC 原型运行

```bash
cd melody2score
python -m pip install -r requirements.txt

# 直接识别 mp3 / wav（默认同时生成标准歌谱 PNG）
python melody2score_demo.py your_song.mp3
python melody2score_demo.py your_song.mp3 -o out.xml
python melody2score_demo.py your_song.mp3 -s my_song_sheet.pdf

# 现场录音
python melody2score_demo.py -r 5

# 可选调参
python melody2score_demo.py song.wav --model small --no-denoise --threads 2

# 不生成图片
python melody2score_demo.py song.wav --no-sheet
```

---

## 二之续、企业级可视化界面（app/）

提供两种形态，复用同一套 `core/` 流水线：

### A. 桌面 GUI（推荐，打开即用，PyQt5）
独立窗口应用：选择音频 / 内置样例 → 后台线程识别 → 直接显示简谱 + 五线谱 + 音高轮廓 + 音符明细，一键保存 Markdown。

```bash
cd melody2score
python -m pip install pyqt5 soundfile          # 核心依赖已具备（PyQt5 5.15）
python app/gui.py                              # 或：powershell -File app/start.ps1
```

功能：
- 输入：文件选择、内置经典旋律样例一键识别、**麦克风实时录音**（点「🎙️ 录音」选 3/5/10 秒，对着麦克风唱即可）。
- 场景模式：**人声模式（唱歌/哼唱，默认）** vs 器乐/通用模式。人声模式自动收窄基频范围（80–1000Hz）、启用 VAD 人声活动检测、加强颤音平滑。
- 参数：模型 tiny/small/full、谱减降噪开关、帧移 hop（板端省内存可调小）。
- 输出（实时、后台线程不卡 UI）：
  - 概要指标（调式 / BPM / 音符数 / 各层耗时）；
  - 简谱数字串（高八度 `.`、低八度 `_`、延音 `-`）；
  - Qt 原生绘制的五线谱（音符头+符干+时值块）；
  - 量化音高轮廓图（音符级阶梯）；
  - 音符明细表（MIDI / 音名 / 起始 / 时长）。
- 导出：`保存 Markdown` 一键生成含「识别概要 + 简谱 + 音符明细 + 算法说明」的标准报告，落到 `app/exports/`。

### B. Web 界面（FastAPI + 原生 HTML/Canvas）
浏览器形态，支持 WebRTC 录音。

```bash
cd melody2score
python -m pip install fastapi uvicorn
python app/webui.py                           # 或：powershell -File app/start.ps1 -Mode web
# 浏览器打开 http://127.0.0.1:8012
```

### 核心算法与优化（精确高效）

流水线分五层（见 `core/`）：采集 → 预处理 → 音高检测 → 音乐解析 → 歌谱生成。
本可视化版本对解析层做了两处企业级优化（已合入 `core/analysis.py`）：

1. **调式识别（estimate_key）提速 + 提准**
   - 旧：对整段音频做 `chroma_cqt`（全曲计算，长曲需 20s+）。
   - 新：优先基于**音符 MIDI 轮廓**（按时长加权统计 12 音级，O(音符数)），仅无音符时回退到 4kHz 降采样信号的 `chroma_stft`（比 cqt 快一个数量级）。
   - 叠加**旋律学先验**：起始音、终止音加倍权重（主音强倾向），纠正「属音被 K-S 误判为主音」的常见错误（如小星星原误判 G 大调，现正确为 C 大调）。
   - 实测：解析耗时从 ~21.5s 降至 ~3s。

2. **简谱八度标记（to_jianpu）修正**
   - 旧：`d = midi - tonic_pc(%12)` 后 `oct_shift = d//12`，把绝对八度误计入，导致中央 C 出现 `.....1` 多点异常。
   - 新：`tonic_midi = note_to_midi(key+"4")`（含八度），`oct_shift = (midi - tonic_midi)//12`、`rel = (midi - tonic_midi)%12`，八度点与音级严格符合记谱习惯。

### 精度要点
- 颤音/滑音：midi 轮廓中值滤波(win=5，人声模式 win=7) + 半音量化 + 短段(<min_note_dur)就近合并，过滤帧抖动毛刺。
- 无声帧：**人声模式额外启用 VAD 人声活动检测**（`core/vad.py`）：以短时帧的「能量门限 + 谱质心范围 + 谱平坦度」三条件判定有声段，再按 `min_voiced_ms` 去毛刺。仅在有声帧上产出音符，彻底排除呼吸、停顿、气声、环境噪声造成的假音高。实测人声模式下基频范围收窄 + VAD 预过滤还能把整段识别耗时降低约 6 倍（后端只需处理有声帧）。
- 置信度门限(0.3)过滤低置信帧。
- 板端部署：tiny + 关降噪 + 限核；PC 高精度用 small/full（torchcrepe/crepe_onnx 后端更快）。

输出示例：
```
简谱： 3 3 4 5 5 4 3 2 1 1 2 3 3 - 2 2 -
[info] BPM=96.0  Key=Cmajor  音符数=16
```

---

## 三、经典旋律数据集（不同旋律 × 不同乐器 × 人声 × 纯音乐）

为了让「真实识别」覆盖**几十个**样例且**无版权风险**，我们用合成方式生成公版经典旋律：

- 旋律：小星星 / 欢乐颂 / 生日歌 / 两只老虎 / 茉莉花 / 致爱丽丝 / 雪绒花 /
  友谊地久天长 / 伦敦大桥 / 玛丽的小羊 / 铃儿响叮当 / 老麦克唐纳 / 划船歌 /
  平安夜 / 红河谷 / 婚礼进行曲（共 16 首，精确标注 MIDI 为 ground truth）。
- 音色三大类：
  - **乐器 instrument**：钢琴 piano / 吉他 guitar(Karplus-Strong) / 弦乐 strings / 长笛 flute / 风琴 organ / 钟声 bell
  - **人声 voice**：human_voice（元音 /a/ 共振峰合成 + 颤音）
  - **纯音乐 pure**：纯正弦 pure_sine / 纯三角波 pure_triangle

每首旋律 × 9 种音色渲染一次 → **144 个样例**（`audio/*.wav` + `audio/manifest.json`）。

```bash
python gen_classic_melodies.py          # 生成 audio/ 下 144 个 wav + manifest
```

> 注：音源为合成（非下载 copyrighted 音频），但**识别是真实的**——音高检测层对
> 每个音频跑真实模型（CREPE tiny / 概率化 YIN），端到端验证旋律恢复能力。

---

## 四、真实识别 + 精度采集

```bash
# 自动选择可用后端（crepe_onnx → torchcrepe → pyin）
python run_recognition.py

# 或显式指定后端（沙箱因 torchcrepe 导入段错误，示例用 pyin）
MELODY_BACKEND=pyin python run_recognition.py
```

脚本调用真实 `core` 流水线（采集→预处理→音高检测→解析），把识别出的音符序列与
`manifest.expected_midi` 比对，逐首统计：

- **音高类准确率 pitch_class_acc**：恢复音高的音名（含八度）命中率
- **音符召回 note_recall** / **音符精确 note_precision**

结果写入 `results/classic_results.json`，并打印按**类别 / 音色**的聚合统计。

---

## 五、信息图谱融合（D13 能力域）

本应用已作为独立需求根 **D13** 注入项目关图：

1. **`guantu.req.json`** 新增 `D13 哼唱旋律转歌谱应用`，并以 `Bind` 边绑定到
   `melody2score/` 各代码/脚本节点（见《关图骨架定义.md》§2.1）。
2. **`graph/melody_infograph.json`** 是旋律领域的完整子图，严格遵循关图规范
   `节点(id,kind,name,path,summary,external)` / `边(id,from,to,kind,label,evidence,external)`
   schema，可直接被 `tools/info-graph` 加载：

```bash
# 构建子图（依赖 results/classic_results.json）
python graph/build_melody_graph.py

# 用真实关图工具校验 / 导出 / 查询
tools/info-graph/target/release/info-graph validate --graph graph/melody_infograph.json
tools/info-graph/target/release/info-graph export   --graph graph/melody_infograph.json --format mermaid
tools/info-graph/target/release/info-graph query    --graph graph/melody_infograph.json --kind Data
```

子图包含的关系（信息图谱关联）：
- `Requirement:D13 --Bind--> CodeFile:melody2score/...`（需求-代码绑定）
- `Business:application/旋律转谱 --Reference--> Business:category/{instrument,voice,pure}`（应用覆盖的分类）
- `Data:melody/<id> --Reference--> Business:category/<cat>` / `Business:timbre/<音色>`（旋律样本 ↔ 内容分类/音色）
- `CodeFile:pipeline.py --ReadWrite--> Data:melody/<id>`（流水线读取旋律音频）
- `Data:melody/<id> --Reference--> Data:melody_result/<id> --ReadWrite--> CodeFile:pipeline.py`（识别结果及出处）
- `CodeFile:pitch.py --Dependency--> Dependency:{torchcrepe|crepe_onnx|librosa}`（音高检测技术栈）

把该子图与 `graph.enterprise.json` 合并即可得到「全项目关图 + 旋律应用域」的统一视图。

---

## 六、流水线分层说明

| 层 | 模块/方法 | 关键实现 |
|----|----------|----------|
| 采集层 | `capture.load_audio` / `capture.record` | librosa 重采样 16k 单声道；pyaudio 或 arecord(alsa) 录音 |
| 预处理 | `preprocess.preprocess` | 去直流偏移 + 归一化 + 谱减降噪（板端可关） |
| 音高检测 | `pitch.PitchDetector` | 可插拔后端：**crepe_onnx tiny**（板端）/ **torchcrepe**（真实 CREPE tiny）/ **pyin**(librosa，兜底) |
| 音乐解析 | `analysis.segment_notes` / `detect_bpm` / `estimate_key` | 状态机切音符 + 中值滤波去颤音 + 短段就近合并(滑音/颤音毛刺) + BPM + Krumhansl 调式识别 |
| 歌谱生成 | `score.to_musicxml` / `score.to_jianpu` | music21 生成 musicxml（调号/速度/量化音符）+ 简谱数字串 |
| 输出层 | `pipeline.run` | 打印简谱 / 存 xml / 可选 MuseScore 出图 |

### 颤音/滑音毛刺过滤
`analysis.segment_notes` 三步处理：
1. **midi 轮廓中值滤波**（`median_win=5`）消除颤音与帧间抖动；
2. 半音量化后按相同音高分段；
3. 把 `< min_note_dur` 的短段（滑音尾音/颤音过冲）合并到音高最近的相邻音符，再过滤仍过短者。

---

## 七、开发板移植（树莓派 4B / 5 / RK3568）

### 关键约束
- 板端用 **crepe_onnx tiny** + onnxruntime（STM32 算力不足，不推荐）。
- `torchcrepe` 在部分 Windows 环境存在 onnxruntime 段错误，仅作 PC 备选；板端仍以 ONNX 为准。
- 树莓派 4B 内存建议 ≥ 2G；RK3568 可上 small / int8 量化。

### 构建与运行
```bash
sudo apt update
sudo apt install -y python3-pip python3-pyaudio portaudio19-dev
python3 -m pip install -r requirements.txt   # 可去掉 torch/torchcrepe 省空间

./board_run.sh record 6     # alsa 录音 6 秒转谱
./board_run.sh file test.wav
python board/run_board.py record 6 -o /tmp/melody.xml
```
开发板默认用 `board/board_config.py`：限 2 核、关降噪、tiny 模型。

### 调优建议
- `OMP_NUM_THREADS`/`intra_op_threads=2`：限核避免占满导致系统卡顿。
- 内存吃紧可去掉 `preprocess` 谱减（`--no-denoise` 或 `enable_denoise=False`）。
- `hop` 从 10ms 提到 20ms 可提速约一倍，精度略降。
- RK3568 带 NPU 时把 Crepe ONNX 做 int8 量化进一步加速。

---

## 八、输出格式
- **简谱文本**：数字 1–7 表音级，`.` 前缀表高八度，`_` 后缀表低八度，`-` 表延音；`#` 为近似离调音。
- **musicxml**：标准可导入 MuseScore / 各打谱软件，含调号、速度标记、量化音符。
