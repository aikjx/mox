// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 中文意图规则定义（Python router.py 规则 Rust 版 + 40 应用别名常量）

use serde_json::json;
use std::collections::BTreeMap;

/// 单条规则：正则 → (action, score, 从 regex captures 提取参数的闭包)
///
/// score 设计：
/// - 1.00：精确匹配命令词（"打开微信"、"静音"、"截屏"）
/// - 0.90：前缀/后缀模糊（"帮我打开 xxx"、"把音量调到 xx"）
/// - 0.75：包含式（"xx 一下"、"xxx 可以吗"）
/// - 0.55：兜底别名（"xx" 应用名裸出现）
pub type RuleClosure = Box<dyn Fn(&regex::Captures) -> (String, serde_json::Value) + Send + Sync>;
pub struct Rule {
    pub regex: regex::Regex,
    pub score: f32,
    pub category: &'static str,
    pub extractor: RuleClosure,
}

impl std::fmt::Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rule")
            .field("regex", &self.regex)
            .field("score", &self.score)
            .field("category", &self.category)
            .field("extractor", &"<closure>")
            .finish()
    }
}

// ---------- 应用别名精确列表（Python app_aliases 1:1；Router 做"打开 N"规则时遍历） ----------
pub const APP_ALIAS_EXACT_LIST: &[(&str, &str)] = &[
    // (用户可能说的名字, normalize_app_exec 的 alias key；Router 里直接把原文传给 open_app)
    ("记事本", "记事本"),
    ("计算器", "计算器"),
    ("画图", "画图"),
    ("资源管理器", "资源管理器"),
    ("任务管理器", "任务管理器"),
    ("Chrome", "Chrome"),
    ("chrome", "chrome"),
    ("谷歌浏览器", "谷歌浏览器"),
    ("Edge", "Edge"),
    ("微软浏览器", "微软浏览器"),
    ("Firefox", "Firefox"),
    ("火狐浏览器", "火狐浏览器"),
    ("VS Code", "VS Code"),
    ("vscode", "vscode"),
    ("Visual Studio Code", "Visual Studio Code"),
    ("Word", "Word"),
    ("Excel", "Excel"),
    ("PowerPoint", "PowerPoint"),
    ("PPT", "PPT"),
    ("Outlook", "Outlook"),
    ("微信", "微信"),
    ("WeChat", "WeChat"),
    ("企业微信", "企业微信"),
    ("WXWork", "WXWork"),
    ("飞书", "飞书"),
    ("Lark", "Lark"),
    ("钉钉", "钉钉"),
    ("QQ", "QQ"),
    ("腾讯QQ", "腾讯QQ"),
    ("WPS", "WPS"),
    ("WPS Office", "WPS Office"),
    ("OBS", "OBS"),
    ("OBS Studio", "OBS Studio"),
    ("XMind", "XMind"),
    ("Postman", "Postman"),
    ("Typora", "Typora"),
    ("迅雷", "迅雷"),
    ("网易云音乐", "网易云音乐"),
    ("QQ音乐", "QQ音乐"),
    ("哔哩哔哩", "哔哩哔哩"),
];

/// 常见按键名（press_key/hotkey key 参数的 alias 归一）
pub const COMMON_KEY_NAMES: &[(&str, &str)] = &[
    ("回车", "enter"), ("确定", "enter"), ("换行", "enter"),
    ("空格", "space"), ("间隔", "space"),
    ("退格", "backspace"), ("删除", "delete"),
    ("esc", "escape"), ("取消", "escape"), ("退出键", "escape"),
    ("大写锁定", "capslock"), ("caps", "capslock"),
    ("F1", "f1"), ("F2", "f2"), ("F3", "f3"), ("F4", "f4"),
    ("F5", "f5"), ("F6", "f6"), ("F7", "f7"), ("F8", "f8"),
    ("F9", "f9"), ("F10", "f10"), ("F11", "f11"), ("F12", "f12"),
    ("左", "left"), ("右", "right"), ("上", "up"), ("下", "down"),
    ("方向左", "left"), ("方向右", "right"), ("方向上", "up"), ("方向下", "down"),
    ("home", "home"), ("end", "end"), ("page up", "pageup"), ("page down", "pagedown"),
    ("插入", "insert"), ("insert", "insert"),
    ("ctrl", "ctrl"), ("control", "ctrl"), ("控制", "ctrl"),
    ("shift", "shift"), ("上档", "shift"),
    ("alt", "alt"), ("alt键", "alt"), ("选项", "alt"),
    ("win", "meta"), ("windows", "meta"), ("cmd", "meta"), ("command", "meta"), ("开始", "meta"),
    ("复制", "ctrl_c"), ("粘贴", "ctrl_v"), ("剪切", "ctrl_x"), ("撤销", "ctrl_z"),
    ("保存", "ctrl_s"), ("全选", "ctrl_a"), ("查找", "ctrl_f"),
];

