# xiaobai_voice — 璇玑系统离线语音 & 桌面小白（xiaobai）AI 助手

> ASR：**Paraformer-zh + sherpa-onnx**（Apache2 / 离线 CPU 最优）
> TTS：**Fish-Speech-S2-Pro**（Research，默认启用 iff 本地权重完整且 license_tier != apache2）
>        ↘ **CosyVoice2**（Apache2，信创/政务默认回退）
>        ↘ **浏览器 SpeechSynthesis**（MessageBubble 旧实现，保留兜底）
> 端口：语音服务默认 **3717**；前端 Vite dev server 走 `/voice` proxy

## 一、快速启动（开发态）

```powershell
# 1. 创建 venv
cd projects\xiaobai_voice
python -m venv .venv
.\.venv\Scripts\Activate.ps1

# 2. 安装：asr + service 必装；tts(=CosyVoice2)、desktop、dev 建议装全
pip install -e ".[asr,service,desktop,dev]"
# （可选）如果要启用 Fish-S2-Pro，手动装：
# pip install fish-speech[s2pro]

# 3. 下载默认模型（ASR Paraformer INT8 + CosyVoice2-0.5B）
python -m xiaobai_voice download --defaults

# 4. 启动语音服务（端口 3717）
python -m xiaobai_voice serve

# 5. 另一个终端：启动桌面小白（浮窗 + 快捷键 + 内嵌 /#/ai WebView）
python -m xiaobai_voice desktop

# 6. 冒烟自检测试
python -m xiaobai_voice selftest
```

## 二、打包发布（windowed，零控制台闪退）

```powershell
.\build_exe.ps1 -UseVenv ".\.venv"
# 产物：dist\Xiaobai\Xiaobai.exe
# 以 Start-Process 双击方式启动验证（避免掩盖 stderr=None 问题）
Start-Process dist\Xiaobai\Xiaobai.exe
Start-Process dist\Xiaobai\Xiaobai.exe -ArgumentList "--selftest-full"
```

## 三、配置位置

Windows：`%APPDATA%\xuanji\xiaobai\config.yaml`
macOS：`~/Library/Application Support/xuanji/xiaobai/config.yaml`
Linux：`$XDG_CONFIG_HOME/xuanji/xiaobai/config.yaml`

启动后可用合规 φ Chip 对话框直接切换 `license_tier=apache2`（默认 auto）。
