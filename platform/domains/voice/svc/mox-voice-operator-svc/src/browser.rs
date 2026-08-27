// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Browser 算子：浏览器控制（open_url / search_query / list_tabs / close_tab / bookmark_add）
//!
//! 跨平台回退链：
//! - open_url：默认浏览器（cmd start / open / xdg-open）→ Chrome → Edge → Firefox
//! - search_query：构造 URL（百度/必应/谷歌）后复用 open_url
//! - list_tabs：优先返回与浏览器相关的进程窗口（ps+窗口标题近似），后续可扩展接入 Chrome DevTools Protocol
//! - close_tab：enigo Ctrl+W 顶层窗口（需用户在前台），失败 XB-007
//! - bookmark_add：构造浏览器 bookmarks 页面并 open_url（实际持久化留给浏览器后续 CDP/扩展插件）

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::json;

use crate::helpers::{platform_tag, run_command, run_command_xb};
use crate::app::normalize_app_exec;
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;

#[derive(Debug, Default, Clone)]
pub struct BrowserOperator;

impl BrowserOperator {
    // ============ open_url ============
    pub(crate) fn open_url_impl(&self, url: &str, browser_hint: Option<&str>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        if url.is_empty() {
            return Err(XiaobaiError::InvalidArgument {
                action: "open_url".into(),
                param: "url".into(),
                value: "<empty>".into(),
                hint: "url 不能为空".into(),
            });
        }
        // 若无 http 前缀，自动补 http
        let normalized = if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("about:") {
            url.to_string()
        } else {
            format!("https://{url}")
        };
        let mut fbs = Vec::new();
        // 优先按 hint 打开指定浏览器
        if let Some(b) = browser_hint {
            let exec = normalize_app_exec(b);
            if cfg!(windows) {
                fbs.push("hint_cmd_start_browser");
                let r = run_command("cmd", &["/c", "start", "", &exec, &normalized]);
                if let Ok((_, _, 0)) = r {
                    return Ok((fbs, format!("已通过 {exec} 打开：{normalized}")));
                }
            } else if cfg!(target_os = "macos") {
                fbs.push("hint_open_-a_browser");
                let r = run_command("open", &["-a", &exec, &normalized]);
                if let Ok((_, _, 0)) = r {
                    return Ok((fbs, format!("已通过 {exec} 打开：{normalized}")));
                }
            } else {
                fbs.push("hint_browser_direct");
                let r = run_command(&exec, &[&normalized]);
                if let Ok((_, _, 0)) = r {
                    return Ok((fbs, format!("已通过 {exec} 打开：{normalized}")));
                }
            }
        }
        // 回退：系统默认浏览器
        if cfg!(windows) {
            fbs.push("cmd_start_default");
            let r = run_command("cmd", &["/c", "start", "", &normalized]);
            if let Ok((_, _, 0)) = r {
                return Ok((fbs, format!("默认浏览器已打开：{normalized}")));
            }
        } else if cfg!(target_os = "macos") {
            fbs.push("open_url_default");
            let r = run_command("open", &[&normalized]);
            if let Ok((_, _, 0)) = r {
                return Ok((fbs, format!("默认浏览器已打开：{normalized}")));
            }
        } else {
            fbs.push("xdg_open_default");
            let r = run_command("xdg-open", &[&normalized]);
            if let Ok((_, _, 0)) = r {
                return Ok((fbs, format!("默认浏览器已打开：{normalized}")));
            }
        }
        // 最终兜底：依次尝试 Chrome/Edge/Firefox 直启
        let final_list: &[(&str, &[&str])] = if cfg!(windows) {
            &[("chrome.exe", &["chrome"] as &[&str]), ("msedge.exe", &["msedge"]), ("firefox.exe", &["firefox"])]
        } else if cfg!(target_os = "macos") {
            &[("Google Chrome", &["-a", "Google Chrome"]), ("Microsoft Edge", &["-a", "Microsoft Edge"]), ("Firefox", &["-a", "Firefox"])]
        } else {
            &[("google-chrome", &["google-chrome"]), ("microsoft-edge", &["microsoft-edge"]), ("firefox", &["firefox"])]
        };
        for (name, args_head) in final_list {
            let mut args: Vec<&str> = Vec::new();
            for a in *args_head { args.push(a); }
            args.push(&normalized);
            let _cmd = if cfg!(target_os = "macos") { "open" } else { args_head[0] };
            let fb_name = format!("fallback_{name}");
            let r = if cfg!(target_os = "macos") {
                run_command("open", &args)
            } else {
                run_command(args_head[0], &[&normalized])
            };
            // Push fallback AFTER borrow of static literal
            fbs.push(Box::leak(fb_name.into_boxed_str()));
            if let Ok((_, _, 0)) = r {
                return Ok((fbs, format!("{name} 已打开：{normalized}")));
            }
        }
        Err(XiaobaiError::OperatorUnsupported {
            category: OperatorCategory::Browser.as_str().to_string(),
            action: "open_url".into(),
            platform: platform_tag(),
            fallbacks_used: fbs.iter().map(|s| s.to_string()).collect(),
        })
    }

