//! Network 算子：网络与代理（ping / dns_lookup / traffic_usage / netstat / disable_iface / enable_iface）
//!
//! 跨平台回退链：
//! - ping：Windows(ping -n) / macOS·Linux(ping -c)
//! - dns_lookup：Windows(nslookup) / macOS·Linux(dig +short → host)
//! - traffic_usage：Windows(netstat -e) / macOS(netstat -ib) / Linux(cat /proc/net/dev)
//! - netstat -anp：Windows(netstat -ano) / macOS(netstat -anp tcp) / Linux(ss -tlnp → netstat -tlnp)
//! - disable_iface / enable_iface：Windows(netsh int set int) / Linux(ip link) / macOS(ifconfig up/down) — L3 MoxAdmin

use std::collections::BTreeMap;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::helpers::{run_command, run_command_xb};
use mox_voice_core_svc::errors::{XiaobaiError, XiaobaiResult};
use mox_voice_core_svc::identity::OperatorIdentity;
use mox_voice_core_svc::operator::{
    ActionParam, ActionSignature, OperatorCategory, OperatorOutput, SystemOperator,
};
use mox_voice_core_svc::rbac::ClearanceLevel;

#[derive(Debug, Default, Clone)]
pub struct NetworkOperator;

impl NetworkOperator {
    // ============ ping ============
    pub(crate) fn ping_impl(&self, host: &str, count: u32) -> XiaobaiResult<(Vec<&'static str>, Value)> {
        let count_s = count.to_string();
        let mut fbs = Vec::new();
        let (cmd, args): (&str, Vec<&str>) = if cfg!(windows) {
            fbs.push("ping_-n");
            ("ping", vec!["-n", &count_s, host])
        } else {
            fbs.push("ping_-c");
            ("ping", vec!["-c", &count_s, host])
        };
        let (stdout, stderr, code) = run_command_xb(cmd, &args, OperatorCategory::Network, "ping")?;
        // ping 失败（主机不可达）exit_code 非 0，但仍有 RTT 统计，不视为错误，仅在 output 标记
        Ok((fbs, json!({"host": host, "count": count, "exit_code": code, "stdout": stdout, "stderr": stderr, "ok": code == 0})))
    }

    // ============ dns_lookup ============
    pub(crate) fn dns_lookup_impl(&self, domain: &str) -> XiaobaiResult<(Vec<&'static str>, Value)> {
        let mut fbs = Vec::new();
        let (stdout, stderr, code) = if cfg!(windows) {
            fbs.push("nslookup");
            run_command("nslookup", &[domain])
        } else {
            // 优先 dig +short，再 host
            let r = run_command("dig", &["+short", "+time=2", domain]);
            if let Ok((so, _, 0)) = &r {
                if !so.trim().is_empty() {
                    fbs.push("dig_+short");
                    r
                } else {
                    fbs.push("host_-t_a");
                    run_command("host", &["-t", "a", domain])
                }
            } else {
                fbs.push("host_-t_a");
                run_command("host", &["-t", "a", domain])
            }
        }
        .map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Network.as_str().into(),
            action: "dns_lookup".into(),
            detail: format!("dns query failed: {e}"),
        })?;
        Ok((fbs, json!({"domain": domain, "exit_code": code, "stdout": stdout, "stderr": stderr})))
    }

    // ============ traffic_usage ============
    pub(crate) fn traffic_usage_impl(&self) -> XiaobaiResult<(Vec<&'static str>, Value)> {
        let mut fbs = Vec::new();
        let (stdout, stderr, code) = if cfg!(windows) {
            fbs.push("netstat_-e");
            run_command("netstat", &["-e"])
        } else if cfg!(target_os = "linux") {
            fbs.push("proc_net_dev");
            match std::fs::read_to_string("/proc/net/dev") {
                Ok(s) => Ok((s, String::new(), 0)),
                Err(_) => {
                    fbs.push("netstat_-ib");
                    run_command("netstat", &["-ib"])
                }
            }
        } else {
            fbs.push("netstat_-ib");
            run_command("netstat", &["-ib"])
        }
        .map_err(|e| XiaobaiError::ExecutionError {
            category: OperatorCategory::Network.as_str().into(),
            action: "traffic_usage".into(),
            detail: format!("traffic read failed: {e}"),
        })?;
        Ok((fbs, json!({"exit_code": code, "stdout": stdout, "stderr": stderr})))
    }

    // ============ netstat ============
    pub(crate) fn netstat_impl(&self) -> XiaobaiResult<(Vec<&'static str>, Vec<BTreeMap<String, String>>)> {
        let mut fbs = Vec::new();
        let (stdout, _stderr, _code) = if cfg!(windows) {
            fbs.push("netstat_-ano");
            run_command_xb("netstat", &["-ano"], OperatorCategory::Network, "netstat")?
        } else if cfg!(target_os = "linux") {
            fbs.push("ss_-tlnp");
            let r = run_command("ss", &["-tlnpH"]);
            let (so, se, co) = match r {
                Ok((so, _, 0)) if !so.trim().is_empty() => (so, String::new(), 0),
                _ => {
                    fbs.push("netstat_-tlnp");
                    run_command_xb("netstat", &["-tlnp"], OperatorCategory::Network, "netstat")?
                }
            };
            (so, se, co)
        } else {
            fbs.push("netstat_-anp_tcp");
            run_command_xb("netstat", &["-anp", "tcp"], OperatorCategory::Network, "netstat")?
        };
        let rows = parse_netstat_rows(&stdout);
        Ok((fbs, rows))
    }

    // ============ disable_iface / enable_iface ============
    fn toggle_iface_impl(&self, iface: &str, enable: bool) -> XiaobaiResult<(Vec<&'static str>, String)> {
        let mut fbs = Vec::new();
        let action_word = if enable { "启用" } else { "禁用" };
        let (cmd, args): (&str, Vec<&str>) = if cfg!(windows) {
            let tag = if enable { "enable" } else { "disable" };
            fbs.push("netsh_int_set_int");
            ("netsh", vec!["interface", "set", "interface", iface, tag])
        } else if cfg!(target_os = "linux") {
            let tag = if enable { "up" } else { "down" };
            fbs.push("ip_link_set");
            ("ip", vec!["link", "set", iface, tag])
        } else {
            let tag = if enable { "up" } else { "down" };
            fbs.push("ifconfig_up_down");
            ("ifconfig", vec![iface, tag])
        };
        let action = if enable { "enable_iface" } else { "disable_iface" };
        let (stdout, stderr, code) = run_command_xb(cmd, &args, OperatorCategory::Network, action)?;
        if code != 0 {
            return Err(XiaobaiError::ExecutionError {
                category: OperatorCategory::Network.as_str().into(),
                action: if enable { "enable_iface" } else { "disable_iface" }.into(),
                detail: format!("exit={code} stdout={stdout} stderr={stderr}"),
            });
        }
        Ok((fbs, format!("已{action_word}网卡：{iface}")))
    }
}

