# Melody2Score mox 模块化系统架构维度稳定性 / 性能优化 · 企业级验证报告

> 日期：2026-08-18
> 模式：开发专家联盟处理模式（mox 模块化系统架构维度系统性优化）
> 验证工具：`tests/verify_all_dims.py`（无声卡 / 无 torch 依赖，CI 与打包后 green 环境可直接跑）

---

## 一、优化项与修复映射

| 维度 | 问题根因 | 修复位置 | 修复方式 |
|------|----------|----------|----------|
| 播放引擎性能 | `RingBuffer.write/read` 逐字节循环拷贝，`enum` + 单字节赋值 | `app/audio_play.py` | 改用 `memoryview` 批量切片拷贝（跨边界自动拆段） |
| 合成性能 | 每音符重建 `np.arange`×3 + 多次 `astype` + `librosa.tone` 解析开销 | `core/synth.py` | 单次向量化合成；时间轴/衰减曲线按 `(n,sr)` 缓存 |
| 识别性能 | robust 模式（4 次重识别）每次循环新建 `PitchDetector`，重复加载 ONNX 模型 | `core/pipeline.py` | 循环外复用单一 `PitchDetector` 实例 |
| WebUI 性能 | WebUI 已解码 `(y,sr)` 又重打包 wav 再让编排器解码（二次编解码往返） | `app/webui.py` + `core/pipeline.py` | 新增 `"array"` 音源类型，直接复用已解码数据 |
| WebUI 并发 | 同步路由长识别阻塞 FastAPI 事件循环 | `app/webui.py` | 改为 `async` + `anyio.to_thread.run_sync` 线程池执行 |
| GUI 卡顿 | 选文件时 `open().read()` 被调用 2 次（2× I/O + 双解码）；试听解码在 UI 线程同步做 | `app/gui.py` | 只读一次文件共用字节；试听解码延迟到点击时 |
| 进度反馈 | CLI/长任务无进度，观感"假死" | `core/pipeline.py` + `melody2score_demo.py` | 新增 `progress_cb` 分阶段钩子 + CLI `tqdm` 实时进度 |
| 异常安全 | 声卡启动失败残留半成品状态；`_underruns` 回调早于 `play()` 触发 `AttributeError` | `app/audio_play.py` | 启动失败 try/except 清理；`_underruns` 提前初始化 |
| 打包一致性 | spec 把 exe 放 `app/Melody2Score`，但 `build_exe.py` 校验/启动器指向根目录（矛盾→找不到 exe） | `build_exe.py` | 校验与启动器路径统一为 `app/Melody2Score.exe` |

---

## 二、mox 模块化系统架构维度验证结果（`tests/verify_all_dims.py`，18/18 PASS）

```
[P1] RingBuffer 批量 memoryview 拷贝
  [PASS] write 满容量返回实际字节
  [PASS] read 超容量截断到实际
  [PASS] 跨边界读写内容一致
  [PASS] 多段读写顺序无损
  [PASS] 10MB 批量写 < 1.0s（有消费者排空）

[P2/P3] synth_piano 向量化性能 + 确定性
  [PASS] 合成确定性(相同输入相同输出)
  [PASS] 输出为 float32 且峰值<=1
  [PASS] 时间轴缓存命中(同参数仅1条)
  [PASS] 200 次合成 < 100ms

[S1/S2] RingBuffer 边界 & 满缓冲超时保护
  [PASS] 满缓冲写超时(不阻塞)
  [PASS] 空缓冲读返回空字节

[C1] 并发读写安全（多线程写 + 读，无死锁 / 无交叉错乱）
  [PASS] 并发读写为无死锁完成
  [PASS] 各生产者数据完整不丢失

[A1] pipeline._load_source('array') 免二次编解码
  [PASS] array 源直接复用 y
  [PASS] array 源保留采样率
  [PASS] 未知源类型抛错(契约)

[A2] recognize/run progress_cb 分阶段回调
  [PASS] run() 支持 progress_cb 参数
  [PASS] recognize() 支持 progress_cb 参数
```

### 关键量化指标
- **播放缓冲写入吞吐**：10MB 批量写 < 1.0s（memoryview 批量拷贝，消除逐字节瓶颈）。
- **音符合成**：200 次合成 < 100ms，时间轴缓存使同参数重复合成零冗余计算。
- **并发安全**：8 线程并发写 + 单线程读，每生产者 5000 字节完整无丢失、无交叉。
- **确定性**：合成与缓存完全可复现（相同输入位级一致），支持稳定缓存与测试。

---

## 三、打包验证（build_exe.py / build_exe.spec）

- `build_exe.py --help` 正常解析（语法 / 依赖导入无误）。
- 修复 spec 与启动器路径不一致：spec 将 exe 置于 `dist/Melody2Score/app/Melody2Score.exe`，
  `build_exe.py` 校验与 `build_exe.ps1` 启动器现已对齐该路径（原 `build_exe.py` 指向根目录会导致
  产物校验失败 exit 3 + 启动器找不到 exe）。
- `tests/verify_all_dims.py` 不依赖 torch / 声卡，可纳入打包后 smoke test：
  `dist/Melody2Score/app/python.exe tests/verify_all_dims.py`（green 环境直接验证核心修复）。

---

## 四、运行验证套件

```bash
# 源代码环境
python tests/verify_all_dims.py

# 打包后 green 环境（示例路径）
dist/Melody2Score/app/Melody2Score.exe  # 启动 GUI
# 或后台 smoke test：
python tests/verify_all_dims.py
```

退出码 0 = mox 模块化系统架构维度通过；1 = 存在失败项（对应维度名会列出）。

---

## 五、结论

mox 模块化系统架构维度（性能 / 稳定性 / 并发 / 确定性 / API 契约 / 打包一致性）优化已完成并通过企业级
验证套件，所有对外契约（CLI 参数、GUI 行为、WebUI API、测试接口）保持不变。现有
`tests/test_audio_play.py` 与 `tests/test_pipeline.py` 契约不受影响。