    // ============ search_query ============
    pub(crate) fn build_search_url(query: &str, engine: &str) -> String {
        let q = url_encode(query);
        match engine {
            "bing" | "必应" => format!("https://www.bing.com/search?q={q}"),
            "google" | "谷歌" => format!("https://www.google.com/search?q={q}"),
            _ => format!("https://www.baidu.com/s?wd={q}"), // 默认百度，中文最优
        }
    }

    // ============ list_tabs ============
    pub(crate) fn list_tabs_impl(&self, browser_filter: Option<&str>) -> XiaobaiResult<(Vec<&'static str>, Vec<BTreeMap<String, String>>)> {
        let mut fbs = Vec::new();
        // 跨平台统一最佳努力：列出进程中浏览器相关的进程名，并尽量从命令行推断 "用户打开了多少个 浏览器实例"
        // Windows：tasklist /v /fo csv 有窗口标题
        // Linux：ps -eo pid,comm,args
        // macOS：ps -eo pid,comm,args
        let (stdout, _stderr, _code) = if cfg!(windows) {
            fbs.push("tasklist_v_csv");
            run_command_xb("tasklist", &["/V", "/FO", "CSV", "/NH"], OperatorCategory::Browser, "list_tabs")?
        } else {
            fbs.push("ps_eo_pid_comm_args");
            run_command_xb("ps", &["-eo", "pid=,comm=,args="], OperatorCategory::Browser, "list_tabs")?
        };
        let filter_names: Vec<&str> = match browser_filter {
            Some(b) => {
                let exec = normalize_app_exec(b).to_lowercase().replace(".exe", "");
                vec![Box::leak(exec.into_boxed_str())]
            }
            None => vec!["chrome", "msedge", "edge", "firefox", "lark", "wechat", "qqbrowser", "360se", "brave", "opera", "vivaldi", "safari"],
        };
        let mut rows = Vec::new();
        for line in stdout.lines().take(200) {
            let l = line.trim();
            if l.is_empty() { continue; }
            let lower = l.to_lowercase();
            if !filter_names.iter().any(|n| lower.contains(n)) { continue; }
            let mut r = BTreeMap::new();
            if cfg!(windows) {
                // CSV："Image Name","PID",...,"Window Title"
                let cols: Vec<&str> = l.split("\",\"").collect();
                r.insert("process".into(), cols.get(0).map(|s| s.trim_matches('"').to_string()).unwrap_or_default());
                r.insert("pid".into(), cols.get(1).map(|s| s.trim_matches('"').to_string()).unwrap_or_default());
                r.insert("window_title".into(), cols.last().map(|s| s.trim_matches('"').to_string()).unwrap_or_default());
            } else {
                let parts: Vec<&str> = l.splitn(3, char::is_whitespace).collect();
                r.insert("pid".into(), parts.get(0).copied().unwrap_or("").to_string());
                r.insert("comm".into(), parts.get(1).copied().unwrap_or("").to_string());
                r.insert("args".into(), parts.get(2).copied().unwrap_or("").to_string());
            }
            rows.push(r);
        }
        Ok((fbs, rows))
    }

