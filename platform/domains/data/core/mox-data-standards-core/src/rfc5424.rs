// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RFC 5424 Syslog 格式化审计日志。
//!
//! 参考: https://www.rfc-editor.org/rfc/rfc5424

use std::collections::BTreeMap;

/// Syslog Facility + Severity 编码：pri = facility*8 + severity。
/// 例如：Audit(13) + Info(6) = 13*8+6 = 110。
pub fn make_pri(facility: u8, severity: u8) -> u16 {
    (facility as u16) * 8 + (severity as u16)
}

/// RFC 5424 Syslog 事件结构。
#[derive(Debug, Clone)]
pub struct SyslogEvent {
    pub pri: u16,
    pub ts: String,                                        // RFC 3339 timestamp or "-"
    pub host: String,                                      // hostname or "-"
    pub app: String,                                       // APP-NAME or "-"
    pub procid: String,                                    // PROCID or "-"
    pub msgid: String,                                     // MSGID or "-"
    pub sdata: BTreeMap<String, BTreeMap<String, String>>, // SD-ID → PARAM-NAME → PARAM-VALUE
    pub msg: String, // free-form MSG (optional BOM 前缀由调用方控制)
}

fn escape_sd_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            ']' => out.push_str("\\]"),
            _ => out.push(c),
        }
    }
    out
}

impl SyslogEvent {
    /// 序列化为 RFC 5424 单行格式（无尾换行）。
    pub fn to_rfc5424(&self) -> String {
        // Header: <pri>VERSION SP TS SP HOST SP APP SP PROCID SP MSGID SP
        let header = format!(
            "<{}>1 {} {} {} {} {} ",
            self.pri,
            if self.ts.is_empty() { "-" } else { &self.ts },
            if self.host.is_empty() {
                "-"
            } else {
                &self.host
            },
            if self.app.is_empty() { "-" } else { &self.app },
            if self.procid.is_empty() {
                "-"
            } else {
                &self.procid
            },
            if self.msgid.is_empty() {
                "-"
            } else {
                &self.msgid
            },
        );
        // Structured Data
        let sd = if self.sdata.is_empty() {
            "-".to_string()
        } else {
            let mut s = String::new();
            for (sd_id, params) in &self.sdata {
                s.push('[');
                s.push_str(sd_id);
                for (k, v) in params {
                    s.push(' ');
                    s.push_str(k);
                    s.push_str("=\"");
                    s.push_str(&escape_sd_value(v));
                    s.push('"');
                }
                s.push(']');
            }
            s
        };
        if self.msg.is_empty() {
            format!("{}{}", header, sd)
        } else {
            format!("{}{} {}", header, sd, self.msg)
        }
    }
}
