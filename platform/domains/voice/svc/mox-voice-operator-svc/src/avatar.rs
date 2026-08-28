// Copyright (c) 2026 璇玑 RelGraph · 小白形象模型规范 (AIS-FR14/V1.0)
// Licensed under the MIT License.

//! # 小白 · 形象模型注册表（Avatar Registry）
//!
//! 全维形象模型规范 **AIS-FR14/V1.0**：把伙伴的**视觉(visual) + 语音(voice) + 性格(persona)**
//! 三维参数化为一个 JSON 模型文件，支持无限个模型注册与运行时一键切换。
//!
//! ## 模型文件
//! 每个模型一个 JSON：`models/avatars/<id>.json`（默认扫描该目录，跳过 `*.schema.json`）。
//! 结构：
//! ```json
//! {
//!   "id": "xiaobai-white",
//!   "name": "小白·纯白",
//!   "version": "1.0.0",
//!   "desc": "默认形象：白色 Q 版圆润小人",
//!   "voice":  { "tts_sid": 45, "speed": 1.0, "pitch": 1.0 },
//!   "persona":{ "self_name":"小白", "style":"soft", "greeting":"你好呀，我是小白～",
//!               "speak_prefix":"", "speak_suffix":"～", "tone_words":["呢","呀"], "temperature":0.8 },
//!   "visual": { "render_mode":"xiaobai", "body_color":"#f4f4f4", "accent":["#4fa3ff","#7b5cd6"],
//!               "eye_glow":"#ffffff", "particle_count":140, "material":"matte", "scale":1.0,
//!               "state_colors": { "idle":"#7fd4c1","listen":"#ff5a5a","think":"#4a6cf7","speak":"#34d17b","executing":"#ffb340" } }
//! }
//! ```
//!
//! ## 切换
//! - 文字对话：`切换模型 xxx` / `变成 xxx` / `换装 xxx`（GUI 内拦截，不占算子）
//! - HTTP：`POST /v1/avatar/switch {"id":"xxx"}`
//! - 切换联动：TTS 音色(voice.tts_sid) → 语音通道；speak_prefix/suffix → 性格措辞；
//!   视觉参数 → WebGL 形象（render_mode=topo 拓扑球 / xiaobai Q版小人）。

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

// ================================================================ 规范结构 ====

/// 语音通道配置（TTS 音色等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceConf {
    /// Kokoro 音色 id（multi-lang：45=小北 / 46=小妮 / 47=小小 / 48=小一）
    pub tts_sid: i32,
    /// 语速 0.5~2.0
    pub speed: f32,
    /// 音调 0.5~2.0
    pub pitch: f32,
}
impl Default for VoiceConf {
    fn default() -> Self {
        Self { tts_sid: 45, speed: 1.0, pitch: 1.0 }
    }
}

/// 性格通道配置（措辞 / 语气 / 自称）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PersonaConf {
    /// 自称（如“小白”）
    pub self_name: String,
    /// 性格风格：soft / energetic / calm / witty / loyal
    pub style: String,
    /// 开场白模板
    pub greeting: String,
    /// 回答前缀（性格措辞）
    pub speak_prefix: String,
    /// 回答后缀（语气词）
    pub speak_suffix: String,
    /// 语气词池
    pub tone_words: Vec<String>,
    /// 语气浓度（0~1）
    pub temperature: f32,
}
impl Default for PersonaConf {
    fn default() -> Self {
        Self {
            self_name: "小白".into(),
            style: "soft".into(),
            greeting: "你好呀，我是小白～".into(),
            speak_prefix: String::new(),
            speak_suffix: "～".into(),
            tone_words: vec!["呢".into(), "呀".into()],
            temperature: 0.8,
        }
    }
}

/// 五状态球体色（倾听/思考/执行/回应/待机）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StateColors {
    pub idle: String,
    pub listen: String,
    pub think: String,
    pub speak: String,
    pub executing: String,
}
impl Default for StateColors {
    fn default() -> Self {
        Self {
            idle: "#7fd4c1".into(),
            listen: "#ff5a5a".into(),
            think: "#4a6cf7".into(),
            speak: "#34d17b".into(),
            executing: "#ffb340".into(),
        }
    }
}

/// 视觉通道配置（WebGL 形象渲染参数）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualConf {
    /// 渲染模式：topo（拓扑球）/ xiaobai（Q版小白小人）
    pub render_mode: String,
    /// 主体色
    pub body_color: String,
    /// 流光/粒子强调色（东方国风青金/绛紫）
    pub accent: Vec<String>,
    /// 眼睛发光色
    pub eye_glow: String,
    /// 粒子数量
    pub particle_count: usize,
    /// 材质：matte / gloss / frost
    pub material: String,
    /// 缩放
    pub scale: f32,
    /// 五状态色
    pub state_colors: StateColors,
}
impl Default for VisualConf {
    fn default() -> Self {
        Self {
            render_mode: "xiaobai".into(),
            body_color: "#f4f4f4".into(),
            accent: vec!["#4fa3ff".into(), "#7b5cd6".into()],
            eye_glow: "#ffffff".into(),
            particle_count: 140,
            material: "matte".into(),
            scale: 1.0,
            state_colors: StateColors::default(),
        }
    }
}