    // ============ close_tab ============
    pub(crate) fn close_tab_impl(&self) -> XiaobaiResult<(Vec<&'static str>, String)> {
        // 使用 enigo 模拟 Ctrl+W（通用关闭标签快捷键）；风险：需要浏览器窗口焦点在前台
        let mut fbs = Vec::new();
        use enigo::{Keyboard, Key, Settings};
        fbs.push("enigo_ctrl_w");
        let mut en = enigo::Enigo::new(&Settings::default()).map_err(|_e| XiaobaiError::OperatorUnsupported {
            category: OperatorCategory::Browser.as_str().to_string(),
            action: "close_tab".into(),
            platform: platform_tag(),
            fallbacks_used: vec!["enigo_not_available".into()],
        })?;
        en.key(Key::Control, enigo::Direction::Press).map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Browser.as_str().into(),
            action: "close_tab".into(),
            detail: format!("enigo ctrl press failed: {e}"),
        })?;
        let r_text = en.key(Key::Unicode('w'), enigo::Direction::Click)
            .and_then(|_| en.key(Key::Control, enigo::Direction::Release));
        match r_text {
            Ok(()) => Ok((fbs, "已向前台窗口发送 Ctrl+W（若为浏览器即关闭当前标签）".into())),
            Err(e) => Err(XiaobaiError::ExecutionError {
                category: OperatorCategory::Browser.as_str().into(),
                action: "close_tab".into(),
                detail: format!("enigo close_tab failed: {e}"),
            }),
        }
    }

    // ============ bookmark_add ============
    pub(crate) fn bookmark_add_impl(&self, url: &str, title: Option<&str>) -> XiaobaiResult<(Vec<&'static str>, String)> {
        // 目前最佳努力：打开浏览器书签管理页 + 提醒用户手动保存（CDP 版可后续接入）
        let mut fbs = Vec::new();
        let bookmarks_page = if cfg!(windows) || cfg!(target_os = "linux") {
            "chrome://bookmarks"
        } else {
            "https://support.apple.com/zh-cn/guide/safari/ibrw1053/mac"
        };
        fbs.push("open_bookmarks_page");
        let (fbs_sub, msg) = self.open_url_impl(bookmarks_page, None)?;
        fbs.append(&mut fbs_sub.iter().map(|s| Box::leak((*s).to_owned().into_boxed_str()) as &str).collect());
        let combined = format!(
            "{msg}；请手动添加书签 — url={url}{title}",
            title = title.map(|t| format!(" title={t}")).unwrap_or_default()
        );
        Ok((fbs, combined))
    }
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[async_trait]
impl SystemOperator for BrowserOperator {
    fn id(&self) -> &'static str {
        "browser_operator_v1"
    }
    fn category(&self) -> OperatorCategory {
        OperatorCategory::Browser
    }
    fn list_actions(&self) -> Vec<ActionSignature> {
        use ClearanceLevel::*;
        let mut p_open = BTreeMap::new();
        p_open.insert("url", "string，https? 地址；缺省前缀自动补 https://；可选 browser=chrome/edge/firefox/微信");
        let mut p_search = BTreeMap::new();
        p_search.insert("query", "string，中文/英文搜索词");
        p_search.insert("engine", "string，可选：baidu(默认) / bing / google");
        let mut p_tabs = BTreeMap::new();
        p_tabs.insert("browser", "string，可选，仅返回指定浏览器进程/窗口（chrome/edge/firefox/...）");
        let mut p_bookmark = BTreeMap::new();
        p_bookmark.insert("url", "string，要收藏的地址");
        p_bookmark.insert("title", "string，可选，书签标题");
        vec![
            ActionSignature {
                name: "open_url",
                category: OperatorCategory::Browser,
                clearance: L1,
                own_qualified: false,
                description: "打开一个 URL：系统默认浏览器 → 指定 browser hint → Chrome/Edge/Firefox 三级回退",
                params: Some(p_open),
            },
            ActionSignature {
                name: "search_query",
                category: OperatorCategory::Browser,
                clearance: L1,
                own_qualified: false,
                description: "用搜索引擎构造 URL 后走 open_url（默认百度，中文体验更优）",
                params: Some(p_search),
            },
            ActionSignature {
                name: "list_tabs",
                category: OperatorCategory::Browser,
                clearance: L1,
                own_qualified: true,
                description: "返回浏览器相关进程列表（窗口标题/args 近似 tab 数），Own 场景 L0 可查看",
                params: Some(p_tabs),
            },
            ActionSignature {
                name: "close_tab",
                category: OperatorCategory::Browser,
                clearance: L2,
                own_qualified: false,
                description: "向当前前台窗口发送 Ctrl+W 关闭标签（enigo；浏览器需在前台），失败 XB-007",
                params: None,
            },
            ActionSignature {
                name: "bookmark_add",
                category: OperatorCategory::Browser,
                clearance: L2,
                own_qualified: false,
                description: "打开浏览器书签管理页并提示手动收藏（CDP 版本可后续自动写入）",
                params: Some(p_bookmark),
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
            "open_url" => {
                let url = param.get_str("url").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "open_url".into(),
                    param: "url".into(),
                    value: "<missing>".into(),
                    hint: "需要 url 字符串".into(),
                })?;
                let hint = param.get_str("browser");
                let (fbs, msg) = self.open_url_impl(url, hint)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "search_query" => {
                let query = param.get_str("query").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "search_query".into(),
                    param: "query".into(),
                    value: "<missing>".into(),
                    hint: "需要 query 搜索词字符串".into(),
                })?;
                let engine = param.get_str("engine").unwrap_or("baidu");
                let url = Self::build_search_url(query, engine);
                let (fbs, msg) = self.open_url_impl(&url, None)?;
                Ok(OperatorOutput::quick(msg)
                    .with_payload(json!({"engine": engine, "query": query, "final_url": url}))
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "list_tabs" => {
                let bf = param.get_str("browser");
                let (fbs, rows) = self.list_tabs_impl(bf)?;
                Ok(OperatorOutput::quick(format!("浏览器相关进程 {} 条", rows.len()))
                    .with_payload(json!(rows))
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "close_tab" => {
                let (fbs, msg) = self.close_tab_impl()?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "bookmark_add" => {
                let url = param.get_str("url").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "bookmark_add".into(),
                    param: "url".into(),
                    value: "<missing>".into(),
                    hint: "需要 url 字符串".into(),
                })?;
                let title = param.get_str("title");
                let (fbs, msg) = self.bookmark_add_impl(url, title)?;
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
    fn build_search_url_baidu_default_encodes_chinese() {
        let u = BrowserOperator::build_search_url("小白语音 助手", "baidu");
        assert!(u.starts_with("https://www.baidu.com/s?wd="));
        assert!(u.contains("%E5%B0%8F%E7%99%BD"), "URL 编码失败：{u}");
        assert!(u.contains("%E8%AF%AD%E9%9F%B3"));
    }

    #[test]
    fn build_search_url_bing_google_ok() {
        let b = BrowserOperator::build_search_url("rust", "bing");
        assert!(b.starts_with("https://www.bing.com/search?q=rust"));
        let g = BrowserOperator::build_search_url("rust", "google");
        assert!(g.starts_with("https://www.google.com/search?q=rust"));
    }

    #[test]
    fn url_encode_alphanum_noop() {
        assert_eq!(url_encode("abcXYZ-0._~9"), "abcXYZ-0._~9");
    }

    #[test]
    fn list_actions_5_covered() {
        let op = BrowserOperator::default();
        let acts = op.list_actions();
        assert_eq!(acts.len(), 5);
        let names: Vec<_> = acts.iter().map(|a| a.name).collect();
        for n in ["open_url", "search_query", "list_tabs", "close_tab", "bookmark_add"] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert_eq!(
            acts.iter().find(|a| a.name == "close_tab").unwrap().clearance,
            ClearanceLevel::L2,
            "Ctrl+W 属键盘操作类 L2"
        );
    }
}
