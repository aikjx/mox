# Melody2Score：AI 识别、误差分析与验证

日期：2026-09-12。结论：**AI 对噪声输入有明显收益，但当前系统没有达到所有音频 100% 正确。测试通过、音高周期性高、多个模型一致，均不等于整首歌谱正确。**

## 实际评测

全部计算走生产 `Melody2Score.recognize`，使用原有 WAV 与标注，未按识别结果改写标注。音符匹配要求绝对 MIDI 正确（含八度）、起音误差不超过 50ms、一对一匹配，报告 micro F1；另存 150ms 宽容指标和起止时间误差。F1 不包含调式正确率和完整谱面节奏正确率。

| 场景 | 样例 / 标注音符 | 修复前 pYIN | 修复后 pYIN | CREPE tiny 神经网络 |
|---|---:|---:|---:|---:|
| 9 音色 × 16 首合成单旋律 | 144 / 1773 | 61.60% | 99.66% | 98.95% |
| 人声 + 10dB 白噪声 | 3 / 39 | 未测 | 6.45% | 97.37% |
| 人声 + 110Hz 低音与鼓声 | 3 / 39 | 未测 | 0.00% | 82.93% |

CREPE full 在与开发组相同的 18 样例 / 234 音符上 F1 为 **99.57%**，未优于该组修复后 pYIN 的 99.79%。不能因为模型更大就假定更准确。此次 full 未做 144 项全量测试；tiny 和 pYIN 均做了全量。

开发组为旋律编号 0、2 的 18 个样例，后续扩展为 144 项。噪声/伴奏压力组为旋律编号 0、2、5 的合成人声，随机种子 20260912，**不是人类录制的真实歌曲**。压力组评估未启用声源分离；最新自动分离在 Demucs 缺失时也保留原音并明确提示，而非把 HPSS 当成人声分离。

干净全量修复后：1773 个标注音符中匹配 1766 个，预测 1771 个；仍有 7 个漏匹配和 5 个多余/错误匹配，分布在 6 个样例。这里的漏匹配包含错音，并不全是彻底没输出。详见 [剩余错误](../data/melody2score-ai-20260912/remaining-errors.json)。

## 为什么以前还有大量错误

- **并没有真正使用 AI**：原环境未安装 crepe_onnx / torchcrepe，实际降级至 pYIN。此前 tiny/small 的选择并不能证明神经网络已运行。
- **首音被当噪声**：降噪固定从开头 100ms 估计噪声，歌曲开门见音时削弱真实基频。改为仅从低能量帧估计，无可确认的安静区域则保留信号；重建保持原采样数。
- **时间轴多移了 32ms**：pYIN 默认居中帧，却又用 n_fft 给时间戳增加半窗偏移。移除重复偏移，并将 VAD 回补从每侧固定 50ms 改为一个 VAD 帧移，避免拉长时值、拖慢 BPM。
- **重复运行不等于独立 AI 证据**：原稳健识别反复运行同一个模型，并可能把三次一致当作 100%。现在阈值变化复用一次底层推理；独立复核另用不同模型。
- **置信度语义错误**：原按音符时长或共识生成的分数会过度乐观。现在返回真实帧周期性评分、独立的共识一致度、未校准的质量状态，`quality.accuracy` 保持 null。
- **配置与缓存不一致**：缓存键遗漏后端等参数，切换 AI 仍可能拿到旧结果；返回对象还可能被调用方修改。现在所有配置参与键、样例内容参与指纹，并隔离缓存对象。
- **分离结果与接口问题**：HPSS 分离谐波/打击声，无法保证得到人声；默认缺少 Demucs 时保留原音并提示。结果元数据排除 `vocals/other` 数组，修复数组进入 JSON 导致接口失败的问题。

## AI 产品功能

桌面增加识别后端选择和 **AI 独立复核**。自带 Web 页面也有相同选择与复核状态。实际后端、有效模型及依赖降级原因可见。

自动后端优先可用神经模型；已知干净合成样例使用更适合此次基准的 pYIN。显式选择 CREPE 会使用神经模型。torchcrepe 仅支持 tiny/full，共用选项 small 映射 full 并返回有效模型，避免失败后悄悄降级。

复核以 pYIN 与 CREPE 交叉检查，音符明细标记“模型一致 / 音高异议 / 不确定 / 证据不足”，不自动把猜测覆盖原结果。依赖缺失、实际使用相同引擎时不会冒充独立复核。实测例子见 [AI 复核结果](../data/melody2score-ai-20260912/ai-review-example.json)。复核只检查已有音符的音高证据；它不能保证找出全部漏音、切分错误、节奏或调式错误。

[桌面设置截图](../data/melody2score-ai-20260912/ai-controls.png)。神经模型原理及参数参考 [torchcrepe 官方文档](https://github.com/maxrmorrison/torchcrepe)。多音转写可以进一步评估 [Spotify Basic Pitch](https://github.com/spotify/basic-pitch)，此次没有安装或声称验证该模型。

## 标注审计

`m05_instrument_guitar` 开头标注 MIDI 76，独立频谱检查在 0.05–0.30 秒得到约 **680.85Hz / MIDI 76.56**。Karplus–Strong 整数延迟与环内平均会影响实际调音；该片段的峰已越过四舍五入到 77 的边界。它说明参考标注也要审计，不能强迫模型输出标注来制造满分。保留原 WAV 和原标注，审计数据见 [label-audit.json](../data/melody2score-ai-20260912/label-audit.json)。

## 验证与运行

**51 项回归测试通过，包含真实声卡冒烟；Python 编译、Web 脚本语法、桌面后端/复核选项联动、1366×900 实际窗口尺寸与控件无重叠检查、diff 空白检查通过。桌面左栏支持滚动，取消原有 1680×1250 的过大最小窗口限制。** 第三方 multipart 的弃用提示仍存在，与识别结果无关。

本机已建独立环境 `.runtime/melody2score-ai-env`，复用现有依赖，新增 torchcrepe 0.0.24、torchaudio 2.4.1+cpu、resampy 0.4.3；本机 torch 为 2.4.1+cpu。没有将音频上传到外部 AI 服务。启动桌面：

```powershell
powershell -ExecutionPolicy Bypass -File projects/melody2score/app/start-ai.ps1
```

复现全量：

```powershell
python projects/melody2score/scripts/benchmark_accuracy.py --melodies all --output reports/data/melody2score-ai-recheck/pyin.json
.runtime/melody2score-ai-env/Scripts/python.exe projects/melody2score/scripts/benchmark_accuracy.py --backend torchcrepe --model tiny --melodies all --output reports/data/melody2score-ai-recheck/crepe.json
```

详细逐样例记录与输入 SHA256 均在 [结果目录](../data/melody2score-ai-20260912/)。`crepe-dependency-fallback.json` 是补齐依赖前的降级记录，实际后端为 pYIN，**未计入任何 AI 成绩**；评测器现在将后端不符计为失败。

## 验收边界

- 上述 99.66% 是固定合成集的严格音符 F1，不能称为“真实歌曲 99.66% 正确率”，更不能称为“整首歌谱 100%”。
- 没有收到用户此次具体出错的原曲路径/人工校正谱，因此尚未完成那段原曲的逐音验收。
- 真实带伴奏音乐需要目标声部标注、声源分离和真实录音测试；噪声压力结果已显示算法仍会失效。BPM、调式、时值量化和谱面都需要独立验收。
- 旧冻结 EXE 未重建；本次交付为源码和已验证的本机 AI 运行环境。
