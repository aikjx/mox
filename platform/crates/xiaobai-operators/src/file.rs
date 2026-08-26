//! File 算子：文件操作（file_exists / read_text_head / open_file_with_app / copy_to_clipboard / move_to_trash / hard_delete）
//!
//! 跨平台回退链：
//! - copy_to_clipboard：arboard → Windows(clipbrd.exe echo) → macOS(pbcopy) → Linux(wl-copy→xclip)
//! - move_to_trash：首选 `trash` crate（后续接）→ 回退 cmd 命令（Windows 使用 shell recycle bin 需要 shell32，先记录为"永久删除需 L3"）→ 缺失依赖时转 XB-007
//! - hard_delete(L3)：`std::fs::remove_file/remove_dir_all`，需参数 allow_permanent_delete=true 二次确认
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use async_trait::async_trait;
use serde_json::json;
use crate::helpers::{platform_tag, run_command, truncate_head};
use xiaobai_core::errors::XiaobaiError;
use xiaobai_core::identity::OperatorIdentity;
use xiaobai_core::operator::{
   ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use xiaobai_core::rbac::ClearanceLevel;
#[derive(Debug, Default, Clone)]
pub struct FileOperator;
impl FileOperator {
   pub(crate) fn copy_impl(&self, content: &str) -> Result<Vec<&'static str>, XiaobaiError> {
       let mut fb = Vec::new();
       // 回退 1：arboard 跨平台剪贴板（静态 Clipboard 可能在无头失败 → 捕获 Err）
       match arboard::Clipboard::new() {
           Ok(mut clip) => {
               match clip.set_text(content.to_string()) {
                   Ok(_) => {
                       fb.push("arboard_set_text");
                       return Ok(fb);
                   }
                   Err(_) => fb.push("arboard_failed"),
               }
           }
           Err(_) => fb.push("arboard_init_failed"),
       }
       // 回退 2：平台专用命令
       if cfg!(windows) {
           // Windows：cmd /c echo <content> | clip
           let r = run_command(
               "cmd",
               &["/C", &format!("echo {}| clip", escape_cmd(content))],
           );
           fb.push("cmd_echo_pipe_clip");
           if matches!(r, Ok((_, _, 0))) {
               return Ok(fb);
           }
       } else if cfg!(target_os = "macos") {
           let r = run_command("bash", &["-c", &format!("printf '%s' '{}' | pbcopy", escape_sh(content))]);
           fb.push("bash_pipe_pbcopy");
           if matches!(r, Ok((_, _, 0))) {
               return Ok(fb);
           }
       } else {
           // Linux：优先 wl-copy（Wayland）回退 xclip（X11）
           let candidates: [(&[&str], &str); 2] = [
               (&["bash", "-c", &format!("printf '%s' '{}' | wl-copy", escape_sh(content))], "bash_pipe_wl-copy"),
               (&["bash", "-c", &format!("printf '%s' '{}' | xclip -selection clipboard", escape_sh(content))], "bash_pipe_xclip"),
           ];
           for (args, tag) in candidates.iter() {
               let r = run_command(args[0], &args[1..]);
               fb.push(tag);
               if matches!(r, Ok((_, _, 0))) {
                   return Ok(fb);
               }
           }
       }
       Err(XiaobaiError::OperatorUnsupported {
           category: OperatorCategory::File.as_str().to_string(),
           action: "copy_to_clipboard".into(),
           platform: platform_tag(),
           fallbacks_used: fb.iter().map(|s| s.to_string()).collect(),
       })
   }
   fn trash_impl(&self, path: &Path, allow_permanent: bool) -> Result<(Vec<&'static str>, String), XiaobaiError> {
       let mut fb = Vec::new();
       // 回退 1：真实回收站（P2 接入 trash 库/windows-rs SHEmptyRecycleBin）
       fb.push("recycle_bin_stub_pending"); // 标记为"P2 待真实接入"
       // 回退 2：只有 allow_permanent=true（二次确认）才永久删除
       if allow_permanent {
           let r = if path.is_dir() {
               fs::remove_dir_all(path)
           } else {
               fs::remove_file(path)
           };
           fb.push("permanent_delete_fallback");
           match r {
               Ok(_) => {
                   return Ok((
                       fb,
                       format!("⚠️ 回收站未实现，已按 allow_permanent_delete=true 永久删除：{}", path.display()),
                   ));
               }
               Err(e) => {
                   return Err(XiaobaiError::ExecutionError {
                       category: OperatorCategory::File.as_str().into(),
                       action: "move_to_trash".into(),
                       detail: format!("永久删除回退也失败：{e}"),
                   });
               }
           }
       }
       Err(XiaobaiError::OperatorUnsupported {
           category: OperatorCategory::File.as_str().to_string(),
           action: "move_to_trash".into(),
           platform: platform_tag(),
           fallbacks_used: fb.iter().map(|s| s.to_string()).collect(),
       })
   }
}
#[async_trait]
impl SystemOperator for FileOperator {
   fn id(&self) -> &'static str {
       "file_operator_v1"
   }
   fn category(&self) -> OperatorCategory {
       OperatorCategory::File
   }
   fn list_actions(&self) -> Vec<ActionSignature> {
       use ClearanceLevel::*;
       use std::collections::BTreeMap;
       let mut p_path = BTreeMap::new();
       p_path.insert("path", "string，文件/目录绝对路径（PII 敏感路径强制升 L3）");
       let mut p_trash = BTreeMap::new();
       p_trash.insert("path", "同 path");
       p_trash.insert("allow_permanent_delete", "bool 默认 false；回收站不可用时 true 才允许永久删除");
       let mut p_copy = BTreeMap::new();
       p_copy.insert("content", "string 要复制的纯文本内容，最长 1MB；或 path=某个文件则复制文件内容");
       let mut p_read = BTreeMap::new();
       p_read.insert("path", "同 path；可选 max_lines=3 / max_chars_per_line=200");
       vec![
           ActionSignature {
               name: "file_exists",
               category: OperatorCategory::File,
               clearance: L0,
               own_qualified: false,
               description: "只读：判断 path 指向的文件或目录是否存在",
               params: Some(p_path.clone()),
           },
           ActionSignature {
               name: "read_text_head",
               category: OperatorCategory::File,
               clearance: L0,
               own_qualified: false,
               description: "只读：读取文本文件前 N 行（默认 3 行，每行 200 字截断）",
               params: Some(p_read),
           },
           ActionSignature {
               name: "open_file_with_app",
               category: OperatorCategory::File,
               clearance: L1,
               own_qualified: false,
               description: "用系统默认关联程序打开 path 指定的文件",
               params: Some(p_path.clone()),
           },
           ActionSignature {
               name: "copy_to_clipboard",
               category: OperatorCategory::File,
               clearance: L2,
               own_qualified: false,
               description: "复制 content 纯文本或 path 文件内容到系统剪贴板（Expert/Coordinator）",
               params: Some(p_copy),
           },
           ActionSignature {
               name: "move_to_trash",
               category: OperatorCategory::File,
               clearance: L3,
               own_qualified: true,
               description: "把 path 丢进系统回收站（Owner 可宽容 L2→L3；PII 资源强制 L3）",
               params: Some(p_trash.clone()),
           },
           ActionSignature {
               name: "hard_delete",
               category: OperatorCategory::File,
               clearance: L3,
               own_qualified: true,
               description: "永久删除文件/目录（不可恢复），需 allow_permanent_delete=true 二次确认",
               params: Some(p_trash.clone()),
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
           "file_exists" => {
               let p = require_path(&param, action)?;
               let ok = Path::new(&p).exists();
               Ok(OperatorOutput::quick(format!("文件存在判定：{} = {ok}", p))
                   .with_payload(json!({"path": p, "exists": ok, "is_dir": Path::new(&p).is_dir()}))
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "read_text_head" => {
               let p = require_path(&param, action)?;
               let max_lines = param.get_i64("max_lines").unwrap_or(3).max(1).min(50) as usize;
               let max_chars = param.get_i64("max_chars_per_line").unwrap_or(200).max(20).min(4000) as usize;
               let raw = fs::read_to_string(PathBuf::from(&p)).map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::File.as_str().into(),
                   action: "read_text_head".into(),
                   detail: format!("读取文件 {p} 失败：{e}"),
               })?;
               let clipped = truncate_head(&raw, max_lines, max_chars);
               Ok(OperatorOutput::quick(format!("read_text_head: {} 已读（截断显示）", p))
                   .with_payload(json!({"path": p, "lines": clipped, "max_lines": max_lines}))
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "open_file_with_app" => {
               let p = require_path(&param, action)?;
               let app = crate::app::AppOperator::default();
               let r = app.open_app_impl(&p).map_err(|_e| XiaobaiError::OperatorUnsupported {
                   category: OperatorCategory::File.as_str().to_string(),
                   action: "open_file_with_app".into(),
                   platform: platform_tag(),
                   fallbacks_used: vec!["delegate_to_app_operator".to_string()],
               })?;
               Ok(OperatorOutput::quick(r.1).with_fallbacks(r.0.iter().map(|s| s.to_string()).collect::<Vec<_>>()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "copy_to_clipboard" => {
               let content = match (param.get_str("content"), param.get_str("path")) {
                   (Some(c), _) => c.to_string(),
                   (None, Some(p)) => fs::read_to_string(p).map_err(|e| XiaobaiError::ExecutionError {
                       category: OperatorCategory::File.as_str().into(),
                       action: "copy_to_clipboard".into(),
                       detail: format!("读取 path={p} 失败：{e}"),
                   })?,
                   (None, None) => {
                       return Err(XiaobaiError::InvalidArgument {
                           action: "copy_to_clipboard".into(),
                           param: "content|path".into(),
                           value: "<missing>".into(),
                           hint: "需 content 字符串 或 path 文件路径两者之一".into(),
                       });
                   }
               };
               if content.len() > 1024 * 1024 {
                   return Err(XiaobaiError::InvalidArgument {
                       action: "copy_to_clipboard".into(),
                       param: "content".into(),
                       value: format!("len={} 字节", content.len()),
                       hint: "单次复制最大 1MB；大文件请打开文件手动复制".into(),
                   });
               }
               let fb = self.copy_impl(&content)?;
               Ok(OperatorOutput::quick(format!("已复制 {} 字符到剪贴板", content.chars().count()))
                   .with_fallbacks(fb.iter().map(|s| s.to_string()).collect())
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "move_to_trash" => {
               let p = require_path(&param, action)?;
               let allow = param.get_bool("allow_permanent_delete").unwrap_or(false);
               let (fb, msg) = self.trash_impl(Path::new(&p), allow)?;
               Ok(OperatorOutput::quick(msg).with_fallbacks(fb.iter().map(|s| s.to_string()).collect()).with_elapsed(t0.elapsed().as_millis() as u64))
           }
           "hard_delete" => {
               let p = require_path(&param, action)?;
               let confirm = param.get_bool("allow_permanent_delete").unwrap_or(false);
               if !confirm {
                   return Err(XiaobaiError::PermissionDenied {
                       action: "hard_delete".into(),
                       required: xiaobai_core::errors::ClearanceLevelRepr(3),
                       identity: "<二次确认>".into(),
                       reason: "hard_delete 是不可恢复操作，必须 allow_permanent_delete=true 二次确认",
                   });
               }
               let r = if Path::new(&p).is_dir() {
                   fs::remove_dir_all(Path::new(&p))
               } else {
                   fs::remove_file(Path::new(&p))
               };
               r.map_err(|e| XiaobaiError::ExecutionError {
                   category: OperatorCategory::File.as_str().into(),
                   action: "hard_delete".into(),
                   detail: format!("删除 {p} 失败：{e}"),
               })?;
               Ok(OperatorOutput::quick(format!("已永久删除：{}（不可恢复）", p))
                   .with_elapsed(t0.elapsed().as_millis() as u64))
           }
           other => Err(XiaobaiError::IntentUnknown(other.into())),
       }
   }
}
fn require_path(param: &ActionParam, action: &str) -> Result<String, XiaobaiError> {
   param
       .get_str("path")
       .map(|s| s.to_string())
       .ok_or_else(|| XiaobaiError::InvalidArgument {
           action: action.into(),
           param: "path".into(),
           value: "<missing>".into(),
           hint: "需要 path 字符串参数（文件绝对/相对路径）".into(),
       })
}
fn escape_cmd(s: &str) -> String {
   // 最基础：把所有 " 加倍，&/| 等保留（copy_to_clipboard 内容里常见引号）
   s.replace('"', "\"\"")
}
fn escape_sh(s: &str) -> String {
   s.replace('\'', "'\\''")
}
#[cfg(test)]
mod tests {
   use super::*;
   #[test]
   fn escape_shell_single_quote_chained() {
       assert_eq!(escape_sh("it's ok"), "it'\\''s ok");
   }
}