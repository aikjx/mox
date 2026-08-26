//! Volume 算子：get/set 音量、静音、设备枚举（跨平台）
//!
//! 回退链（与 Python volume_operator 一致，减少平台差 bug）：
//! - Windows：pycaw 等价 = 未来 windows.Media.Audio + IAudioEndpointVolume → 这里先回退注册表 + nircmd（XB-007 时用户可装 nircmd）
//! - macOS：osascript -e 'set volume output volume N' / osascript get volume
//! - Linux：pactl set-sink-volume @DEFAULT_SINK@ +N% → 回退 amixer sset Master N%
//! - 无头环境/缺失命令 → XB-007 OperatorUnsupported
use std::time::Instant;
use async_trait::async_trait;
use serde_json::json;
use crate::helpers::{platform_tag, run_command};
use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
   ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;
#[derive(Debug, Default, Clone)]
pub struct VolumeOperator;
impl VolumeOperator {
   fn get_impl(&self) -> Result<(Vec<&'static str>, Option<i32>, Vec<String>), XiaobaiError> {
       let mut fb = Vec::new();
       if cfg!(windows) {
           fb.push("nircmd_get_volume_writes_to_file_not_supported");
       } else if cfg!(target_os = "macos") {
           let r = run_command("osascript", &["-e", "output volume of (get volume settings)"]);
           fb.push("osascript_get_output_volume");
           if let Ok((stdout, _, 0)) = r {
               let n: Option<i32> = stdout.trim().parse::<i32>().ok();
               return Ok((fb, n, vec!["default".into()]));
           }
       } else {
           // pactl list sinks short → 设备列表；get-sink-volume → 当前音量
           let r1 = run_command("bash", &["-c", "pactl list short sinks || echo 'NO_PACTL'"]);
           fb.push("pactl_list_short_sinks");
           let devices: Vec<String> = match r1 {
               Ok((out, _, 0)) if !out.contains("NO_PACTL") => {
                   out.lines()
                       .filter(|l| !l.trim().is_empty())
                       .filter_map(|l| l.split_whitespace().nth(1).map(|s| s.to_string()))
                       .collect()
               }
               _ => Vec::new(),
           };
           let r2 = run_command(
               "bash",
               &["-c", "pactl get-sink-volume @DEFAULT_SINK@ 2>/dev/null | head -1"],
           );
           fb.push("pactl_get_sink_volume_default");
           let vol = r2.ok().and_then(|(out, _, _)| {
               // "Volume: front-left: 26163 /  40% / -24.10 dB,   front-right: 26163 /  40% / ..."
               out.split('%').next().and_then(|s| s.rsplit(' ').last())?.trim().parse::<i32>().ok()
           });
           if vol.is_some() || !devices.is_empty() {
               return Ok((fb, vol, devices));
           }
           // 回退 amixer
           let r3 = run_command("amixer", &["sget", "Master"]);
           fb.push("amixer_sget_Master");
           if let Ok((out, _, 0)) = r3 {
               let n: Option<i32> = out
                   .lines()
                   .find(|l| l.contains("Playback") && l.contains('%'))
                   .and_then(|l| l.split('%').next()?.rsplit('[').last()?.parse::<i32>().ok());
               return Ok((fb, n, vec!["Master (amixer)".into()]));
           }
       }
       Err(XiaobaiError::OperatorUnsupported {
           category: OperatorCategory::Volume.as_str().to_string(),
           action: "get_volume / list_devices".into(),
           platform: platform_tag(),
           fallbacks_used: fb.iter().map(|s| s.to_string()).collect(),
       })
   }
   fn set_impl(&self, pct: i32) -> Result<(Vec<&'static str>, String), XiaobaiError> {
       let pct = pct.max(0).min(100);
       let mut fb = Vec::new();
       if cfg!(target_os = "macos") {
           let s = pct.to_string();
           let r = run_command("osascript", &["-e", &format!("set volume output volume {s}")]);
           fb.push("osascript_set_output_volume");
           if matches!(r, Ok((_, _, 0))) {
               return Ok((fb, format!("macOS 音量已设置为 {pct}%")));
           }
       } else if cfg!(windows) {
           // 调用 nircmd.exe setsysvolume / setvolume（需用户环境有 nircmd，失败记 fallback）
           let try_cmds: [(&[&str], &str); 2] = [
               (
                   &["nircmd.exe", "setsysvolume", &((pct as u32) * 65535 / 100).to_string()],
                   "nircmd_setsysvolume",
               ),
               (
                   &["nircmdc.exe", "setsysvolume", &((pct as u32) * 65535 / 100).to_string()],
                   "nircmdc_setsysvolume",
               ),
           ];
           for (args, tag) in try_cmds.iter() {
               let r = run_command(args[0], &args[1..]);
               fb.push(tag);
               if matches!(r, Ok((_, _, 0))) {
                   return Ok((fb, format!("Windows nircmd 设置音量 {pct}%")));
               }
           }
       } else {
           let s = format!("{pct}%");
           let r = run_command("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &s]);
           fb.push("pactl_set-sink-volume_DEFAULT");
           if matches!(r, Ok((_, _, 0))) {
               return Ok((fb, format!("Linux pactl 设置音量 {pct}%")));
           }
           let r2 = run_command("amixer", &["sset", "Master", &format!("{s} unmute")]);
           fb.push("amixer_sset_Master");
           if matches!(r2, Ok((_, _, 0))) {
               return Ok((fb, format!("Linux amixer Master 音量 {pct}%（unmute）")));
           }
       }
       Err(XiaobaiError::OperatorUnsupported {
           category: OperatorCategory::Volume.as_str().to_string(),
           action: "set_volume".into(),
           platform: platform_tag(),
           fallbacks_used: fb.iter().map(|s| s.to_string()).collect(),
       })
   }
   fn mute_impl(&self, mute: bool, toggle: bool) -> Result<(Vec<&'static str>, String), XiaobaiError> {
       let mut fb = Vec::new();
       if cfg!(target_os = "macos") {
           let arg = if toggle {
               "output muted of (get volume settings) is false"
           } else if mute {
               "true"
           } else {
               "false"
           };
           let script = if toggle {
               format!(
                   "set currentMuted to {arg}\nif currentMuted then\n  set volume with output muted false\nelse\n  set volume with output muted true\nend if"
               )
           } else {
               format!("set volume output muted {arg}")
           };
           let r = run_command("osascript", &["-e", &script]);
           fb.push("osascript_output_muted");
           if matches!(r, Ok((_, _, 0))) {
               return Ok((fb, format!("macOS mute={mute} toggle={toggle} 完成")));
           }
       } else if cfg!(windows) {
           fb.push("nircmd_mutesysvolume_todo");
       } else {
           let (cmd_arg, action_label) = if toggle {
               (vec!["set-sink-mute", "@DEFAULT_SINK@", "toggle"], "toggle")
           } else if mute {
               (vec!["set-sink-mute", "@DEFAULT_SINK@", "1"], "mute")
           } else {
               (vec!["set-sink-mute", "@DEFAULT_SINK@", "0"], "unmute")
           };
           let r = run_command("pactl", &cmd_arg.iter().map(|s| *s).collect::<Vec<_>>());
           fb.push("pactl_set-sink-mute_DEFAULT");
           if matches!(r, Ok((_, _, 0))) {
               return Ok((fb, format!("pactl mute action={action_label} 成功")));
           }
           let amixer_arg = if toggle {
               vec!["sset", "Master", "toggle"]
           } else if mute {
               vec!["sset", "Master", "mute"]
           } else {
               vec!["sset", "Master", "unmute"]
           };
           let r2 = run_command("amixer", &amixer_arg.iter().map(|s| *s).collect::<Vec<_>>());
           fb.push("amixer_sset_Master_mute");
           if matches!(r2, Ok((_, _, 0))) {
               return Ok((fb, format!("amixer mute action={action_label} 成功")));
           }
       }
       Err(XiaobaiError::OperatorUnsupported {
           category: OperatorCategory::Volume.as_str().to_string(),
           action: if toggle { "toggle_mute" } else if mute { "mute" } else { "unmute" }.into(),
           platform: platform_tag(),
           fallbacks_used: fb.iter().map(|s| s.to_string()).collect(),
       })
   }
}
#[async_trait]
impl SystemOperator for VolumeOperator {
   fn id(&self) -> &'static str {
       "volume_operator_v1"
   }
   fn category(&self) -> OperatorCategory {
       OperatorCategory::Volume
   }
   fn list_actions(&self) -> Vec<ActionSignature> {
       use ClearanceLevel::*;
       use std::collections::BTreeMap;
       let mut p_set = BTreeMap::new();
       p_set.insert("percent", "int 0~100；0 视为静音需 L3 MoxAdmin 权限（强制静音是破坏性动作）");
       vec![
           ActionSignature {
               name: "get_volume",
               category: OperatorCategory::Volume,
               clearance: L0,
               own_qualified: false,
               description: "只读：获取系统主音量百分比（None 表示未知）",
               params: None,
           },
           ActionSignature {
               name: "list_devices",
               category: OperatorCategory::Volume,
               clearance: L0,
               own_qualified: false,
               description: "只读：枚举音频输出设备（pactl sinks / osasound cards）",
               params: None,
           },
           ActionSignature {
               name: "set_volume",
               category: OperatorCategory::Volume,
               clearance: L1,
               own_qualified: false,
               description: "设置主音量 percent ∈ [1,100]（L1）；传 0 强制静音会提升到 L3",
               params: Some(p_set),
           },
           ActionSignature {
               name: "mute",
               category: OperatorCategory::Volume,
               clearance: L1,
               own_qualified: false,
               description: "静音（不强制 L3，与 set_volume 0 区分）",
               params: None,
           },
           ActionSignature {
               name: "unmute",
               category: OperatorCategory::Volume,
               clearance: L1,
               own_qualified: false,
               description: "取消静音",
               params: None,
           },
           ActionSignature {
               name: "toggle_mute",
               category: OperatorCategory::Volume,
               clearance: L1,
               own_qualified: false,
               description: "切换静音/非静音",
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
       match action {
           "get_volume" | "list_devices" => {
               let (fb, v, devs) = self.get_impl()?;
               let is_get = action == "get_volume";
               let msg = if is_get {
                   format!("主音量：{}%（返回 devices={}）", v.unwrap_or(-1), devs.len())
               } else {
                   format!("音频设备：{} 个，主音量：{}%", devs.len(), v.unwrap_or(-1))
               };
               Ok(OperatorOutput::quick(msg)
                   .with_payload(json!({"devices": devs, "volume_pct": v}))
                   .with_fallbacks(fb.iter().map(|s| s.to_string()).collect())
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "set_volume" => {
               let pct = param.get_i64("percent").ok_or_else(|| XiaobaiError::InvalidArgument {
                   action: "set_volume".into(),
                   param: "percent".into(),
                   value: "<missing>".into(),
                   hint: "需要 percent ∈ int [0,100]；0 将被 Engine 在 PII 层提升到 L3".into(),
               })?;
               // 参数内提前兜底：<0 按 0，>100 按 100
               let clamped = pct.max(0).min(100) as i32;
               let (fb, msg) = self.set_impl(clamped)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fb.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "mute" => {
               let (fb, msg) = self.mute_impl(true, false)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fb.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "unmute" => {
               let (fb, msg) = self.mute_impl(false, false)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fb.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "toggle_mute" => {
               let (fb, msg) = self.mute_impl(false, true)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fb.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           other => Err(XiaobaiError::IntentUnknown(other.into())),
       }
   }
}