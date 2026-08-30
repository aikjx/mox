// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 壁纸设置实现

use crate::helpers::{platform_tag, run_command, run_command_xb};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::operator::OperatorCategory;

use super::common::escape_ps;
use super::NotifyOperator;

impl NotifyOperator {
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