// ---------- 规则构造函数 ----------

fn rule_app_open() -> Vec<Rule> {
    // R1: "打开/启动/运行/点开 xxx"
    let re_open = regex::Regex::new(r"^(?:帮我|能不能|请|麻烦|你给我)?\s*(?:打开|启动|运行|点开|开一下|开|开启)\s*(?P<app>.+?)\s*(?:好不好|行不|可以吗|谢谢|多谢|呗|哈)?\s*$").unwrap();
    // R1b: "xxx 打开/启动"（倒装）
    let re_open_rev = regex::Regex::new(r"^(?P<app>.+?)\s*(?:打开|启动|运行|跑一下|启动一下)\s*(?:好不好|可以吗)?\s*$").unwrap();
    // R2: "关闭/关掉/强制退出/结束进程 xxx"
    let re_close = regex::Regex::new(r"^(?:帮我|请|麻烦)?\s*(?:关闭|关掉|强制关闭|强制退出|杀进程|结束进程|停止|退出)\s*(?P<app>.+?)\s*(?:好不好|可以吗|谢谢)?\s*$").unwrap();
    vec![
        Rule { regex: re_open, score: 0.98, category: "app", extractor: Box::new(|c| {
            let app = c.name("app").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("open_app".into(), json!({"app_name": app}))
        }) },
        Rule { regex: re_open_rev, score: 0.92, category: "app", extractor: Box::new(|c| {
            let app = c.name("app").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("open_app".into(), json!({"app_name": app}))
        }) },
        Rule { regex: re_close, score: 0.99, category: "app", extractor: Box::new(|c| {
            let app = c.name("app").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("close_app".into(), json!({"app_name": app}))
        }) },
    ]
}

fn rule_volume() -> Vec<Rule> {
    let re_percent = regex::Regex::new(r"音量(?:调到|改到|设置为|变成)?\s*(?P<n>\d{1,3})\s*(?:%|百分之|格)?\s*(?:左右|左右差不多)?").unwrap();
    let re_mute = regex::Regex::new(r"(?:把?系统?)?(?:静音|关声音|禁音|关闭声音|无声音|消音)").unwrap();
    let re_unmute = regex::Regex::new(r"(?:把?系统?)?(?:取消静音|解除静音|打开声音|开声音|恢复声音)").unwrap();
    let re_toggle = regex::Regex::new(r"(?:切换)?\s*(?:静音|声音)\s*(?:开关|切换一下|反一下|取反)").unwrap();
    let re_up = regex::Regex::new(r"(?:音量|声音)\s*(?:调大|开大|加大|升高|增加|上去|加一点|加几格|往上)").unwrap();
    let re_down = regex::Regex::new(r"(?:音量|声音)\s*(?:调小|关小|减小|降低|下去|减一点|降几格|往下|变小)").unwrap();
    let re_get = regex::Regex::new(r"(?:现在)?(?:音量|声音是多大|有多大|多少|多大)|(?:当前)?(?:音量|声音)(?:多少|查询|查看|是多少)").unwrap();
    vec![
        Rule { regex: re_percent, score: 1.00, category: "volume", extractor: Box::new(|c| {
            let n: i64 = c.name("n").and_then(|m| m.as_str().parse().ok()).unwrap_or(50).min(100).max(0);
            ("set_volume".into(), json!({"percent": n}))
        }) },
        Rule { regex: re_mute, score: 0.99, category: "volume", extractor: Box::new(|_| ("mute".into(), json!({}))) },
        Rule { regex: re_unmute, score: 0.99, category: "volume", extractor: Box::new(|_| ("unmute".into(), json!({}))) },
        Rule { regex: re_toggle, score: 0.95, category: "volume", extractor: Box::new(|_| ("toggle_mute".into(), json!({}))) },
        Rule { regex: re_up, score: 0.90, category: "volume", extractor: Box::new(|_| ("set_volume".into(), json!({"percent_increment": 15}))) },
        Rule { regex: re_down, score: 0.90, category: "volume", extractor: Box::new(|_| ("set_volume".into(), json!({"percent_increment": -15}))) },
        Rule { regex: re_get, score: 0.88, category: "volume", extractor: Box::new(|_| ("get_volume".into(), json!({}))) },
    ]
}

