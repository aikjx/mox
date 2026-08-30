// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Input 算子：鼠标/键盘/输入（mouse_position / mouse_move / click / double_click / type_text / press_key / hotkey / key_sequence / mouse_drag / screenshot / scroll_wheel / move_cursor_to_center）
//!
//! 回退链：
//! - 键鼠：enigo 跨平台 → Windows(win32 API) 兜底（P2 接 windows-rs SendInput）
//! - 中文 type_text：arboard 剪贴板粘贴 Ctrl+V 回退（enigo 对中文 Unicode 不保证完美）
//! - 截图：screenshots 跨平台 crate → L3 权限

mod common;
mod mouse;
mod keyboard;
mod text;
mod screenshot;

use std::time::Instant;

use async_trait::async_trait;

use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;

// enigo 0.2：Keyboard/Mouse/Axis 在单独 trait 模块中，必须 use 后才能调 key/button/scroll 等方法
#[allow(unused_imports)]
use enigo::{Keyboard, Mouse, Axis};

pub use mouse::parse_button;
pub use keyboard::{parse_key, parse_modifier};
pub use common::{require_int, enigo_check_ok};

#[derive(Debug, Default, Clone)]
pub struct InputOperator;

#[async_trait]
impl SystemOperator for InputOperator {
    fn id(&self) -> &'static str {
        "input_operator_v1"
    }
    fn category(&self) -> OperatorCategory {
        OperatorCategory::Input
    }
    fn list_actions(&self) -> Vec<ActionSignature> {
        use ClearanceLevel::*;
        use std::collections::BTreeMap;
        let mut p_mouse = BTreeMap::new();
        p_mouse.insert("x", "int 屏幕像素 X 坐标");
        p_mouse.insert("y", "int 屏幕像素 Y 坐标");
        let mut p_click = BTreeMap::new();
        p_click.insert("button", "string left/right/middle，默认 left");
        p_click.insert("x", "int 可选点击前先把鼠标移动到该点");
        p_click.insert("y", "int 同上");
        let mut p_type = BTreeMap::new();
        p_type.insert("text", "string：ASCII 走 enigo.key_sequence，中文走剪贴板粘贴 Ctrl+V 回退（L2）");
        let mut p_key = BTreeMap::new();
        p_key.insert("key", "string：a/b/c/enter/esc/ctrl/alt/shift/f1~f12 等 enigo::Key 名称");
        let mut p_hotkey = BTreeMap::new();
        p_hotkey.insert("modifiers", "string[]：['ctrl','shift','alt','win'] 任意组合");
        p_hotkey.insert("key", "同 key 参数");
        let mut p_seq = BTreeMap::new();
        p_seq.insert("keys", "string[]：顺序按下的 key 序列（同 key 名称）");
        let mut p_drag = BTreeMap::new();
        p_drag.insert("from_x", "int 起点 X");
        p_drag.insert("from_y", "int 起点 Y");
        p_drag.insert("to_x", "int 终点 X");
        p_drag.insert("to_y", "int 终点 Y");
        p_drag.insert("button", "同 click，默认 left（L3 破坏性：拖拽会改变文件/选择）");
        let mut p_scroll = BTreeMap::new();
        p_scroll.insert("delta", "int 负数向上，正数向下，单位：3 lines ≈ 1 tick");
        vec![
            ActionSignature {
                name: "mouse_position",
                category: OperatorCategory::Input,
                clearance: L0,
                own_qualified: false,
                description: "只读：返回当前鼠标坐标 (x,y)",
                params: None,
            },
            ActionSignature {
                name: "mouse_move",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "把鼠标绝对移动到 (x,y) 屏幕像素坐标",
                params: Some(p_mouse.clone()),
            },
            ActionSignature {
                name: "click",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "在当前位置（或指定 x,y）按一下鼠标键（默认 left）",
                params: Some(p_click.clone()),
            },
            ActionSignature {
                name: "double_click",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "鼠标左键双击（或指定位置）",
                params: Some(p_click),
            },
            ActionSignature {
                name: "type_text",
                category: OperatorCategory::Input,
                clearance: L1,
                own_qualified: false,
                description: "输入文本；ASCII L1 放行；中文需要 L2（因为走剪贴板，Expert/Coordinator）",
                params: Some(p_type),
            },
            ActionSignature {
                name: "press_key",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "按下并松开一个键",
                params: Some(p_key.clone()),
            },
            ActionSignature {
                name: "hotkey",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "组合键：按住 modifiers 再按 key 再松开（如 Ctrl+C）",
                params: Some(p_hotkey),
            },
            ActionSignature {
                name: "key_sequence",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "按顺序按下一系列键（如 ['ctrl','a','ctrl','c']）",
                params: Some(p_seq),
            },
            ActionSignature {
                name: "mouse_drag",
                category: OperatorCategory::Input,
                clearance: L3,
                own_qualified: false,
                description: "按住鼠标左键从 A 拖到 B（破坏性：移动/删除文件/选区，MoxAdmin 权限）",
                params: Some(p_drag),
            },
            ActionSignature {
                name: "screenshot",
                category: OperatorCategory::Input,
                clearance: L3,
                own_qualified: false,
                description: "截取主屏 PNG 返回 base64 + 尺寸（L3：屏幕可能含 PII 敏感信息）",
                params: None,
            },
            ActionSignature {
                name: "scroll_wheel",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "上下滚动鼠标滚轮",
                params: Some(p_scroll),
            },
            ActionSignature {
                name: "move_cursor_to_center",
                category: OperatorCategory::Input,
                clearance: L2,
                own_qualified: false,
                description: "把鼠标移到主屏中心位置（方便后续定位）",
                params: None,
            },
        ]
    }
    async fn execute(
        &self,
        action: &str,
        param: ActionParam,
        _identity: &OperatorIdentity,
    ) -> Result<OperatorOutput, XiaobaiError> {
        let t0 = Instant::now();
        let mut fbs: Vec<&'static str> = Vec::new();
        // 无头探测
        if !common::enigo_check_ok() {
            fbs.push("enigo_init_failed_headless_or_no_display");
        } else {
            fbs.push("enigo_ready");
        }
        let _ = t0; // 各子模块自己计时
        match action {
            "mouse_position" => mouse::mouse_position(&fbs),
            "mouse_move" => mouse::mouse_move(&param, &fbs),
            "click" | "double_click" => mouse::click_or_double(action, &param, &fbs),
            "type_text" => text::type_text(&param, &fbs),
            "press_key" => keyboard::press_key(&param, &fbs),
            "hotkey" => keyboard::hotkey(&param, &fbs),
            "key_sequence" => keyboard::key_sequence(&param, &fbs),
            "mouse_drag" => mouse::mouse_drag(&param, &fbs),
            "screenshot" => screenshot::screenshot(&param, &fbs),
            "scroll_wheel" => mouse::scroll_wheel(&param, &fbs),
            "move_cursor_to_center" => mouse::move_cursor_to_center(&fbs),
            other => Err(XiaobaiError::IntentUnknown(other.into())),
        }
    }
}
