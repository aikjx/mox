// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Syslog Sink — RFC 5424 格式，写入系统日志或 SIEM
//!
//! 适用场景：实时告警、SIEM 对接
//! 支持协议：TCP / UDP / TLS（TLS 占位，可扩展）
//! 刷新策略：Immediate / Batch / Periodic

use crate::error::AuditError;
use crate::event::AuditEvent;
use crate::sink::{AuditSink, FlushPolicy};
use std::io::Write;
use std::net::TcpStream;
use std::sync::RwLock;
use std::time::Duration;

/// Syslog 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyslogProtocol {
    Tcp,
    Udp,
    Tls,
}

/// Syslog Sink
pub struct SyslogSink {
    address: String,
    protocol: SyslogProtocol,
    app_name: String,
    proc_id: String,
    flush_policy: FlushPolicy,
    buffer: RwLock<Vec<String>>,
    last_flush_ms: RwLock<u64>,
    reconnect_interval_sec: u64,
}

impl SyslogSink {
    pub fn new(address: &str, protocol: &str) -> Self {
        let proto = match protocol.to_lowercase().as_str() {
            "tcp" => SyslogProtocol::Tcp,
            "udp" => SyslogProtocol::Udp,
            "tls" => SyslogProtocol::Tls,
            _ => SyslogProtocol::Tcp,
        };
        Self {
            address: address.into(),
            protocol: proto,
            app_name: "mox-audit".into(),
            proc_id: std::process::id().to_string(),
            flush_policy: FlushPolicy::default(),
            buffer: RwLock::new(Vec::new()),
            last_flush_ms: RwLock::new(now_ms()),
            reconnect_interval_sec: 5,
        }
    }

    pub fn with_app_name(mut self, name: &str) -> Self {
        self.app_name = name.into();
        self
    }

    pub fn with_flush_policy(mut self, policy: FlushPolicy) -> Self {
        self.flush_policy = policy;
        self
    }

    pub fn with_reconnect_interval(mut self, secs: u64) -> Self {
        self.reconnect_interval_sec = secs;
        self
    }

    /// 序列化为 RFC 5424 格式
    fn serialize(&self, event: &AuditEvent) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let severity: u8 = event.severity.into();
        let pri = severity as usize;
        write!(s, "<{pri}>1 ").unwrap();
        write!(s, "{} ", event.timestamp.to_rfc3339()).unwrap();
        // HOSTNAME：取环境变量 HOSTNAME 或 fallback
        if let Ok(h) = std::env::var("HOSTNAME") {
            write!(s, "{} ", h).unwrap();
        } else {
            write!(s, "- ").unwrap();
        }
        write!(s, "{} ", self.app_name).unwrap();
        write!(s, "[{}] ", self.proc_id).unwrap();
        write!(s, "{} ", event.action).unwrap();
        write!(s, "[audit@32473").unwrap();
        write!(s, " event_id=\"{}\"", event.event_id).unwrap();
        write!(s, " tenant=\"{}\"", event.tenant_id).unwrap();
        write!(s, " actor=\"{}\"", event.actor.id).unwrap();
        write!(s, " role=\"{}\"", event.actor.role).unwrap();
        write!(s, " source=\"{:?}\"", event.actor.source).unwrap();
        if let Some(ref session) = event.session_id {
            write!(s, " session=\"{session}\"").unwrap();
        }
        if let Some(ref ip) = event.client_ip {
            write!(s, " client_ip=\"{ip}\"").unwrap();
        }
        if let Some(trace_id) = &event.trace_id {
            write!(s, " trace_id=\"{trace_id}\"").unwrap();
        }
        if let Some(ref phase) = event.phase {
            write!(s, " phase=\"{phase}\"").unwrap();
        }
        write!(s, " outcome=\"{:?}\"", event.outcome).unwrap();
        write!(s, " resource_type=\"{}\"", event.resource.resource_type).unwrap();
        write!(s, " resource_id=\"{}\"", event.resource.resource_id).unwrap();
        if !event.extra.is_empty() {
            if let Ok(json) = serde_json::to_string(&event.extra) {
                write!(s, " extra={json}").unwrap();
            }
        }
        write!(s, " content_hash=\"{}\"", event.content_hash).unwrap();
        write!(s, " prev_hash=\"{}\"", event.prev_hash).unwrap();
        if let Some(ref sig) = event.signature {
            write!(s, " signature=\"{sig}\"").unwrap();
        }
        write!(s, "] ").unwrap();
        write!(
            s,
            "action={} resource={}:{} outcome={:?}",
            event.action, event.resource.resource_type, event.resource.resource_id, event.outcome
        )
        .unwrap();
        s
    }

    fn connect_tcp(&self) -> std::io::Result<TcpStream> {
        let addr: std::net::SocketAddr = self.address.parse::<std::net::SocketAddr>().map_err(
            |e: std::net::AddrParseError| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
            },
        )?;
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        Ok(stream)
    }

    fn write_frame(&self, frame: &str) -> Result<(), AuditError> {
        match self.protocol {
            SyslogProtocol::Tcp => {
                let mut retry = 0;
                loop {
                    match self.connect_tcp() {
                        Ok(mut stream) => {
                            stream
                                .write_all(frame.as_bytes())
                                .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
                            stream.write_all(b"\n").ok();
                            return Ok(());
                        }
                        Err(_e) if retry < 3 => {
                            retry += 1;
                            std::thread::sleep(Duration::from_secs(
                                self.reconnect_interval_sec * 2u64.pow(retry as u32),
                            ));
                        }
                        Err(e) => {
                            return Err(AuditError::Connection(format!(
                                "TCP failed after 3 retries: {e}"
                            )))
                        }
                    }
                }
            }
            SyslogProtocol::Udp => {
                let addr: std::net::SocketAddr = self
                    .address
                    .parse::<std::net::SocketAddr>()
                    .map_err(|e| AuditError::Connection(e.to_string()))?;
                let socket = std::net::UdpSocket::bind("0.0.0.0:0")
                    .map_err(|e| AuditError::Connection(e.to_string()))?;
                socket
                    .send_to(frame.as_bytes(), addr)
                    .map_err(|e| AuditError::WriteFailed(e.to_string()))?;
                Ok(())
            }
            SyslogProtocol::Tls => Err(AuditError::Connection("TLS not yet implemented".into())),
        }
    }
}