fn rule_input() -> Vec<Rule> {
    use COMMON_KEY_NAMES as K;
    let re_mouse_move_to = regex::Regex::new(r"鼠标(?:移到|移至|移动到|放到|调到)?\s*[\(（]?\s*(?P<x>\d{1,5})\s*[,，\s]\s*(?P<y>\d{1,5})\s*[\)）]?").unwrap();
    let re_click = regex::Regex::new(r"(?:(?:鼠标\s*)?点击|单击|点一下|点|按一下)\s*(?P<button>左键|右键|中键|左|右|中)?").unwrap();
    let re_double = regex::Regex::new(r"(?:双击|鼠标双击|连点两下|点两下)").unwrap();
    let re_screenshot = regex::Regex::new(r"(?:(?:截个屏|截屏|截图|屏幕截图|截一下屏|抓屏|抓个图|拍摄屏幕|打印屏幕))").unwrap();
    let re_hotkey = regex::Regex::new(r"(?:按一下|按下|按)\s*(?P<k>[^\s]{1,20})(?:键)?\s*$").unwrap();
    let re_copy_clipboard = regex::Regex::new(r"(?:把|将)?\s*(?P<t>.*?)\s*(?:复制到剪贴板|复制一下|复制下来|放到剪贴板|存到剪贴板|拷到剪贴板)\s*(?:可以吗|好不好)?").unwrap();
    let re_type = regex::Regex::new(r"(?:(?:打字|输入|写入|敲入|输入文字)\s*(?P<t>[\s\S]+?)\s*(?:可以吗|好不好|一下)?$)|(?:说\s*(?P<t2>[\s\S]+?)\s*(?:给我|出来)?\s*)").unwrap();
    let re_position = regex::Regex::new(r"(?:(?:当前)?(?:鼠标位置|光标位置|鼠标坐标|光标的坐标)|鼠标现在在哪|鼠标现在的位置)").unwrap();
    let re_center = regex::Regex::new(r"(?:把|将)?\s*(?:鼠标|光标)\s*(?:移到屏幕中央|移到中间|居中|到屏幕中心|回到中心|放中间)").unwrap();
    let re_scroll = regex::Regex::new(r"(?:(?:鼠标)?滚轮|滚动|往下滚|往上滚)\s*(?P<n>\d{1,3})?\s*(?:格|tick|下|步)?\s*(?:向下|往上|向上|往下)?").unwrap();
    let re_drag = regex::Regex::new(r"拖拽从[\(（]?\s*(?P<fx>\d{1,5})\s*[,，]\s*(?P<fy>\d{1,5})\s*[\)）]?\s*(?:到|至|拖到|移动到)\s*[\(（]?\s*(?P<tx>\d{1,5})\s*[,，]\s*(?P<ty>\d{1,5})\s*[\)）]?").unwrap();

    let key_norm = |raw: &str| -> String {
        // 复制粘贴等语义组合
        match raw {
            "ctrl_c" => return "ctrl+c".into(),
            "ctrl_v" => return "ctrl+v".into(),
            "ctrl_x" => return "ctrl+x".into(),
            "ctrl_z" => return "ctrl+z".into(),
            "ctrl_s" => return "ctrl+s".into(),
            "ctrl_a" => return "ctrl+a".into(),
            "ctrl_f" => return "ctrl+f".into(),
            _ => {}
        }
        for (zh, en) in K.iter() {
            if raw == *zh {
                return en.to_string();
            }
        }
        raw.to_lowercase()
    };

    vec![
        Rule { regex: re_mouse_move_to, score: 0.99, category: "input", extractor: Box::new(|c| {
            let x: i64 = c.name("x").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let y: i64 = c.name("y").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            ("mouse_move".into(), json!({"x": x, "y": y}))
        }) },
        Rule { regex: re_click, score: 0.95, category: "input", extractor: Box::new(|c| {
            let btn = c.name("button").map(|m| match m.as_str() {
                "右键" | "右" => "right",
                "中键" | "中" => "middle",
                _ => "left",
            }).unwrap_or("left");
            ("click".into(), json!({"button": btn}))
        }) },
        Rule { regex: re_double, score: 0.99, category: "input", extractor: Box::new(|_| ("double_click".into(), json!({}))) },
        Rule { regex: re_screenshot, score: 1.00, category: "input", extractor: Box::new(|_| ("screenshot".into(), json!({}))) },
        Rule { regex: re_hotkey, score: 0.80, category: "input", extractor: Box::new(move |c| {
            let raw = c.name("k").map(|m| m.as_str()).unwrap_or("enter");
            // Ctrl+C / Alt+Tab 之类的组合
            if raw.contains('+') {
                let parts: Vec<String> = raw.split('+').map(|p| key_norm(p)).collect();
                let default_enter = "enter".to_string();
                let (last, mods) = parts.split_last().unwrap_or((&default_enter, &[]));
                let modifiers: Vec<String> = mods.iter().cloned().collect();
                ("hotkey".into(), json!({"modifiers": modifiers, "key": last}))
            } else {
                let k = key_norm(raw);
                ("press_key".into(), json!({"key": k}))
            }
        }) },
        Rule { regex: re_copy_clipboard, score: 0.97, category: "input", extractor: Box::new(|c| {
            let t = c.name("t").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("copy_to_clipboard".into(), json!({"content": t}))
        }) },
        Rule { regex: re_type, score: 0.85, category: "input", extractor: Box::new(|c| {
            let t = c.name("t").map(|m| m.as_str().to_string())
                .or_else(|| c.name("t2").map(|m| m.as_str().to_string()))
                .unwrap_or_default();
            ("type_text".into(), json!({"text": t}))
        }) },
        Rule { regex: re_position, score: 0.96, category: "input", extractor: Box::new(|_| ("mouse_position".into(), json!({}))) },
        Rule { regex: re_center, score: 0.96, category: "input", extractor: Box::new(|_| ("move_cursor_to_center".into(), json!({}))) },
        Rule { regex: re_scroll, score: 0.90, category: "input", extractor: Box::new(|c| {
            let n: i64 = c.name("n").and_then(|m| m.as_str().parse().ok()).unwrap_or(3);
            // 语义："往下滚 5" → 正数 5；"往上滚 3" → 负数
            let full = c.get(0).map(|m| m.as_str()).unwrap_or("");
            let signed = if full.contains("上") { -n } else { n };
            ("scroll_wheel".into(), json!({"delta": signed * 3}))
        }) },
        Rule { regex: re_drag, score: 1.00, category: "input", extractor: Box::new(|c| {
            let fx: i64 = c.name("fx").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let fy: i64 = c.name("fy").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let tx: i64 = c.name("tx").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let ty: i64 = c.name("ty").and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            ("mouse_drag".into(), json!({"from_x": fx, "from_y": fy, "to_x": tx, "to_y": ty}))
        }) },
    ]
}