/// 形象模型（全维：视觉 + 语音 + 性格）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Avatar {
    pub id: String,
    pub name: String,
    pub version: String,
    pub desc: String,
    /// 默认启动模型标记（注册表把它排到首位）
    pub default: bool,
    pub voice: VoiceConf,
    pub persona: PersonaConf,
    pub visual: VisualConf,
}
impl Default for Avatar {
    fn default() -> Self {
        Self {
            id: "xiaobai-white".into(),
            name: "小白·纯白".into(),
            version: "1.0.0".into(),
            desc: "默认形象：白色 Q 版圆润小人（源自三视角设定图）".into(),
            default: true,
            voice: VoiceConf::default(),
            persona: PersonaConf::default(),
            visual: VisualConf::default(),
        }
    }
}

/// 列表元信息（免全量传输）。
#[derive(Debug, Clone, Serialize)]
pub struct AvatarMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub desc: String,
    pub style: String,
    pub render_mode: String,
}

/// 多路径探测形象模型目录（`models/avatars/`）。
/// 顺序：环境变量 XIAOBAI_AVATAR_DIR > 当前目录 > 仓库路径 > exe 相对。
pub fn locate_avatar_dir() -> std::path::PathBuf {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(env) = std::env::var("XIAOBAI_AVATAR_DIR") {
        if !env.trim().is_empty() {
            candidates.push(std::path::PathBuf::from(env.trim()));
        }
    }
    candidates.push(std::path::PathBuf::from("models/avatars"));
    candidates.push(std::path::PathBuf::from("projects/xiaobai_voice/models/avatars"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("models/avatars"));
        }
    }
    for c in &candidates {
        if c.is_dir() && std::fs::read_dir(c).map(|mut it| it.next().is_some()).unwrap_or(false) {
            return c.clone();
        }
    }
    candidates[0].clone()
}

/// 形象模型注册表（线程安全，运行时切换）。
pub struct AvatarRegistry {
    avatars: Vec<Avatar>,
    current: AtomicUsize,
}

impl AvatarRegistry {
    /// 扫描目录加载全部模型；目录缺失/为空时回退默认“小白·纯白”。
    /// 跳过 `*.schema.json`。
    pub fn load_dir(dir: &Path) -> Result<Self, String> {
        let mut avatars: Vec<Avatar> = Vec::new();
        if dir.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(dir)
                .map_err(|e| format!("读取形象模型目录失败: {dir:?} {e}"))?
                .filter_map(|e| e.ok())
                .collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if fname.ends_with(".schema.json") {
                    continue;
                }
                match std::fs::read_to_string(&path) {
                    Ok(raw) => match serde_json::from_str::<Avatar>(&raw) {
                        Ok(mut a) => {
                            // 未显式给 id 时用文件名
                            if a.id.is_empty() {
                                a.id = fname.trim_end_matches(".json").to_string();
                            }
                            avatars.push(a);
                        }
                        Err(e) => {
                            eprintln!("[avatar] 跳过解析失败的模型 {fname}: {e}");
                        }
                    },
                    Err(e) => eprintln!("[avatar] 读取失败 {fname}: {e}"),
                }
            }
        }
        if avatars.is_empty() {
            eprintln!("[avatar] 未发现形象模型（{dir:?}），回退默认“小白·纯白”");
            avatars.push(Avatar::default());
        }
        // 默认模型：优先 "default": true 标记 → 其次 id=="xiaobai-white" → 否则第一个
        let default_idx = avatars
            .iter()
            .position(|a| a.default)
            .or_else(|| avatars.iter().position(|a| a.id == "xiaobai-white"))
            .unwrap_or(0);
        if default_idx != 0 {
            let a = avatars.remove(default_idx);
            avatars.insert(0, a);
        }
        Ok(Self {
            avatars,
            current: AtomicUsize::new(0),
        })
    }

    /// 模型列表。
    pub fn list(&self) -> Vec<AvatarMeta> {
        self.avatars
            .iter()
            .map(|a| AvatarMeta {
                id: a.id.clone(),
                name: a.name.clone(),
                version: a.version.clone(),
                desc: a.desc.clone(),
                style: a.persona.style.clone(),
                render_mode: a.visual.render_mode.clone(),
            })
            .collect()
    }

    /// 当前模型。
    pub fn current(&self) -> &Avatar {
        &self.avatars[self.current.load(Ordering::Relaxed).min(self.avatars.len() - 1)]
    }

    /// 按 id 切换；不存在返回 Err。
    pub fn switch(&self, id: &str) -> Result<&Avatar, String> {
        let idx = self
            .avatars
            .iter()
            .position(|a| a.id == id || a.name == id)
            .ok_or_else(|| format!("未找到形象模型: {id}"))?;
        self.current.store(idx, Ordering::Relaxed);
        Ok(self.current())
    }

    /// 直接设置当前序号（越界回夹）。
    pub fn set_current(&self, idx: usize) {
        self.current.store(idx.min(self.avatars.len() - 1), Ordering::Relaxed);
    }

    /// 把当前模型的语音通道应用到 VoiceEngine（切换 TTS 音色）。
    #[cfg(feature = "voice-engine")]
    pub fn apply_voice(&self, engine: &crate::voice_engine::VoiceEngine) {
        engine.set_tts_sid(self.current().voice.tts_sid);
    }

    /// 性格措辞：对回答文本套用当前模型的 前缀 + 文本 + 后缀。
    pub fn decorate(&self, text: &str) -> String {
        let p = &self.current().persona;
        if text.is_empty() {
            return String::new();
        }
        format!("{}{}{}", p.speak_prefix, text, p.speak_suffix)
    }
}
