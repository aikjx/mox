//! Notify 算子：系统通知 & 桌面反馈（toast_notify / balloon_notify / set_wallpaper / flash_taskbar / lock_workstation）
//!
//! 跨平台回退链：
//! - toast_notify：Windows（PowerShell BurntToast → msg → PowerShell Add-Type WinForms MessageBox）/ macOS(osascript display notification) / Linux(notify-send)
//! - balloon_notify：等同 toast_notify（现代系统统一 Toast；Windows 若任务栏托盘可用则 BalloonTip via WinForms NotifyIcon 尝试）
//! - set_wallpaper：Windows(SystemParametersInfoW SPI_SETDESKWALLPAPER via windows-rs → PowerShell reg+Win32) / Linux(gsettings) / macOS(osascript)
//! - flash_taskbar：Windows(FlashWindowEx user32 → PowerShell)；macOS/Linux 无对应窗口管理器 API → XB-007
//! - lock_workstation：Windows(rundll32 user32.dll,LockWorkStation) / Linux(loginctl lock-sessions) / macOS(pmset displaysleepnow)

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;

use crate::helpers::{platform_tag, run_command, run_command_xb};
use xiaobai_core::errors::{XiaobaiError, XiaobaiResult};
use xiaobai_core::identity::OperatorIdentity;
use xiaobai_core::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use xiaobai_core::rbac::ClearanceLevel;

#[derive(Debug, Default, Clone)]
pub struct NotifyOperator;