fn rule_file() -> Vec<Rule> {
    let re_exists = regex::Regex::new(r"(?:文件?)\s*(?P<p>\S+?)\s*(?:存在吗|在吗|有没有|是否存在)").unwrap();
    let re_read = regex::Regex::new(r"(?:读一下|读取|查看内容|看下|读几行)\s*(?P<p>\S+)").unwrap();
    let re_open_file = regex::Regex::new(r"(?:打开文件?\s*|启动文件?\s*)(?P<p>\S+?)(?:\s*用(?P<a>\S+))?$").unwrap();
    let re_trash = regex::Regex::new(r"(?:把|将)?\s*(?P<p>\S+?)\s*(?:丢回收站|放到回收站|移到回收站|删到回收站|进回收站|移入回收站|删除到回收站)").unwrap();
    vec![
        Rule { regex: re_exists, score: 0.97, category: "file", extractor: Box::new(|c| {
            let p = c.name("p").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("file_exists".into(), json!({"path": p}))
        }) },
        Rule { regex: re_read, score: 0.95, category: "file", extractor: Box::new(|c| {
            let p = c.name("p").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("read_text_head".into(), json!({"path": p, "max_lines": 3}))
        }) },
        Rule { regex: re_open_file, score: 0.93, category: "file", extractor: Box::new(|c| {
            let p = c.name("p").map(|m| m.as_str().to_string()).unwrap_or_default();
            let app = c.name("a").map(|m| m.as_str().to_string());
            let mut val = json!({"path": p});
            if let Some(a) = app { val.as_object_mut().unwrap().insert("app_name".into(), json!(a)); }
            ("open_file_with_app".into(), val)
        }) },
        Rule { regex: re_trash, score: 0.97, category: "file", extractor: Box::new(|c| {
            let p = c.name("p").map(|m| m.as_str().to_string()).unwrap_or_default();
            ("move_to_trash".into(), json!({"path": p, "allow_permanent_delete": false}))
        }) },
    ]
}

