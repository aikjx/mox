// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! App 算子：应用控制（list_running / open_app / open_file_with_app / close_app / shell_exec）
//!
//! 跨平台回退链：
//! - list_running：Windows(tasklist) / macOS(ps -Ao) / Linux(ps -Ao) → 未来接入 windows-rs ProcessStatus EnumProcesses
//! - open_app：Windows(cmd /c start) / macOS(open) / Linux(xdg-open)
//! - close_app：Windows(taskkill /IM /F) / macOS(killall -9) / Linux(pkill -9)
//! - shell_exec(L3)：统一 std::process::Command，捕获 stdout/stderr/exit_code
use std::collections::BTreeMap;
use std::time::Instant;
use async_trait::async_trait;
use serde_json::json;
use crate::helpers::{platform_tag, run_command};
use mox_voice_core_svc::errors::XiaobaiResult;
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
   ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;
#[derive(Debug, Default, Clone)]
pub struct AppOperator;
impl AppOperator {
   pub(crate) fn open_app_impl(&self, app_name: &str) -> Result<(Vec<&'static str>, String), mox_voice_core_svc::XiaobaiError> {
       let mut fallbacks = Vec::new();
       let exec_name = normalize_app_exec(app_name);
       if cfg!(windows) {
           // 回退1：cmd /c start "" <name>（关联可执行/协议）
           let r = run_command("cmd", &["/c", "start", "", &exec_name]);
           fallbacks.push("cmd_start");
           match r {
               Ok((_, _, code)) if code == 0 => return Ok((fallbacks, format!("已启动：{}", exec_name))),
               _ => {}
           }
           // 回退2：直接创建子进程
           let r = run_command(&exec_name, &[]);
           fallbacks.push("direct_spawn");
           if let Ok((_, _, code)) = r {
               if code == 0 {
                   return Ok((fallbacks, format!("已启动进程：{}", exec_name)));
               }
           }
       } else if cfg!(target_os = "macos") {
           let r = run_command("open", &["-a", &exec_name]);
           fallbacks.push("open_-a");
           if let Ok((_, _, 0)) = r {
               return Ok((fallbacks, format!("已通过 macOS open 启动：{}", exec_name)));
           }
       } else {
           let r = run_command("xdg-open", &[&exec_name]);
           fallbacks.push("xdg-open");
           if let Ok((_, _, 0)) = r {
               return Ok((fallbacks, format!("已通过 xdg-open 启动：{}", exec_name)));
           }
       }
       Err(mox_voice_core_svc::XiaobaiError::OperatorUnsupported {
           category: OperatorCategory::App.as_str().to_string(),
           action: "open_app".into(),
           platform: platform_tag(),
           fallbacks_used: fallbacks.iter().map(|s| s.to_string()).collect(),
       })
   }
   pub(crate) fn list_running_impl(&self) -> Result<(Vec<&'static str>, Vec<BTreeMap<String, String>>), mox_voice_core_svc::XiaobaiError> {
       let mut fallbacks = Vec::new();
       let rows = if cfg!(windows) {
           let (stdout, _, _) = run_command("tasklist", &["/FO", "CSV", "/NH"]).map_err(|e| {
               mox_voice_core_svc::XiaobaiError::ExecutionError {
                   category: OperatorCategory::App.as_str().into(),
                   action: "list_running".into(),
                   detail: format!("tasklist exec failed: {e}"),
               }
           })?;
           fallbacks.push("tasklist_csv");
           parse_csv_rows(&stdout, &["image_name", "pid", "session_name", "session_num", "mem_kb"])
       } else {
           let (stdout, _, _) = run_command("ps", &["-Ao", "pid=,comm=,rss="]).map_err(|e| {
               mox_voice_core_svc::XiaobaiError::ExecutionError {
                   category: OperatorCategory::App.as_str().into(),
                   action: "list_running".into(),
                   detail: format!("ps exec failed: {e}"),
               }
           })?;
           fallbacks.push("ps_-Ao_pid_comm_rss");
           parse_ps_rows(&stdout)
       };
       Ok((fallbacks, rows))
   }
   pub(crate) fn close_app_impl(&self, app_name: &str) -> Result<(Vec<&'static str>, String), mox_voice_core_svc::XiaobaiError> {
       let exec_name = normalize_app_exec(app_name);
       let mut fallbacks = Vec::new();
       let (cmd, args_owned): (&str, Vec<String>) = if cfg!(windows) {
           fallbacks.push("taskkill_IM_F");
           let img = if !exec_name.ends_with(".exe") {
               format!("{exec_name}.exe")
           } else {
               exec_name.to_string()
           };
           ("taskkill", vec!["/IM".into(), img, "/F".into()])
       } else {
           fallbacks.push("pkill_-9");
           ("pkill", vec!["-9".into(), exec_name.clone()])
       };
       let args_refs: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
       let (stdout, stderr, code) = run_command(cmd, &args_refs).map_err(|e| {
           mox_voice_core_svc::XiaobaiError::ExecutionError {
               category: OperatorCategory::App.as_str().into(),
               action: "close_app".into(),
               detail: format!("{cmd} {args_refs:?} failed: {e}"),
           }
       })?;
       if code != 0 {
           return Err(mox_voice_core_svc::XiaobaiError::ExecutionError {
               category: OperatorCategory::App.as_str().into(),
               action: "close_app".into(),
               detail: format!("{cmd} exit_code={code} stderr={stderr} stdout={stdout}"),
           });
       }
       Ok((fallbacks, format!("已强制关闭进程：{}", exec_name)))
   }
}
#[async_trait]
impl SystemOperator for AppOperator {
   fn id(&self) -> &'static str {
       "app_operator_v1"
   }
   fn category(&self) -> OperatorCategory {
       OperatorCategory::App
   }
   fn list_actions(&self) -> Vec<ActionSignature> {
       use ClearanceLevel::*;
       let mut p = BTreeMap::new();
       p.insert("app_name", "string，应用别名或可执行文件名，如 chrome/notepad/微信");
       let p_open = Some(p.clone());
       let mut p_close = BTreeMap::new();
       p_close.insert("app_name", "同 open_app");
       let mut p_shell = BTreeMap::new();
       p_shell.insert("command", "string，命令名；可选 args=string[] 数组；L3 MoxAdmin");
       let mut p_file = BTreeMap::new();
       p_file.insert("path", "string，文件绝对路径；可选 app_name 指定打开方式");
       vec![
           ActionSignature {
               name: "list_running",
               category: OperatorCategory::App,
               clearance: L0,
               own_qualified: false,
               description: "列出当前系统进程（image_name/pid/mem_kb）",
               params: None,
           },
           ActionSignature {
               name: "open_app",
               category: OperatorCategory::App,
               clearance: L1,
               own_qualified: false,
               description: "通过系统 open/start/xdg-open 启动应用或协议（可识别 40 项中文别名）",
               params: p_open,
           },
           ActionSignature {
               name: "open_file_with_app",
               category: OperatorCategory::App, // 注册在 App 算子，File 算子别名也会走同一条
               clearance: L1,
               own_qualified: false,
               description: "用系统默认或指定应用打开一个本地文件",
               params: Some(p_file),
           },
           ActionSignature {
               name: "close_app",
               category: OperatorCategory::App,
               clearance: L3,
               own_qualified: false,
               description: "强制结束进程（taskkill/pkill -9），破坏性 L3 MoxAdmin 权限",
               params: Some(p_close),
           },
           ActionSignature {
               name: "shell_exec",
               category: OperatorCategory::App,
               clearance: L3,
               own_qualified: false,
               description: "直接执行一段命令行（stdout/stderr/exit_code 全返回），L3 一票",
               params: Some(p_shell),
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
           "list_running" => {
               let (fbs, rows) = self.list_running_impl()?;
               Ok(OperatorOutput::quick(format!("当前进程 {} 个", rows.len()))
                   .with_payload(json!(rows))
                   .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "open_app" => {
               let name = param.get_str("app_name").ok_or_else(|| mox_voice_core_svc::XiaobaiError::InvalidArgument {
                   action: "open_app".into(),
                   param: "app_name".into(),
                   value: "<missing>".into(),
                   hint: "需要 app_name 字符串参数，如 微信/Chrome/记事本".into(),
               })?;
               let (fbs, msg) = self.open_app_impl(name)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "open_file_with_app" => {
               let path = param.get_str("path").ok_or_else(|| mox_voice_core_svc::XiaobaiError::InvalidArgument {
                   action: "open_file_with_app".into(),
                   param: "path".into(),
                   value: "<missing>".into(),
                   hint: "需要 path 绝对路径参数".into(),
               })?;
               let (fbs, msg) = self.open_app_impl(path)?;
               Ok(OperatorOutput::quick(format!("open_file: {msg}")).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "close_app" => {
               let name = param.get_str("app_name").ok_or_else(|| mox_voice_core_svc::XiaobaiError::InvalidArgument {
                   action: "close_app".into(),
                   param: "app_name".into(),
                   value: "<missing>".into(),
                   hint: "需要 app_name 参数".into(),
               })?;
               let (fbs, msg) = self.close_app_impl(name)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fbs.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "shell_exec" => {
               let cmd = param.get_str("command").ok_or_else(|| mox_voice_core_svc::XiaobaiError::InvalidArgument {
                   action: "shell_exec".into(),
                   param: "command".into(),
                   value: "<missing>".into(),
                   hint: "需要 command 字符串".into(),
               })?;
               let args_vec = param.0.get("args").and_then(|v| v.as_array()).cloned().unwrap_or_default();
               let args: Vec<String> = args_vec.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
               let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
               let (stdout, stderr, code) = run_command(cmd, &args_refs).map_err(|e| mox_voice_core_svc::XiaobaiError::ExecutionError {
                   category: OperatorCategory::App.as_str().into(),
                   action: "shell_exec".into(),
                   detail: format!("{cmd} {args_refs:?} failed: {e}"),
               })?;
               let fb = vec!["std_process_command"];
               Ok(OperatorOutput::quick(format!("shell 执行完毕，exit_code={code}"))
                   .with_payload(json!({"stdout": stdout, "stderr": stderr, "exit_code": code}))
                   .with_fallbacks(fb.iter().map(|s| s.to_string()).collect())
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           other => Err(mox_voice_core_svc::XiaobaiError::IntentUnknown(other.into())),
       }
   }
}
// ============ Helpers ============
/// 中文/英文 40 项别名 → 真实可执行文件名（与 Python intent/app_aliases 同步）
pub fn normalize_app_exec(alias: &str) -> String {
   let a = alias.trim().to_lowercase().replace([' ', '（', '）', '(', ')', '·', '-', '_'], "");
   // 精确映射
   let exact: &[(&str, &str)] = &[
       ("记事本", "notepad.exe"),
       ("notepad", "notepad.exe"),
       ("计算器", "calc.exe"),
       ("calc", "calc.exe"),
       ("画图", "mspaint.exe"),
       ("mspaint", "mspaint.exe"),
       ("资源管理器", "explorer.exe"),
       ("explorer", "explorer.exe"),
       ("任务管理器", "taskmgr.exe"),
       ("taskmgr", "taskmgr.exe"),
       ("chrome", "chrome.exe"),
       ("谷歌浏览器", "chrome.exe"),
       ("googlechrome", "chrome.exe"),
       ("edge", "msedge.exe"),
       ("微软浏览器", "msedge.exe"),
       ("edgemicrosoft", "msedge.exe"),
       ("firefox", "firefox.exe"),
       ("火狐浏览器", "firefox.exe"),
       ("vscode", "code"),
       ("visualstudiocode", "code"),
       ("vs代码", "code"),
       ("word", "winword.exe"),
       ("excel", "excel.exe"),
       ("powerpoint", "powerpnt.exe"),
       ("ppt", "powerpnt.exe"),
       ("outlook", "outlook.exe"),
       ("微信", "wechat.exe"),
       ("wechat", "wechat.exe"),
       ("weixin", "wechat.exe"),
       ("企业微信", "wxwork.exe"),
       ("wxwork", "wxwork.exe"),
       ("飞书", "lark.exe"),
       ("lark", "lark.exe"),
       ("钉钉", "dingtalk.exe"),
       ("dingtalk", "dingtalk.exe"),
       ("qq", "qq.exe"),
       ("腾讯qq", "qq.exe"),
       ("wps", "wps.exe"),
       ("wpsoffice", "wps.exe"),
       ("obs", "obs64.exe"),
       ("obsstudio", "obs64.exe"),
       ("xmind", "xmind.exe"),
       ("postman", "postman.exe"),
   ];
   for (k, v) in exact {
       let kk = k.to_lowercase().replace([' ', '（', '）', '(', ')'], "");
       if a == kk {
           return v.to_string();
       }
   }
   // 非命中：原样返回；Windows 调用方会自动补 .exe
   alias.trim().to_string()
}
fn parse_csv_rows(
   csv: &str,
   headers: &[&str],
) -> Vec<BTreeMap<String, String>> {
   let mut out = Vec::new();
   for line in csv.lines().skip_while(|l| l.trim().is_empty()) {
       let cols: Vec<&str> = line.split("\",\"").collect();
       if cols.len() != headers.len() {
           continue;
       }
       let mut row = BTreeMap::new();
       for (i, h) in headers.iter().enumerate() {
           let v = cols[i].trim_matches('"').trim().to_string();
           row.insert((*h).to_string(), v);
       }
       out.push(row);
   }
   out.truncate(50); // 最多返回 50 条，避免 UI/审计爆炸
   out
}
fn parse_ps_rows(
   out: &str,
) -> Vec<BTreeMap<String, String>> {
   let mut rows = Vec::new();
   for line in out.lines().take(50) {
       let trimmed = line.trim();
       if trimmed.is_empty() {
           continue;
       }
       // "  1234 firefox-esr    123456" → pid / comm / rss
       let parts: Vec<&str> = trimmed.split_whitespace().collect();
       if parts.len() < 2 {
           continue;
       }
       let mut r = BTreeMap::new();
       r.insert("pid".into(), parts[0].to_string());
       r.insert("comm".into(), parts[1].to_string());
       r.insert(
           "rss_kb".into(),
           parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
       );
       rows.push(r);
   }
   rows
}
#[cfg(test)]
mod tests {
   use super::*;
   #[test]
   fn app_alias_chinese_to_executable() {
       assert_eq!(normalize_app_exec("飞书"), "lark.exe");
       assert_eq!(normalize_app_exec("企业微信"), "wxwork.exe");
       assert_eq!(normalize_app_exec(" VS Code "), "code");
       assert_eq!(normalize_app_exec("vscode"), "code");
       assert_eq!(normalize_app_exec("PowerPoint"), "powerpnt.exe");
   }
}