impl AuditSink for SyslogSink {
    fn append_sync(&self, event: &AuditEvent) -> Result<(), AuditError> {
        let frame = self.serialize(event);
        match self.flush_policy {
            FlushPolicy::Immediate => self.write_frame(&frame),
            FlushPolicy::Batch { max_events } => {
                // 注意：必须先在独立作用域内释放写锁，再获取读锁，
                // 否则同一线程先持写锁再取读锁会触发 RwLock 死锁/panic。
                let should_flush = {
                    let mut buf = self
                        .buffer
                        .write()
                        .expect("审计缓冲写锁已 poison，无法继续追加");
                    buf.push(frame);
                    buf.len() >= max_events
                };
                if should_flush {
                    let buf: Vec<String> = self
                        .buffer
                        .read()
                        .expect("审计缓冲读锁已 poison，无法继续追加")
                        .clone();
                    for f in &buf {
                        self.write_frame(f)?;
                    }
                    self.buffer
                        .write()
                        .expect("审计缓冲写锁已 poison，无法清空")
                        .clear();
                    *self.last_flush_ms.write().expect("刷新时间戳写锁已 poison") = now_ms();
                }
                Ok(())
            }
            FlushPolicy::Periodic { interval_ms } => {
                let should_flush = {
                    let mut buf = self
                        .buffer
                        .write()
                        .expect("审计缓冲写锁已 poison，无法继续追加");
                    buf.push(frame);
                    now_ms() - *self.last_flush_ms.read().expect("刷新时间戳读锁已 poison")
                        >= interval_ms
                };
                if should_flush {
                    let buf: Vec<String> = self
                        .buffer
                        .read()
                        .expect("审计缓冲读锁已 poison，无法继续追加")
                        .clone();
                    for f in &buf {
                        self.write_frame(f)?;
                    }
                    self.buffer
                        .write()
                        .expect("审计缓冲写锁已 poison，无法清空")
                        .clear();
                    *self.last_flush_ms.write().expect("刷新时间戳写锁已 poison") = now_ms();
                }
                Ok(())
            }
        }
    }