impl NotifyOperator {
    // ============ toast_notify ============
    pub(crate) fn toast_impl(&self, title: &str, body: &str, _icon: Option<&str>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let msg = if cfg!(windows) {
            // 回退 1：BurntToast（PowerShell Gallery 模块，已预装企业/家用不同）
            fbs.push("powershell_burnttoast");
            let ps_cmd = format!(
                "$BT = '{{\\n    \"title\" = \"{t}\"\\n    \"text\" = \"{b}\"\\n}}' ; try {{ New-BurntToastNotification -Text '{t}','{b}' -ErrorAction Stop }} catch {{ exit 1 }}",
                t = escape_ps(title),
                b = escape_ps(body),
            );
            let r = run_command("powershell", &["-NoProfile", "-Command", &ps_cmd]);
            if let Ok((_, _, 0)) = r {
                "BurntToast 通知已发送".into()
            } else {
                // 回退 2：msg.exe（所有 Win10/11 自带，弹窗式）
                fbs.push("msg_exe_star");
                let r2 = run_command("msg", &["*", "/TIME:5", &format!("{title}\n{body}")]);
                if let Ok((_, _, 0)) = r2 {
                    "msg.exe 弹窗通知已发送（5 秒自动关闭）".into()
                } else {
                    // 回退 3：WinForms MessageBox（模态，用户手动关闭，最后兜底）
                    fbs.push("winforms_messagebox");
                    let ps3 = format!(
                        "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{b}','{t}') | Out-Null",
                        t = escape_ps(title),
                        b = escape_ps(body),
                    );
                    let r3 = run_command("powershell", &["-NoProfile", "-Command", &ps3]);
                    if let Ok((_, _, 0)) = r3 {
                        "WinForms MessageBox 已弹出".into()
                    } else {
                        return Err(XiaobaiError::OperatorUnsupported {
                            category: OperatorCategory::Notify.as_str().to_string(),
                            action: "toast_notify".into(),
                            platform: platform_tag(),
                            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
        } else if cfg!(target_os = "macos") {
            fbs.push("osascript_display_notification");
            let script = format!(
                "display notification \"{}\" with title \"{}\"",
                escape_osa(body),
                escape_osa(title),
            );
            let (_, _, code) = run_command_xb("osascript", &["-e", &script], OperatorCategory::Notify, "toast_notify")?;
            if code != 0 {
                return Err(XiaobaiError::ExecutionError {
                    category: OperatorCategory::Notify.as_str().into(),
                    action: "toast_notify".into(),
                    detail: "osascript notification exit != 0".into(),
                });
            }
            "macOS 通知中心通知已发送".into()
        } else {
            fbs.push("notify_send");
            let r = run_command("notify-send", &[title, body]);
            if let Ok((_, _, 0)) = r {
                "notify-send 桌面通知已发送".into()
            } else {
                fbs.push("zenity_info");
                let r2 = run_command("zenity", &["--info", "--title", title, "--text", body]);
                if let Ok((_, _, 0)) = r2 {
                    "zenity 信息弹窗已显示".into()
                } else {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Notify.as_str().to_string(),
                        action: "toast_notify".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        };
        Ok((fbs, msg))
    }

    // ============ balloon_notify ============
    pub(crate) fn balloon_impl(&self, title: &str, body: &str, timeout_ms: Option<u32>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        if cfg!(windows) {
            // 优先 WinForms NotifyIcon ShowBalloonTip（需要 PowerShell STA 模式）
            let mut fbs = Vec::new();
            fbs.push("powershell_sta_notifyicon_balloon");
            let tm = timeout_ms.unwrap_or(3000).to_string();
            let ps = format!(
                "Add-Type -AssemblyName System.Windows.Forms; $n=New-Object System.Windows.Forms.NotifyIcon; $n.Icon=[System.Drawing.SystemIcons]::Information; $n.Visible=$true; $n.ShowBalloonTip({},'{t}','{b}',[System.Windows.Forms.ToolTipIcon]::Info); Start-Sleep -Milliseconds {}; $n.Dispose()",
                tm,
                (timeout_ms.unwrap_or(3000) + 500).to_string(),
                t = escape_ps(title),
                b = escape_ps(body),
            );
            let r = run_command("powershell", &["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &ps]);
            if let Ok((_, _, 0)) = r {
                return Ok((fbs, format!("托盘气泡通知已弹出 {}ms", tm)));
            }
            // 回退：复用 toast 链路
            let (fbs2, msg) = self.toast_impl(title, body, None)?;
            fbs.push("toast_fallback");
            fbs.append(&mut fbs2.iter().map(|s| Box::leak((*s).to_owned().into_boxed_str()) as &str).collect());
            Ok((fbs, format!("balloon 托盘不可用，已降级为 Toast：{msg}")))
        } else {
            // 非 Windows 系统没有"托盘气球"概念，全部走 toast 语义等价
            let (fbs, msg) = self.toast_impl(title, body, None)?;
            Ok((fbs, format!("balloon 已等价 toast：{msg}")))
        }
    }

    // ============ set_wallpaper ============
    pub(crate) fn set_wallpaper_impl(&self, abs_path: &str) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        // 先验：文件存在（否则 UI 直接报错，避免静默无效）
        if !std::path::Path::new(abs_path).exists() {
            return Err(XiaobaiError::InvalidArgument {
                action: "set_wallpaper".into(),
                param: "abs_path".into(),
                value: abs_path.into(),
                hint: "壁纸文件不存在；请传入绝对路径的 JPG/PNG/BMP".into(),
            });
        }
        let msg = if cfg!(windows) {
            // 回退 1：windows-rs SystemParametersInfoW SPI_SETDESKWALLPAPER
            fbs.push("win32_spi_setdeskwallpaper");
            match set_wallpaper_windows_rs(abs_path) {
                Ok(m) => m,
                Err(_e) => {
                    // 回退 2：PowerShell + reg + Win32 API（P/Invoke）
                    fbs.push("powershell_reg_spi_setdeskwallpaper");
                    let ps = format!(
                        r#"
$code = @'
using System.Runtime.InteropServices;
public class W {{
    [DllImport("user32.dll", CharSet=CharSet.Auto)]
    public static extern int SystemParametersInfo(int uAction, int uParam, string lpvParam, int fuWinIni);
}}
'@
Add-Type $code;
$path = '{p}';
[W]::SystemParametersInfo(0x0014, 0, $path, 0x01 -bor 0x02) | Out-Null;
Set-ItemProperty -Path 'HKCU:\Control Panel\Desktop' -Name Wallpaper -Value $path;
rundll32.exe user32.dll, UpdatePerUserSystemParameters ,1 ,True
"#,
                        p = escape_ps(abs_path),
                    );
                    let r = run_command("powershell", &["-NoProfile", "-Command", &ps]);
                    match r {
                        Ok((_, _, 0)) => "已设置壁纸（PowerShell P/Invoke SPI_SETDESKWALLPAPER + 注册表）".into(),
                        _ => {
                            return Err(XiaobaiError::OperatorUnsupported {
                                category: OperatorCategory::Notify.as_str().to_string(),
                                action: "set_wallpaper".into(),
                                platform: platform_tag(),
                                fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                            });
                        }
                    }
                }
            }
        } else if cfg!(target_os = "linux") {
            fbs.push("gsettings_set_picture_uri");
            let uri = format!("file://{abs_path}");
            // GNOME 优先
            let r_gnome = run_command("gsettings", &["set", "org.gnome.desktop.background", "picture-uri", &uri]);
            let r_gnome_dark = run_command("gsettings", &["set", "org.gnome.desktop.background", "picture-uri-dark", &uri]);
            if matches!(r_gnome, Ok((_, _, 0))) || matches!(r_gnome_dark, Ok((_, _, 0))) {
                format!("gsettings GNOME 壁纸已设置：{uri}")
            } else {
                // KDE
                fbs.push("plasma_script_wallpaper");
                let plasma = format!(
                    "
var allDesktops = desktops();
for (i=0;i<allDesktops.length;i++) {{
    d = allDesktops[i];
    d.wallpaperPlugin = 'org.kde.image';
    d.currentConfigGroup = Array('Wallpaper','org.kde.image','General');
    d.writeConfig('Image', '{uri}');
}}
"
                );
                let r = run_command("qdbus", &[
                    "org.kde.plasmashell",
                    "/PlasmaShell",
                    "org.kde.PlasmaShell.evaluateScript",
                    &plasma,
                ]);
                if let Ok((_, _, 0)) = r {
                    format!("KDE Plasma 壁纸已设置：{uri}")
                } else {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Notify.as_str().to_string(),
                        action: "set_wallpaper".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        } else {
            // macOS：osascript tell Finder set desktop picture
            fbs.push("osascript_finder_set_desktop_picture");
            let script = format!(
                "tell application \"System Events\" to set picture of every desktop to POSIX file \"{p}\"",
                p = abs_path
            );
            let (_, _, code) = run_command_xb("osascript", &["-e", &script], OperatorCategory::Notify, "set_wallpaper")?;
            if code != 0 {
                return Err(XiaobaiError::OperatorUnsupported {
                    category: OperatorCategory::Notify.as_str().to_string(),
                    action: "set_wallpaper".into(),
                    platform: platform_tag(),
                    fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                });
            }
            format!("macOS Finder 桌面壁纸已设置：{abs_path}")
        };
        Ok((fbs, msg))
    }

    // ============ flash_taskbar ============
    pub(crate) fn flash_impl(&self, duration_ms: Option<u32>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        if !cfg!(windows) {
            return Err(XiaobaiError::OperatorUnsupported {
                category: OperatorCategory::Notify.as_str().to_string(),
                action: "flash_taskbar".into(),
                platform: platform_tag(),
                fallbacks_used: vec!["only_windows_supported".into()],
            });
        }
        // Windows：PowerShell FlashWindowEx
        fbs.push("powershell_flashwindowex");
        let ms = duration_ms.unwrap_or(2000).to_string();
        let ps = format!(
            r#"
$code = @'
using System;
using System.Runtime.InteropServices;
public class FX {{
    [StructLayout(LayoutKind.Sequential)] public struct FLASHWINFO {{
        public int cbSize; public IntPtr hwnd; public int dwFlags; public int uCount; public int dwTimeout;
    }}
    [DllImport("user32.dll")] public static extern bool FlashWindowEx(ref FLASHWINFO pfwi);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}}
'@
Add-Type $code;
$fwi = New-Object FX+FLASHWINFO;
$fwi.cbSize = [Runtime.InteropServices.Marshal]::SizeOf($fwi);
$fwi.hwnd = [FX]::GetForegroundWindow();
$fwi.dwFlags = 0x0000000C -bor 0x00000004; # FLASHW_ALL -bor FLASHW_TIMERNOFG
$fwi.uCount = 5;
$fwi.dwTimeout = 0;
[void][FX]::FlashWindowEx([ref]$fwi);
Start-Sleep -Milliseconds {ms};
"#
        );
        let (_, _, code) = run_command("powershell", &["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps])
            .map_err(|e| XiaobaiError::ExecutionError {
                category: OperatorCategory::Notify.as_str().into(),
                action: "flash_taskbar".into(),
                detail: format!("flash ps script failed: {e}"),
            })?;
        if code != 0 {
            return Err(XiaobaiError::ExecutionError {
                category: OperatorCategory::Notify.as_str().into(),
                action: "flash_taskbar".into(),
                detail: "flash taskbar exit != 0".into(),
            });
        }
        Ok((fbs, format!("前台窗口任务栏闪烁 {ms}ms（FLASHW_ALL until active）")))
    }

    // ============ lock_workstation ============
    pub(crate) fn lock_impl(&self) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let msg = if cfg!(windows) {
            fbs.push("rundll32_user32_lockworkstation");
            let (_, _, code) = run_command_xb("rundll32", &["user32.dll,LockWorkStation"], OperatorCategory::Notify, "lock_workstation")?;
            if code != 0 {
                // 回退：Logoff 类脚本不推荐；直接失败
                return Err(XiaobaiError::ExecutionError {
                    category: OperatorCategory::Notify.as_str().into(),
                    action: "lock_workstation".into(),
                    detail: "rundll32 LockWorkStation exit != 0".into(),
                });
            }
            "已调用 LockWorkStation 锁定桌面（需恢复需重新登录）".into()
        } else if cfg!(target_os = "linux") {
            fbs.push("loginctl_lock_sessions");
            let r = run_command("loginctl", &["lock-sessions"]);
            if let Ok((_, _, 0)) = r {
                "loginctl 已锁所有 session".into()
            } else {
                fbs.push("xdg_screensaver_lock");
                let r2 = run_command("xdg-screensaver", &["lock"]);
                if let Ok((_, _, 0)) = r2 {
                    "xdg-screensaver lock 已触发".into()
                } else {
                    return Err(XiaobaiError::OperatorUnsupported {
                        category: OperatorCategory::Notify.as_str().to_string(),
                        action: "lock_workstation".into(),
                        platform: platform_tag(),
                        fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
                    });
                }
            }
        } else {
            // macOS：pmset displaysleepnow（如果已启用 "唤醒需密码" 即为锁屏）
            fbs.push("pmset_displaysleepnow");
            let (_, _, code) = run_command_xb("pmset", &["displaysleepnow"], OperatorCategory::Notify, "lock_workstation")?;
            if code != 0 {
                return Err(XiaobaiError::ExecutionError {
                    category: OperatorCategory::Notify.as_str().into(),
                    action: "lock_workstation".into(),
                    detail: "pmset displaysleepnow exit != 0".into(),
                });
            }
            "macOS displaysleepnow 已触发（若已勾选唤醒需要密码，即完成锁屏）".into()
        };
        Ok((fbs, msg))
    }
}

// ========== Windows helpers ==========
#[cfg(windows)]
fn set_wallpaper_windows_rs(abs_path: &str) -> XiaobaiResult<String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER};
    let wide: Vec<u16> = abs_path.encode_utf16().chain(Some(0)).collect();
    let pw = PCWSTR(wide.as_ptr());
    let ok = unsafe {
        use core::ffi::c_void;
        SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(pw.0 as *mut c_void),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };
    if ok.is_ok() {
        Ok("已设置壁纸（windows-rs SPI_SETDESKWALLPAPER）".into())
    } else {
        Err(XiaobaiError::ExecutionError {
            category: OperatorCategory::Notify.as_str().into(),
            action: "set_wallpaper".into(),
            detail: "SystemParametersInfoW returned false".into(),
        })
    }
}
#[cfg(not(windows))]
fn set_wallpaper_windows_rs(_abs_path: &str) -> XiaobaiResult<String> {
    Err(XiaobaiError::OperatorUnsupported {
        category: OperatorCategory::Notify.as_str().to_string(),
        action: "set_wallpaper".into(),
        platform: platform_tag(),
        fallbacks_used: vec!["win32_skipped_not_windows".into()],
    })
}

fn escape_ps(s: &str) -> String {
    // PowerShell single-quote escape: ' → ''
    s.replace('\'', "''")
}
fn escape_osa(s: &str) -> String {
    // AppleScript: \ → \\, " → \"
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[async_trait]
impl SystemOperator for NotifyOperator {
    fn id(&self) -> &'static str {
        "notify_operator_v1"
    }
    fn category(&self) -> OperatorCategory {
        OperatorCategory::Notify
    }
    fn list_actions(&self) -> Vec<ActionSignature> {
        use ClearanceLevel::*;
        let mut p_toast = BTreeMap::new();
        p_toast.insert("title", "string，通知标题（短，1 行）");
        p_toast.insert("body", "string，通知正文；可选 icon 字符串（暂仅 Windows 预留）");
        let mut p_balloon = BTreeMap::new();
        p_balloon.insert("title", "同 toast_notify");
        p_balloon.insert("body", "同 toast_notify");
        p_balloon.insert("timeout_ms", "int，可选，气泡显示时长毫秒（默认 3000）");
        let mut p_wp = BTreeMap::new();
        p_wp.insert("abs_path", "string，壁纸图片绝对路径（JPG/PNG/BMP，需存在）");
        let mut p_flash = BTreeMap::new();
        p_flash.insert("duration_ms", "int，可选，闪烁时间毫秒（默认 2000，实际由 FlashWindowEx 决定）");
        vec![
            ActionSignature {
                name: "toast_notify",
                category: OperatorCategory::Notify,
                clearance: L1,
                own_qualified: false,
                description: "发送系统通知（BurntToast → msg.exe → WinForms；通知中心 → notify-send → zenity）",
                params: Some(p_toast),
            },
            ActionSignature {
                name: "balloon_notify",
                category: OperatorCategory::Notify,
                clearance: L1,
                own_qualified: false,
                description: "托盘气泡通知（Windows NotifyIcon ShowBalloonTip；非 Windows 等价 toast）",
                params: Some(p_balloon),
            },
            ActionSignature {
                name: "set_wallpaper",
                category: OperatorCategory::Notify,
                clearance: L2,
                own_qualified: false,
                description: "设置桌面壁纸：windows-rs SPI_SETDESKWALLPAPER → PowerShell P/Invoke → gsettings/KDE → System Events Finder",
                params: Some(p_wp),
            },
            ActionSignature {
                name: "flash_taskbar",
                category: OperatorCategory::Notify,
                clearance: L2,
                own_qualified: false,
                description: "Windows 前台窗口任务栏闪烁（FlashWindowEx）；macOS/Linux XB-007",
                params: Some(p_flash),
            },
            ActionSignature {
                name: "lock_workstation",
                category: OperatorCategory::Notify,
                clearance: L3,
                own_qualified: true,
                description: "锁定本机工作站（rundll32 LockWorkStation / loginctl lock / pmset displaysleepnow），L3 破坏性保护；Own 场景 L2 可",
                params: None,
            },
        ]
    }
    async fn execute(
        &self,
        action: &str,
        param: ActionParam,
        _identity: &OperatorIdentity,
    ) -> XiaobaiResult<OperatorOutput> {
        let t0 = Instant::now();
        match action {
            "toast_notify" => {
                let title = param.get_str("title").unwrap_or("小白语音通知");
                let body = param.get_str("body").unwrap_or("（无正文）");
                let icon = param.get_str("icon");
                let (fbs, msg) = self.toast_impl(title, body, icon)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "balloon_notify" => {
                let title = param.get_str("title").unwrap_or("小白语音托盘提示");
                let body = param.get_str("body").unwrap_or("（无正文）");
                let timeout_ms = param.get_i64("timeout_ms").map(|v| v.clamp(500, 600_000) as u32);
                let (fbs, msg) = self.balloon_impl(title, body, timeout_ms)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "set_wallpaper" => {
                let path = param.get_str("abs_path").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "set_wallpaper".into(),
                    param: "abs_path".into(),
                    value: "<missing>".into(),
                    hint: "需要 abs_path 绝对路径".into(),
                })?;
                let (fbs, msg) = self.set_wallpaper_impl(path)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "flash_taskbar" => {
                let dur = param.get_i64("duration_ms").map(|v| v.clamp(100, 60_000) as u32);
                let (fbs, msg) = self.flash_impl(dur)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "lock_workstation" => {
                let (fbs, msg) = self.lock_impl()?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            other => Err(XiaobaiError::IntentUnknown(other.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_ps_and_osa() {
        assert_eq!(escape_ps("a'b"), "a''b");
        assert_eq!(escape_osa(r#"say "hello\"#), r#"say \"hello\\"#);
    }

    #[test]
    fn list_actions_5_covered() {
        let op = NotifyOperator::default();
        let acts = op.list_actions();
        assert_eq!(acts.len(), 5);
        let names: Vec<_> = acts.iter().map(|a| a.name).collect();
        for n in ["toast_notify", "balloon_notify", "set_wallpaper", "flash_taskbar", "lock_workstation"] {
            assert!(names.contains(&n), "missing {n}");
        }
        // 锁屏：L3，Own=true
        let lock = acts.iter().find(|a| a.name == "lock_workstation").unwrap();
        assert_eq!(lock.clearance, ClearanceLevel::L3);
        assert!(lock.own_qualified);
    }
}