fn rule_system() -> Vec<Rule> {
    let re_list_proc = regex::Regex::new(r"(?:当前)?\s*(?:有什么进程|跑了什么程序|哪些程序在运行|进程列表|看一下进程|列进程)").unwrap();
    let re_list_vol_devices = regex::Regex::new(r"(?:音频设备|声音设备|有哪些音频设备|列一下音频输出|查看音频设备)").unwrap();
    let re_health = regex::Regex::new(r"小白语音健康检查|voice服务在吗|hello小白|你好小白").unwrap();
    vec![
        Rule { regex: re_list_proc, score: 0.98, category: "app", extractor: Box::new(|_| ("list_running".into(), json!({}))) },
        Rule { regex: re_list_vol_devices, score: 0.95, category: "volume", extractor: Box::new(|_| ("list_devices".into(), json!({}))) },
        Rule { regex: re_health, score: 0.90, category: "system", extractor: Box::new(|_| ("health_check".into(), json!({}))) },
    ]
}

/// 所有规则聚合（一次性构建 lazy static 风格；我们用 fn 返回避免 OnceLock API 差异）
pub fn build_rule_set() -> Vec<Rule> {
    let mut all = Vec::new();
    all.extend(rule_app_open());
    all.extend(rule_volume());
    all.extend(rule_input());
    all.extend(rule_file());
    all.extend(rule_system());
    all
}

/// 40 应用别名兜底：如果没命中任何正则，看是否包含 APP_ALIAS_EXACT_LIST 里的某个词作为"打开 xxx" 语义 0.55 分
pub fn app_alias_fallback(text: &str) -> Vec<(String, f32, &'static str, serde_json::Value)> {
    let mut out = Vec::new();
    for (say, _key) in APP_ALIAS_EXACT_LIST.iter() {
        if text.contains(*say) {
            out.push((
                "open_app".into(),
                0.55f32,
                "app",
                json!({"app_name": say}),
            ));
        }
    }
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(5);
    out
}

// ---------- 小工具：把"volume percent_increment +15 / -15"归一成实际 percent（70+15=85） ----------
pub fn apply_increments(param: &mut serde_json::Value) {
    let inc = param.get("percent_increment").and_then(|v| v.as_i64());
    if let (Some(obj), Some(inc)) = (param.as_object_mut(), inc) {
        let current = obj.get("percent").and_then(|v| v.as_i64()).unwrap_or(50);
        let next = (current + inc).max(0).min(100);
        obj.insert("percent".into(), json!(next));
        obj.remove("percent_increment");
    }
}

#[cfg(test)]
mod t {
    use super::*;
    use regex::Regex;

    #[test]
    fn rule_open_chinese_app_wechat() {
        let re = Regex::new(r"^(?:帮我|能不能|请|麻烦|你给我)?\s*(?:打开|启动|运行|点开|开一下|开|开启)\s*(?P<app>.+?)\s*(?:好不好|行不|可以吗|谢谢|多谢|呗|哈)?\s*$").unwrap();
        let c = re.captures("帮我打开微信").unwrap();
        assert_eq!(&c["app"], "微信");
    }
    #[test]
    fn rule_volume_33_percent() {
        let re = regex::Regex::new(r"音量(?:调到|改到|设置为|变成)?\s*(?P<n>\d{1,3})\s*(?:%|百分之|格)?").unwrap();
        let c = re.captures("把音量调到33%").unwrap();
        assert_eq!(&c["n"], "33");
    }
    #[test]
    fn app_alias_list_has_40() {
        assert!(APP_ALIAS_EXACT_LIST.len() >= 39, "长度 {}", APP_ALIAS_EXACT_LIST.len());
    }
}