    fn flush(&self) -> Result<(), AuditError> {
        let buf: Vec<String> = self
            .buffer
            .read()
            .expect("审计缓冲读锁已 poison，无法刷新")
            .clone();
        for f in &buf {
            self.write_frame(f)?;
        }
        self.buffer
            .write()
            .expect("审计缓冲写锁已 poison，无法清空")
            .clear();
        *self.last_flush_ms.write().expect("刷新时间戳写锁已 poison") = now_ms();
        Ok(())
    }

    fn health_check(&self) -> Result<(), AuditError> {
        if matches!(self.protocol, SyslogProtocol::Tcp) {
            self.connect_tcp()
                .map_err(|e| AuditError::Connection(e.to_string()))?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "syslog_sink"
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::test_event;

    #[test]
    fn serialize_rfc5424() {
        let sink = SyslogSink::new("localhost:514", "tcp").with_app_name("test-app");
        let frame = sink.serialize(&test_event());
        // INFO severity = 6, facility local0 = 16, pri = 16*8+6 = 134
        assert!(frame.starts_with("<134>1 "), "pri 应为 134，实际: {}", &frame[..10]);
        assert!(frame.contains("test-app"));
        assert!(frame.contains("flow.created"));
        assert!(frame.contains("test-tenant"));
        assert!(frame.contains("event_id=\""));
        assert!(frame.contains("prev_hash=\""));
        assert!(frame.contains("content_hash=\""));
    }

    #[test]
    fn serialize_with_trace_and_phase() {
        use uuid::Uuid;
        let sink = SyslogSink::new("localhost:514", "tcp");
        let ev = test_event()
            .with_trace_id(Uuid::new_v4())
            .with_phase("analyze");
        let frame = sink.serialize(&ev);
        assert!(frame.contains("trace_id=\""));
        assert!(frame.contains("phase=\"analyze\""));
    }

    #[test]
    fn serialize_with_session_and_ip() {
        let sink = SyslogSink::new("localhost:514", "tcp");
        let ev = test_event()
            .with_session("sess-1".into())
            .with_client_ip("10.0.0.1".into());
        let frame = sink.serialize(&ev);
        assert!(frame.contains("session=\"sess-1\""));
        assert!(frame.contains("client_ip=\"10.0.0.1\""));
    }

    #[test]
    fn protocol_parsing() {
        let s = SyslogSink::new("h:514", "tcp");
        assert_eq!(s.protocol, SyslogProtocol::Tcp);
        let s = SyslogSink::new("h:514", "udp");
        assert_eq!(s.protocol, SyslogProtocol::Udp);
        let s = SyslogSink::new("h:514", "tls");
        assert_eq!(s.protocol, SyslogProtocol::Tls);
        let s = SyslogSink::new("h:514", "unknown");
        assert_eq!(s.protocol, SyslogProtocol::Tcp); // default
    }

    #[test]
    fn batch_policy_no_deadlock_under_concurrency() {
        use std::sync::Arc;
        use std::thread;
        // 修复回归：append_sync 中先持写锁再取读锁曾在同线程死锁。
        // 用 Batch 策略在多线程下高频追加，验证不再死锁/panic。
        let sink = Arc::new(
            SyslogSink::new("localhost:514", "udp")
                .with_flush_policy(FlushPolicy::Batch { max_events: 4 }),
        );
        let mut handles = Vec::new();
        for _ in 0..4 {
            let s = Arc::clone(&sink);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    let _ = s.append_sync(&test_event());
                }
            }));
        }
        for h in handles {
            h.join().expect("审计线程应正常结束（无死锁）");
        }
        // 最终 flush 不应死锁
        sink.flush().ok();
    }

    #[test]
    fn sink_name() {
        let s = SyslogSink::new("localhost:514", "tcp");
        assert_eq!(s.name(), "syslog_sink");
    }

    #[test]
    fn severity_priority_mapping() {
        use crate::event::AuditSeverity;
        // 验证 severity → priority 转换
        let pri_info: u8 = AuditSeverity::Info.into();
        assert_eq!(pri_info, 16 * 8 + 6); // local0 + info

        let pri_critical: u8 = AuditSeverity::Critical.into();
        assert_eq!(pri_critical, 16 * 8 + 2); // local0 + critical
    }
}
