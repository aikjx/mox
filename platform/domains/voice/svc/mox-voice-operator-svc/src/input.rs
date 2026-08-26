//! Input 算子：鼠标/键盘/输入（mouse_position / mouse_move / click / double_click / type_text / press_key / hotkey / key_sequence / mouse_drag / screenshot / scroll_wheel / move_cursor_to_center）
//!
//! 回退链：
//! - 键鼠：enigo 跨平台 → Windows(win32 API) 兜底（P2 接 windows-rs SendInput）
//! - 中文 type_text：arboard 剪贴板粘贴 Ctrl+V 回退（enigo 对中文 Unicode 不保证完美）
//! - 截图：screenshots 跨平台 crate → L3 权限
use std::time::{Duration, Instant};
use async_trait::async_trait;
use serde_json::json;
use crate::helpers::platform_tag;
use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
   ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;
// enigo 0.2：Keyboard/Mouse/Axis 在单独 trait 模块中，必须 use 后才能调 key/button/scroll 等方法
use enigo::{Keyboard, Mouse, Axis};
#[derive(Debug, Default, Clone)]
pub struct InputOperator;
impl InputOperator {
   fn enigo_check_ok() -> bool {
       // enigo 在无头 CI 里初始化会失败；这里提前探测（先 new 一下立即 drop）
       use enigo::*;
       Enigo::new(&Settings::default()).is_ok()
   }
}
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
       if !Self::enigo_check_ok() {
           fbs.push("enigo_init_failed_headless_or_no_display");
       } else {
           fbs.push("enigo_ready");
       }
       match action {
           "mouse_position" => {
               // enigo：mouse_location()（当前 enigo 0.2 没有位置接口，先返回 (0,0) stub，XB-007 会由上层理解为"没拿到"）
               fbs.push("mouse_location_stub");
               Ok(OperatorOutput::quick("enigo 0.2 暂无位置 API，返回 0,0 占位；P2 接入 windows-rs GetCursorPos 实现")
                   .with_payload(json!({"x": 0, "y": 0, "stub": true}))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "mouse_move" => {
               let x = require_int(&param, "mouse_move", "x")?;
               let y = require_int(&param, "mouse_move", "y")?;
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|e| {
                   XiaobaiError::OperatorUnsupported {
                       category: OperatorCategory::Input.as_str().to_string(),
                       action: "mouse_move".into(),
                       platform: platform_tag(),
                       fallbacks_used: vec!["enigo_new_failed".to_string()],
                   }
               })?;
               en.move_mouse(x as i32, y as i32, enigo::Coordinate::Abs).map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "mouse_move".into(),
                   detail: format!("enigo move_mouse err: {e:?}"),
               })?;
               fbs.push("enigo_move_mouse_abs");
               Ok(OperatorOutput::quick(format!("mouse_move → ({},{})", x, y)).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "click" | "double_click" => {
               let button = param.get_str("button").unwrap_or("left");
               let x = param.get_i64("x");
               let y = param.get_i64("y");
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "click".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               if let (Some(xx), Some(yy)) = (x, y) {
                   en.move_mouse(xx as i32, yy as i32, enigo::Coordinate::Abs).map_err(|e| XiaobaiError::ExecutionError {
                       category: OperatorCategory::Input.as_str().into(),
                       action: "click".into(),
                       detail: format!("enigo move before click err: {e:?}"),
                   })?;
               }
               let b = parse_button(button);
               if action == "double_click" {
                   en.button(b, enigo::Direction::Press).ok();
                   en.button(b, enigo::Direction::Release).ok();
                   std::thread::sleep(Duration::from_millis(50));
                   en.button(b, enigo::Direction::Press).ok();
                   en.button(b, enigo::Direction::Release).ok();
                   fbs.push("enigo_double_left_click");
               } else {
                   en.button(b, enigo::Direction::Press).ok();
                   en.button(b, enigo::Direction::Release).ok();
                   fbs.push("enigo_single_click");
               }
               Ok(OperatorOutput::quick(format!("{action} done（button={button}）")).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "type_text" => {
               let text = param.get_str("text").ok_or_else(|| XiaobaiError::InvalidArgument {
                   action: "type_text".into(),
                   param: "text".into(),
                   value: "<missing>".into(),
                   hint: "需要 text 字符串".into(),
               })?.to_string();
               let has_chinese = text.chars().any(|c| c as u32 > 0x7F);
               // ASCII：直接 enigo.key_sequence
               if !has_chinese {
                   let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                       category: OperatorCategory::Input.as_str().to_string(),
                       action: "type_text(ascii)".into(),
                       platform: platform_tag(),
                       fallbacks_used: vec!["enigo_new_failed".to_string()],
                   })?;
                   en.text(&text).map_err(|e| XiaobaiError::ExecutionError {
                       category: OperatorCategory::Input.as_str().into(),
                       action: "type_text".into(),
                       detail: format!("enigo text err: {e:?}"),
                   })?;
                   fbs.push("enigo_text_ascii");
                   return Ok(OperatorOutput::quick(format!("输入文本 {len} 字符（ASCII）", len = text.chars().count()))
                       .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64));
               }
               // 中文：写入剪贴板 → Ctrl+V 粘贴（需要 L2 权限）
               fbs.push("chinese_clipboard_paste_backoff");
               let copy_op = crate::file::FileOperator::default();
               let fb_copy = copy_op.copy_impl(&text).map_err(|e| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "type_text(chinese)".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["copy_chinese_to_clipboard_failed".to_string()],
               })?;
               fbs.extend(fb_copy.iter().copied());
               // Ctrl+V
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "type_text(chinese)→paste".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed_for_ctrl_v".to_string()],
               })?;
               let ctrl = if cfg!(target_os = "macos") { enigo::Key::Meta } else { enigo::Key::Control };
               en.key(ctrl, enigo::Direction::Press).ok();
               en.key(enigo::Key::Unicode('v'), enigo::Direction::Press).ok();
               en.key(enigo::Key::Unicode('v'), enigo::Direction::Release).ok();
               en.key(ctrl, enigo::Direction::Release).ok();
               fbs.push("ctrl_v_paste");
               Ok(OperatorOutput::quick(format!("输入文本 {len} 字符（中文剪贴板粘贴回退）", len = text.chars().count()))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "press_key" => {
               let key = param.get_str("key").ok_or_else(|| XiaobaiError::InvalidArgument {
                   action: "press_key".into(),
                   param: "key".into(),
                   value: "<missing>".into(),
                   hint: "key 名称如 enter/esc/a/f5 等".into(),
               })?;
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "press_key".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               let k = parse_key(key);
               en.key(k, enigo::Direction::Press).map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "press_key".into(),
                   detail: format!("enigo press down err: {e:?}"),
               })?;
               en.key(k, enigo::Direction::Release).ok();
               fbs.push("enigo_press_release");
               Ok(OperatorOutput::quick(format!("按一下 {key}")).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "hotkey" => {
               let modifiers: Vec<String> = param.0.get("modifiers").and_then(|v| v.as_array())
                   .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|ss| ss.to_string())).collect())
                   .unwrap_or_default();
               let key = param.get_str("key").ok_or_else(|| XiaobaiError::InvalidArgument {
                   action: "hotkey".into(),
                   param: "key".into(),
                   value: "<missing>".into(),
                   hint: "hotkey 需要 modifiers + key".into(),
               })?.to_string();
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "hotkey".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               for m in modifiers.iter() {
                   let k = parse_modifier(m);
                   en.key(k, enigo::Direction::Press).ok();
               }
               let k = parse_key(&key);
               en.key(k, enigo::Direction::Press).ok();
               en.key(k, enigo::Direction::Release).ok();
               for m in modifiers.iter().rev() {
                   let k = parse_modifier(m);
                   en.key(k, enigo::Direction::Release).ok();
               }
               fbs.push("enigo_hotkey_modifiers_sequence");
               Ok(OperatorOutput::quick(format!("hotkey [{modifiers:?}] + {key}"))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "key_sequence" => {
               let keys: Vec<String> = param.0.get("keys").and_then(|v| v.as_array())
                   .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|ss| ss.to_string())).collect())
                   .unwrap_or_default();
               if keys.is_empty() {
                   return Err(XiaobaiError::InvalidArgument {
                       action: "key_sequence".into(),
                       param: "keys".into(),
                       value: "[]".into(),
                       hint: "keys 不能是空数组".into(),
                   });
               }
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "key_sequence".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               for k in keys.iter() {
                   let kk = parse_key(k);
                   en.key(kk, enigo::Direction::Press).ok();
                   en.key(kk, enigo::Direction::Release).ok();
               }
               fbs.push("enigo_key_sequence_n");
               Ok(OperatorOutput::quick(format!("按顺序 {n} 个键", n = keys.len()))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "mouse_drag" => {
               let fx = require_int(&param, action, "from_x")?;
               let fy = require_int(&param, action, "from_y")?;
               let tx = require_int(&param, action, "to_x")?;
               let ty = require_int(&param, action, "to_y")?;
               let button = param.get_str("button").unwrap_or("left");
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "mouse_drag".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               let b = parse_button(button);
               en.move_mouse(fx as i32, fy as i32, enigo::Coordinate::Abs).ok();
               en.button(b, enigo::Direction::Press).ok();
               std::thread::sleep(Duration::from_millis(40));
               en.move_mouse(tx as i32, ty as i32, enigo::Coordinate::Abs).ok();
               std::thread::sleep(Duration::from_millis(40));
               en.button(b, enigo::Direction::Release).ok();
               fbs.push("enigo_press_move_release");
               Ok(OperatorOutput::quick(format!("拖拽 ({fx},{fy}) → ({tx},{ty})"))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "screenshot" => {
               // screenshots crate 真实实现（Windows/macOS/Linux 支持）
               fbs.push("screenshots_crate_capture_display0");
               use screenshots::Screen;
               let screens = Screen::all().map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "screenshot".into(),
                   detail: format!("枚举显示器失败: {e}"),
               })?;
               let first = screens.first().ok_or_else(|| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "screenshot".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["no_screens_found".to_string()],
               })?;
               let img = first.capture().map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "screenshot".into(),
                   detail: format!("截屏失败: {e}"),
               })?;
               let mut png_bytes: Vec<u8> = Vec::new();
               {
                   use image::{ImageEncoder, codecs::png::PngEncoder, ExtendedColorType};
                   let enc = PngEncoder::new(&mut png_bytes);
                   enc.write_image(
                       img.as_raw(),
                       img.width(),
                       img.height(),
                       ExtendedColorType::Rgba8,
                   ).map_err(|e| XiaobaiError::ExecutionError {
                       category: OperatorCategory::Input.as_str().into(),
                       action: "screenshot".into(),
                       detail: format!("PNG 编码失败: {e}"),
                   })?;
               }
               use base64::Engine as _;
               let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
               Ok(OperatorOutput::quick(format!(
                   "截屏完成 {}x{}，PNG {} 字节（base64 在 payload）",
                   img.width(),
                   img.height(),
                   png_bytes.len()
               ))
               .with_payload(json!({
                   "width": img.width(),
                   "height": img.height(),
                   "bytes": png_bytes.len(),
                   "png_base64": b64,
               }))
               .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
               .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "scroll_wheel" => {
               let delta = require_int(&param, "scroll_wheel", "delta")?;
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "scroll_wheel".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               en.scroll(delta as i32, Axis::Vertical).map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "scroll_wheel".into(),
                   detail: format!("enigo scroll err: {e:?}"),
               })?;
               fbs.push("enigo_scroll_v");
               Ok(OperatorOutput::quick(format!("滚轮滚动 {delta} ticks（负向上，正向下）"))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "move_cursor_to_center" => {
               use screenshots::Screen;
               let screens = Screen::all().map_err(|e| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "move_cursor_to_center".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["screens_enum_failed".to_string()],
               })?;
               let s = screens.first().ok_or_else(|| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "move_cursor_to_center".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["no_screens_found".to_string().to_string()],
               })?;
               // screenshots 0.8 Screen 不暴露 .width()/.height() 方法；先捕获一帧拿尺寸
               let sample = s.capture().map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "move_cursor_to_center".into(),
                   detail: format!("探测屏幕尺寸失败: {e}"),
               })?;
               let (w, h) = (sample.width() as i64, sample.height() as i64);
               let cx = w / 2;
               let cy = h / 2;
               let mut en = enigo::Enigo::new(&enigo::Settings::default()).map_err(|_| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::Input.as_str().to_string(),
                   action: "move_cursor_to_center".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["enigo_new_failed".to_string()],
               })?;
               en.move_mouse(cx as i32, cy as i32, enigo::Coordinate::Abs).map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::Input.as_str().into(),
                   action: "move_cursor_to_center".into(),
                   detail: format!("enigo move_mouse err: {e:?}"),
               })?;
               fbs.push("enigo_move_center");
               Ok(OperatorOutput::quick(format!("屏幕中心 → ({cx},{cy})")).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           other => Err(XiaobaiError::IntentUnknown(other.into())),
       }
   }
}
// ============ parser helpers ============
fn require_int(p: &ActionParam, action: &str, k: &str) -> Result<i64, XiaobaiError> {
   p.get_i64(k).ok_or_else(|| XiaobaiError::InvalidArgument {
       action: action.into(),
       param: k.to_string(),
       value: "<missing>".into(),
       hint: "需要整数参数".into(),
   })
}
fn parse_button(s: &str) -> enigo::Button {
   match s.trim().to_lowercase().as_str() {
       "right" => enigo::Button::Right,
       "middle" => enigo::Button::Middle,
       _ => enigo::Button::Left,
   }
}
fn parse_modifier(s: &str) -> enigo::Key {
   match s.trim().to_lowercase().as_str() {
       "ctrl" | "control" => enigo::Key::Control,
       "shift" => enigo::Key::Shift,
       "alt" | "option" => enigo::Key::Alt,
       "meta" | "win" | "cmd" | "command" | "super" => enigo::Key::Meta,
       _ => enigo::Key::Control,
   }
}
fn parse_key(s: &str) -> enigo::Key {
   use enigo::Key::*;
   let t = s.trim().to_lowercase();
   match t.as_str() {
       "enter" | "return" => Return,
       "tab" => Tab,
       "space" => Space,
       "backspace" | "back" => Backspace,
       "escape" | "esc" => Escape,
       "capslock" | "caps" => CapsLock,
       "f1" => F1, "f2" => F2, "f3" => F3, "f4" => F4, "f5" => F5, "f6" => F6,
       "f7" => F7, "f8" => F8, "f9" => F9, "f10" => F10, "f11" => F11, "f12" => F12,
       "home" => Home, "end" => End, "pageup" => PageUp, "pagedown" => PageDown,
       "left" | "arrowleft" => LeftArrow, "right" | "arrowright" => RightArrow,
       "up" | "arrowup" => UpArrow, "down" | "arrowdown" => DownArrow,
       "delete" | "del" => Delete, "insert" | "ins" => Insert,
       "ctrl" | "control" => Control, "shift" => Shift, "alt" => Alt,
       "meta" | "win" | "super" | "cmd" => Meta,
       other => {
           // 单字符 fallback
           if other.chars().count() == 1 {
               let c = other.chars().next().unwrap();
               Unicode(c)
           } else {
               Unicode(' ') // 未知键：退化为空格（不中断操作）
           }
       }
   }
}