#[async_trait]
impl SystemOperator for NetworkOperator {
    fn id(&self) -> &'static str {
        "network_operator_v1"
    }
    fn category(&self) -> OperatorCategory {
        OperatorCategory::Network
    }
    fn list_actions(&self) -> Vec<ActionSignature> {
        use ClearanceLevel::*;
        let mut p_ping = BTreeMap::new();
        p_ping.insert("host", "string，目标主机/IP；可选 count=number（默认 4）");
        let mut p_dns = BTreeMap::new();
        p_dns.insert("domain", "string，域名，如 baidu.com / ai.infotopograph.com");
        let mut p_iface = BTreeMap::new();
        p_iface.insert("iface", "string，网卡名，如 以太网/Wi-Fi/eth0/en0；L3 权限");
        vec![
            ActionSignature {
                name: "ping",
                category: OperatorCategory::Network,
                clearance: L0,
                own_qualified: false,
                description: "向目标主机发送 ICMP Echo（默认 4 包），主机不可达不视为算子错误（由 ok 字段标记）",
                params: Some(p_ping),
            },
            ActionSignature {
                name: "dns_lookup",
                category: OperatorCategory::Network,
                clearance: L0,
                own_qualified: false,
                description: "查询 A 记录（dig +short → host → nslookup 三级回退）",
                params: Some(p_dns),
            },
            ActionSignature {
                name: "traffic_usage",
                category: OperatorCategory::Network,
                clearance: L0,
                own_qualified: false,
                description: "读取各网卡 RX/TX 字节计数（/proc/net/dev 优先 → netstat -ib → netstat -e）",
                params: None,
            },
            ActionSignature {
                name: "netstat",
                category: OperatorCategory::Network,
                clearance: L0,
                own_qualified: false,
                description: "列 TCP 监听/已建连套接字：proto/local_addr/foreign_addr/state/pid/program",
                params: None,
            },
            ActionSignature {
                name: "disable_iface",
                category: OperatorCategory::Network,
                clearance: L3,
                own_qualified: false,
                description: "禁用指定网卡（netsh/ip link/ifconfig），L3 破坏性网络动作",
                params: Some(p_iface.clone()),
            },
            ActionSignature {
                name: "enable_iface",
                category: OperatorCategory::Network,
                clearance: L3,
                own_qualified: false,
                description: "启用被禁用的网卡（同上链路）",
                params: Some(p_iface),
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
            "ping" => {
                let host = param.get_str("host").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "ping".into(),
                    param: "host".into(),
                    value: "<missing>".into(),
                    hint: "需要 host 字符串参数".into(),
                })?;
                let count = param.get_i64("count").unwrap_or(4).clamp(1, 100) as u32;
                let (fbs, payload) = self.ping_impl(host, count)?;
                let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                Ok(OperatorOutput::quick(format!(
                    "ping {host} x{count}：{}",
                    if ok { "可达" } else { "不可达 / 丢包" }
                ))
                .with_payload(payload)
                .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "dns_lookup" => {
                let domain = param.get_str("domain").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: "dns_lookup".into(),
                    param: "domain".into(),
                    value: "<missing>".into(),
                    hint: "需要 domain 字符串参数".into(),
                })?;
                let (fbs, payload) = self.dns_lookup_impl(domain)?;
                Ok(OperatorOutput::quick(format!("DNS 查询完成：{domain}"))
                    .with_payload(payload)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "traffic_usage" => {
                let (fbs, payload) = self.traffic_usage_impl()?;
                Ok(OperatorOutput::quick("已读取网卡流量统计")
                    .with_payload(payload)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "netstat" => {
                let (fbs, rows) = self.netstat_impl()?;
                Ok(OperatorOutput::quick(format!("当前套接字 {} 条", rows.len()))
                    .with_payload(json!(rows))
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            "disable_iface" | "enable_iface" => {
                let iface = param.get_str("iface").ok_or_else(|| XiaobaiError::InvalidArgument {
                    action: action.into(),
                    param: "iface".into(),
                    value: "<missing>".into(),
                    hint: "需要 iface 网卡名称".into(),
                })?;
                let enable = action == "enable_iface";
                let (fbs, msg) = self.toggle_iface_impl(iface, enable)?;
                Ok(OperatorOutput::quick(msg)
                    .with_fallbacks(fbs.iter().map(|s| s.to_string()).collect())
                    .with_elapsed(t0.elapsed().as_millis() as u64))
            }
            other => Err(XiaobaiError::IntentUnknown(other.into())),
        }
    }
}

fn parse_netstat_rows(stdout: &str) -> Vec<BTreeMap<String, String>> {
    // 统一解析：跳过表头，每行 split_whitespace
    // 典型格式：
    // Windows:  Proto  Local Address          Foreign Address        State           PID
    // Linux ss: LISTEN 0 128 0.0.0.0:22 0.0.0.0:* users:(("sshd",pid=123,fd=3))
    let mut rows = Vec::new();
    for line in stdout.lines().skip_while(|l| {
        // 跳过表头行（含 "Proto" 或 "State"，或首字非字母/数字）
        let t = l.trim();
        t.is_empty()
            || t.contains("Proto")
            || t.contains("Local Address")
            || t.starts_with("Active")
            || t.starts_with("Netid")
    }) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let mut r = BTreeMap::new();
        // 尝试识别 Windows / macOS netstat 行：Proto / Local / Foreign / State / PID
        // 或 Linux ss 行：State / Recv-Q / Send-Q / Local / Peer / Process
        let proto_candidates = ["TCP", "UDP", "tcp", "udp", "tcp6", "udp6"];
        if proto_candidates.contains(&parts[0]) {
            // Windows / macOS netstat
            r.insert("proto".into(), parts[0].to_string());
            r.insert("local_addr".into(), parts.get(1).unwrap_or(&"").to_string());
            r.insert("foreign_addr".into(), parts.get(2).unwrap_or(&"").to_string());
            // 可选 state / pid
            if parts.len() >= 4 && !parts[3].parse::<i64>().is_ok() {
                r.insert("state".into(), parts[3].to_string());
            }
            if let Some(last) = parts.last() {
                if last.parse::<u32>().is_ok() {
                    r.insert("pid".into(), (*last).to_string());
                }
            }
        } else {
            // Linux ss：LISTEN / ESTAB / ...
            r.insert("state".into(), parts[0].to_string());
            if parts.len() >= 5 {
                r.insert("local_addr".into(), parts[3].to_string());
                r.insert("foreign_addr".into(), parts[4].to_string());
            }
            if let Some(last) = parts.last() {
                if last.starts_with("users:(") {
                    r.insert("program".into(), (*last).to_string());
                }
            }
        }
        rows.push(r);
        if rows.len() >= 80 {
            break;
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_netstat_windows_pid_extracts() {
        let sample = "  TCP    0.0.0.0:3717           0.0.0.0:0              LISTENING       8812\r\n  TCP    127.0.0.1:8307         127.0.0.1:63001        ESTABLISHED     1";
        let rows = parse_netstat_rows(sample);
        assert!(rows.len() >= 2, "rows={:?}", rows);
        assert_eq!(rows[0].get("pid").unwrap(), "8812");
        assert_eq!(rows[0].get("proto").unwrap(), "TCP");
        assert_eq!(rows[0].get("state").unwrap(), "LISTENING");
    }

    #[test]
    fn list_actions_6_covered() {
        let op = NetworkOperator::default();
        let acts = op.list_actions();
        assert_eq!(acts.len(), 6);
        let names: Vec<_> = acts.iter().map(|a| a.name).collect();
        assert!(names.contains(&"ping"));
        assert!(names.contains(&"dns_lookup"));
        assert!(names.contains(&"traffic_usage"));
        assert!(names.contains(&"netstat"));
        assert!(names.contains(&"disable_iface"));
        assert!(names.contains(&"enable_iface"));
        // L3 网卡动作权限校验
        assert_eq!(
            acts.iter().find(|a| a.name == "disable_iface").unwrap().clearance,
            ClearanceLevel::L3
        );
    }
}
